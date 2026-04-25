//! Integration tests for journal writing performed by
//! [`subx_cli::core::matcher::engine::MatchEngine::execute_operations`].
//!
//! These tests exercise the journal-write path end-to-end by constructing
//! [`MatchOperation`] values directly and driving them through the engine.
//! No AI service is involved so the tests remain fast and deterministic.

use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use subx_cli::core::matcher::discovery::{MediaFile, MediaFileType};
use subx_cli::core::matcher::engine::{
    ConflictResolution, FileRelocationMode, MatchConfig, MatchEngine, MatchOperation,
};
use subx_cli::core::matcher::journal::{
    JournalData, JournalEntryStatus, JournalOperationType, journal_path,
};
use subx_cli::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use tempfile::TempDir;

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct PanicAI;

#[async_trait]
impl AIProvider for PanicAI {
    async fn analyze_content(&self, _req: AnalysisRequest) -> subx_cli::Result<MatchResult> {
        panic!("AI provider must not be invoked during execute_operations tests")
    }

    async fn verify_match(&self, _req: VerificationRequest) -> subx_cli::Result<ConfidenceScore> {
        panic!("AI provider must not be invoked during execute_operations tests")
    }
}

fn make_config() -> MatchConfig {
    MatchConfig {
        confidence_threshold: 0.0,
        max_sample_length: 0,
        enable_content_analysis: false,
        backup_enabled: false,
        relocation_mode: FileRelocationMode::None,
        conflict_resolution: ConflictResolution::AutoRename,
        ai_model: "test-model".to_string(),
        max_subtitle_bytes: 52_428_800,
    }
}

fn make_media_file(path: PathBuf, file_type: MediaFileType) -> MediaFile {
    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    MediaFile {
        id: name.clone(),
        path,
        file_type,
        size,
        name: name.clone(),
        extension,
        relative_path: name,
    }
}

#[tokio::test]
async fn execute_operations_writes_journal_for_rename() {
    let _guard = TEST_MUTEX.lock().await;

    let tmp = TempDir::new().unwrap();
    // SAFETY: Test serialisation via TEST_MUTEX ensures no concurrent env access.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }

    let workdir = tmp.path().join("work");
    fs::create_dir_all(&workdir).unwrap();

    let video_path = workdir.join("movie.mp4");
    let subtitle_path = workdir.join("original.srt");
    fs::write(&video_path, b"video-bytes").unwrap();
    fs::write(&subtitle_path, b"1\n00:00:00,000 --> 00:00:01,000\nHello\n").unwrap();

    let operation = MatchOperation {
        video_file: make_media_file(video_path.clone(), MediaFileType::Video),
        subtitle_file: make_media_file(subtitle_path.clone(), MediaFileType::Subtitle),
        new_subtitle_name: "movie.srt".to_string(),
        confidence: 0.95,
        reasoning: vec!["same-directory".to_string()],
        relocation_mode: FileRelocationMode::None,
        relocation_target_path: None,
        requires_relocation: false,
    };

    let engine = MatchEngine::new(Box::new(PanicAI), make_config());
    engine
        .execute_operations(std::slice::from_ref(&operation), false)
        .await
        .expect("execute_operations should succeed");

    // The subtitle should now be renamed on disk.
    let expected_destination = workdir.join("movie.srt");
    assert!(
        expected_destination.exists(),
        "renamed subtitle should exist on disk"
    );
    assert!(
        !subtitle_path.exists(),
        "original subtitle path should no longer exist after rename"
    );

    // Journal should be written at journal_path().
    let journal_file = journal_path().expect("journal_path");
    assert!(
        journal_file.exists(),
        "journal file should be created after execute_operations"
    );

    let journal = JournalData::load(&journal_file)
        .await
        .expect("load journal");
    assert!(!journal.batch_id.is_empty());
    assert!(journal.created_at > 0);
    assert_eq!(journal.entries.len(), 1);

    let entry = &journal.entries[0];
    assert_eq!(entry.operation_type, JournalOperationType::Renamed);
    assert_eq!(entry.status, JournalEntryStatus::Completed);
    assert_eq!(entry.source, subtitle_path);
    assert_eq!(entry.destination, expected_destination);
    assert!(entry.backup_path.is_none());
    assert!(entry.file_size > 0);
}

#[tokio::test]
async fn execute_operations_skips_journal_in_dry_run() {
    let _guard = TEST_MUTEX.lock().await;

    let tmp = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }

    let workdir = tmp.path().join("work");
    fs::create_dir_all(&workdir).unwrap();

    let video_path = workdir.join("movie.mp4");
    let subtitle_path = workdir.join("original.srt");
    fs::write(&video_path, b"video-bytes").unwrap();
    fs::write(&subtitle_path, b"sub").unwrap();

    let operation = MatchOperation {
        video_file: make_media_file(video_path.clone(), MediaFileType::Video),
        subtitle_file: make_media_file(subtitle_path.clone(), MediaFileType::Subtitle),
        new_subtitle_name: "movie.srt".to_string(),
        confidence: 0.95,
        reasoning: vec![],
        relocation_mode: FileRelocationMode::None,
        relocation_target_path: None,
        requires_relocation: false,
    };

    let engine = MatchEngine::new(Box::new(PanicAI), make_config());
    engine
        .execute_operations(std::slice::from_ref(&operation), true)
        .await
        .expect("dry-run should succeed");

    let journal_file = journal_path().expect("journal_path");
    assert!(
        !journal_file.exists(),
        "dry-run must not write a journal file"
    );
    assert!(
        subtitle_path.exists(),
        "dry-run must not mutate the filesystem"
    );
}

#[tokio::test]
async fn execute_operations_records_copy_with_backup_disabled() {
    let _guard = TEST_MUTEX.lock().await;

    let tmp = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }

    let video_dir = tmp.path().join("videos");
    let subs_dir = tmp.path().join("subs");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subs_dir).unwrap();

    let video_path = video_dir.join("movie.mp4");
    let subtitle_path = subs_dir.join("original.srt");
    fs::write(&video_path, b"video-bytes").unwrap();
    fs::write(&subtitle_path, b"1\n00:00:00,000 --> 00:00:01,000\nHi\n").unwrap();

    let target = video_dir.join("movie.srt");
    let operation = MatchOperation {
        video_file: make_media_file(video_path.clone(), MediaFileType::Video),
        subtitle_file: make_media_file(subtitle_path.clone(), MediaFileType::Subtitle),
        new_subtitle_name: "movie.srt".to_string(),
        confidence: 0.9,
        reasoning: vec![],
        relocation_mode: FileRelocationMode::Copy,
        relocation_target_path: Some(target.clone()),
        requires_relocation: true,
    };

    let mut config = make_config();
    config.relocation_mode = FileRelocationMode::Copy;
    let engine = MatchEngine::new(Box::new(PanicAI), config);
    engine
        .execute_operations(std::slice::from_ref(&operation), false)
        .await
        .expect("execute_operations should succeed");

    assert!(target.exists(), "copy should create target file");
    assert!(
        subtitle_path.exists(),
        "copy mode should preserve the original subtitle"
    );

    let journal = JournalData::load(&journal_path().unwrap())
        .await
        .expect("journal");
    assert_eq!(journal.entries.len(), 1);
    let entry = &journal.entries[0];
    assert_eq!(entry.operation_type, JournalOperationType::Copied);
    assert_eq!(entry.source, subtitle_path);
    assert_eq!(entry.destination, target);
    assert!(entry.backup_path.is_none());
    assert_eq!(entry.status, JournalEntryStatus::Completed);
}
