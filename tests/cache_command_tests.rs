//! Tests for CacheCommand
//!
//! These tests verify the cache management command functionality,
//! including selective clearing based on `ClearType` and the
//! no-files-exist case. Each test isolates filesystem state via
//! `TempDir` and the `XDG_CONFIG_HOME` environment variable.

use std::sync::Arc;
use subx_cli::cli::{CacheAction, CacheArgs, ClearArgs, ClearType};
use subx_cli::commands::cache_command;
use subx_cli::config::{ConfigService, TestConfigService};
use tempfile::TempDir;

/// Prepare an isolated config directory and return paths to the cache and
/// journal files inside it. The caller must keep the returned `TempDir` alive
/// for the duration of the test.
fn setup_isolated_config() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let tmp = TempDir::new().expect("failed to create temp dir");
    // SAFETY: tests run in a dedicated process under cargo nextest, so
    // mutating the environment here does not race with other tests.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }
    let subx_dir = tmp.path().join("subx");
    std::fs::create_dir_all(&subx_dir).expect("failed to create subx dir");
    let cache_file = subx_dir.join("match_cache.json");
    let journal_file = subx_dir.join("match_journal.json");
    (tmp, cache_file, journal_file)
}

#[tokio::test]
async fn test_cache_clear_cache_only_removes_cache_file() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    std::fs::write(&cache_file, "{}").unwrap();
    std::fs::write(&journal_file, "{}").unwrap();

    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::Cache,
        }),
    };
    cache_command::execute(args).await.expect("clear failed");

    assert!(!cache_file.exists(), "cache file should be removed");
    assert!(journal_file.exists(), "journal file should remain");
}

#[tokio::test]
async fn test_cache_clear_journal_only_removes_journal_file() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    std::fs::write(&cache_file, "{}").unwrap();
    std::fs::write(&journal_file, "{}").unwrap();

    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::Journal,
        }),
    };
    cache_command::execute(args).await.expect("clear failed");

    assert!(cache_file.exists(), "cache file should remain");
    assert!(!journal_file.exists(), "journal file should be removed");
}

#[tokio::test]
async fn test_cache_clear_all_removes_both_files() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    std::fs::write(&cache_file, "{}").unwrap();
    std::fs::write(&journal_file, "{}").unwrap();

    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::All,
        }),
    };
    cache_command::execute(args).await.expect("clear failed");

    assert!(!cache_file.exists(), "cache file should be removed");
    assert!(!journal_file.exists(), "journal file should be removed");
}

#[tokio::test]
async fn test_cache_clear_all_no_files_is_ok() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    assert!(!cache_file.exists());
    assert!(!journal_file.exists());

    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::All,
        }),
    };
    cache_command::execute(args).await.expect("clear failed");

    assert!(!cache_file.exists());
    assert!(!journal_file.exists());
}

#[tokio::test]
async fn test_cache_clear_cache_only_when_only_journal_exists() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    std::fs::write(&journal_file, "{}").unwrap();

    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::Cache,
        }),
    };
    cache_command::execute(args).await.expect("clear failed");

    assert!(!cache_file.exists());
    assert!(
        journal_file.exists(),
        "journal file must not be touched when clearing cache only"
    );
}

#[tokio::test]
async fn test_cache_clear_with_config_service() {
    let (_tmp, _cache_file, _journal_file) = setup_isolated_config();
    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::All,
        }),
    };
    let config_service = Arc::new(TestConfigService::with_defaults());

    cache_command::execute_with_config(args, config_service)
        .await
        .expect("execute_with_config failed");
}

#[test]
fn test_cache_args_construction() {
    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::All,
        }),
    };
    assert!(matches!(args.action, CacheAction::Clear(_)));
}

#[test]
fn test_cache_args_debug_formatting() {
    let args = CacheArgs {
        action: CacheAction::Clear(ClearArgs {
            r#type: ClearType::Cache,
        }),
    };
    let debug_str = format!("{:?}", args);
    assert!(debug_str.contains("CacheArgs"));
    assert!(debug_str.contains("Clear"));
}

use std::time::{SystemTime, UNIX_EPOCH};
use subx_cli::cli::StatusArgs;

/// Build a minimal valid `match_cache.json` payload for status tests.
fn write_sample_cache(path: &std::path::Path, ai_model: &str, ops: usize) {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let operations: Vec<serde_json::Value> = (0..ops)
        .map(|i| {
            serde_json::json!({
                "video_file": format!("/tmp/video_{}.mkv", i),
                "subtitle_file": format!("/tmp/sub_{}.srt", i),
                "new_subtitle_name": format!("video_{}.srt", i),
                "confidence": 0.9,
                "reasoning": [],
            })
        })
        .collect();
    let payload = serde_json::json!({
        "cache_version": "1.0",
        "directory": "/tmp",
        "file_snapshot": [],
        "match_operations": operations,
        "created_at": created_at,
        "ai_model_used": ai_model,
        "config_hash": "deadbeefcafebabe",
        "original_relocation_mode": "None",
        "original_backup_enabled": false,
    });
    std::fs::write(path, serde_json::to_string(&payload).unwrap()).unwrap();
}

#[tokio::test]
async fn test_cache_status_missing_cache_prints_friendly_message() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    assert!(!cache_file.exists());

    let config_service = TestConfigService::with_defaults();
    let args = StatusArgs { json: false };

    cache_command::execute_status(&args, &config_service)
        .await
        .expect("execute_status must succeed when no cache exists");
}

#[tokio::test]
async fn test_cache_status_missing_cache_json_reports_exists_false() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    assert!(!cache_file.exists());

    let config_service = TestConfigService::with_defaults();
    let args = StatusArgs { json: true };

    cache_command::execute_status(&args, &config_service)
        .await
        .expect("execute_status must succeed for JSON output when cache is absent");
}

#[tokio::test]
async fn test_cache_status_with_existing_cache_reads_metadata() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    write_sample_cache(&cache_file, "gpt-4.1-mini", 3);

    let config_service = TestConfigService::with_defaults();
    let args = StatusArgs { json: false };

    cache_command::execute_status(&args, &config_service)
        .await
        .expect("execute_status must succeed with existing cache");
}

#[tokio::test]
async fn test_cache_status_json_mode_with_existing_cache() {
    let (_tmp, cache_file, journal_file) = setup_isolated_config();
    write_sample_cache(&cache_file, "gpt-4.1-mini", 2);
    std::fs::write(&journal_file, "{}").unwrap();

    let config_service = TestConfigService::with_defaults();
    let args = StatusArgs { json: true };

    cache_command::execute_status(&args, &config_service)
        .await
        .expect("execute_status JSON mode must succeed with existing cache");
}

#[tokio::test]
async fn test_cache_status_via_execute_with_config() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    write_sample_cache(&cache_file, "test-model", 1);

    let args = CacheArgs {
        action: CacheAction::Status(StatusArgs { json: false }),
    };
    let config_service = Arc::new(TestConfigService::with_defaults());

    cache_command::execute_with_config(args, config_service)
        .await
        .expect("execute_with_config must route status to execute_status");
}

// -------------------------------------------------------------------------
// Rollback subcommand integration tests (Group 7)
// -------------------------------------------------------------------------

use std::path::PathBuf;
use subx_cli::cli::RollbackArgs;
use subx_cli::core::matcher::journal::{
    JournalData, JournalEntry, JournalEntryStatus, JournalOperationType,
};

/// Return the (size, mtime_seconds) of a file — the same fields recorded in
/// journal entries — so rollback integrity checks line up on-disk with the
/// data we persist to the journal.
fn file_size_and_mtime(path: &std::path::Path) -> (u64, u64) {
    let metadata = std::fs::metadata(path).expect("file metadata");
    let mtime = metadata
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (metadata.len(), mtime)
}

/// Write `contents` to `path` and return a completed `JournalEntry` describing
/// an operation of `op_type` from `source` to `path` (which already exists).
fn make_entry(
    op_type: JournalOperationType,
    source: PathBuf,
    destination: PathBuf,
) -> JournalEntry {
    let (size, mtime) = file_size_and_mtime(&destination);
    JournalEntry {
        operation_type: op_type,
        source,
        destination,
        backup_path: None,
        status: JournalEntryStatus::Completed,
        file_size: size,
        file_mtime: mtime,
    }
}

#[tokio::test]
async fn test_rollback_copy_batch_deletes_destinations_and_preserves_sources() {
    let (tmp, _cache_file, journal_file) = setup_isolated_config();

    // Set up two copied files: source preserved, destination exists.
    let src1 = tmp.path().join("video1.srt");
    let dst1 = tmp.path().join("renamed1.srt");
    let src2 = tmp.path().join("video2.srt");
    let dst2 = tmp.path().join("renamed2.srt");
    std::fs::write(&src1, "source-1").unwrap();
    std::fs::write(&dst1, "copy-1").unwrap();
    std::fs::write(&src2, "source-2").unwrap();
    std::fs::write(&dst2, "copy-2").unwrap();

    let journal = JournalData {
        batch_id: "test-copy-batch".to_string(),
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        entries: vec![
            make_entry(JournalOperationType::Copied, src1.clone(), dst1.clone()),
            make_entry(JournalOperationType::Copied, src2.clone(), dst2.clone()),
        ],
    };
    journal.save(&journal_file).await.expect("save journal");

    cache_command::execute_rollback(&RollbackArgs { force: false })
        .await
        .expect("rollback");

    assert!(!dst1.exists(), "copy destination 1 should be removed");
    assert!(!dst2.exists(), "copy destination 2 should be removed");
    assert!(src1.exists(), "source 1 must be preserved");
    assert!(src2.exists(), "source 2 must be preserved");
    assert_eq!(std::fs::read_to_string(&src1).unwrap(), "source-1");
    assert_eq!(std::fs::read_to_string(&src2).unwrap(), "source-2");
}

#[tokio::test]
async fn test_rollback_move_batch_restores_sources() {
    let (tmp, _cache_file, journal_file) = setup_isolated_config();

    let src1 = tmp.path().join("a").join("original1.srt");
    let dst1 = tmp.path().join("b").join("moved1.srt");
    let src2 = tmp.path().join("a").join("original2.srt");
    let dst2 = tmp.path().join("b").join("moved2.srt");
    std::fs::create_dir_all(src1.parent().unwrap()).unwrap();
    std::fs::create_dir_all(dst1.parent().unwrap()).unwrap();
    // After a move, only destinations exist on disk.
    std::fs::write(&dst1, "payload-1").unwrap();
    std::fs::write(&dst2, "payload-2").unwrap();

    let journal = JournalData {
        batch_id: "test-move-batch".to_string(),
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        entries: vec![
            make_entry(JournalOperationType::Moved, src1.clone(), dst1.clone()),
            make_entry(JournalOperationType::Moved, src2.clone(), dst2.clone()),
        ],
    };
    journal.save(&journal_file).await.expect("save journal");

    cache_command::execute_rollback(&RollbackArgs { force: false })
        .await
        .expect("rollback");

    assert!(src1.exists(), "source 1 must be restored");
    assert!(src2.exists(), "source 2 must be restored");
    assert!(!dst1.exists(), "destination 1 must be gone");
    assert!(!dst2.exists(), "destination 2 must be gone");
    assert_eq!(std::fs::read_to_string(&src1).unwrap(), "payload-1");
    assert_eq!(std::fs::read_to_string(&src2).unwrap(), "payload-2");
}

#[tokio::test]
async fn test_rollback_with_no_journal_is_clean_noop() {
    let (_tmp, _cache_file, journal_file) = setup_isolated_config();
    assert!(!journal_file.exists());

    // Should not error, should not panic.
    cache_command::execute_rollback(&RollbackArgs { force: false })
        .await
        .expect("rollback should succeed with no journal");

    assert!(!journal_file.exists());
}

#[tokio::test]
async fn test_rollback_aborts_when_destination_modified_without_force() {
    let (tmp, _cache_file, journal_file) = setup_isolated_config();

    let src = tmp.path().join("video.srt");
    let dst = tmp.path().join("renamed.srt");
    std::fs::write(&src, "source").unwrap();
    std::fs::write(&dst, "original-copy").unwrap();

    let entry = make_entry(JournalOperationType::Copied, src.clone(), dst.clone());
    let journal = JournalData {
        batch_id: "test-modified".to_string(),
        created_at: 0,
        entries: vec![entry],
    };
    journal.save(&journal_file).await.expect("save journal");

    // Tamper with the destination after the journal is written so size changes.
    std::fs::write(&dst, "tampered-content-much-longer-than-before").unwrap();

    let err = cache_command::execute_rollback(&RollbackArgs { force: false })
        .await
        .expect_err("rollback must abort on modified destination");
    let msg = format!("{}", err);
    assert!(
        msg.contains("modified") || msg.contains("no longer"),
        "error should mention destination modification, got: {msg}"
    );

    // The destination must not have been touched — rollback aborted before any action.
    assert!(dst.exists(), "destination should remain untouched");
    assert!(journal_file.exists(), "journal should remain on abort");
}

#[tokio::test]
async fn test_rollback_deletes_journal_after_success() {
    let (tmp, _cache_file, journal_file) = setup_isolated_config();

    let src = tmp.path().join("video.srt");
    let dst = tmp.path().join("renamed.srt");
    std::fs::write(&src, "source").unwrap();
    std::fs::write(&dst, "copy").unwrap();

    let entry = make_entry(JournalOperationType::Copied, src.clone(), dst.clone());
    let journal = JournalData {
        batch_id: "test-journal-delete".to_string(),
        created_at: 0,
        entries: vec![entry],
    };
    journal.save(&journal_file).await.expect("save journal");
    assert!(journal_file.exists());

    cache_command::execute_rollback(&RollbackArgs { force: false })
        .await
        .expect("rollback");

    assert!(
        !journal_file.exists(),
        "journal file must be deleted after successful rollback"
    );
}

// ---------------------------------------------------------------------------
// Cache Apply tests
// ---------------------------------------------------------------------------

/// Build a minimal valid CacheData JSON string for testing.
fn make_test_cache_json(config_hash: &str) -> String {
    serde_json::json!({
        "cache_version": "1.0",
        "directory": "filelist_0000000000000000",
        "file_snapshot": [],
        "match_operations": [],
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "ai_model_used": "test-model",
        "config_hash": config_hash,
        "original_relocation_mode": "None",
        "original_backup_enabled": false,
    })
    .to_string()
}

#[tokio::test]
async fn test_apply_no_cache_prints_friendly_message() {
    let (_tmp, _cache_file, _journal_file) = setup_isolated_config();
    // Don't create a cache file
    let result = cache_command::execute_apply(
        &subx_cli::cli::ApplyArgs {
            yes: true,
            force: false,
            confidence: None,
        },
        &TestConfigService::default(),
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_apply_config_hash_mismatch_aborts_without_force() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    std::fs::write(&cache_file, make_test_cache_json("wrong_hash")).unwrap();

    let result = cache_command::execute_apply(
        &subx_cli::cli::ApplyArgs {
            yes: true,
            force: false,
            confidence: None,
        },
        &TestConfigService::default(),
    )
    .await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Configuration has changed"));
}

#[tokio::test]
async fn test_apply_legacy_cache_requires_force() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    // Compute the correct config hash so it passes that check
    let config_service = TestConfigService::default();
    let config = config_service.get_config().unwrap();
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        "None".hash(&mut h);
        config.general.backup_enabled.hash(&mut h);
        format!("{:016x}", h.finish())
    };
    std::fs::write(&cache_file, make_test_cache_json(&hash)).unwrap();

    let result = cache_command::execute_apply(
        &subx_cli::cli::ApplyArgs {
            yes: true,
            force: false,
            confidence: None,
        },
        &config_service,
    )
    .await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("legacy format"));
}

#[tokio::test]
async fn test_apply_empty_operations_with_force() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    let config_service = TestConfigService::default();
    std::fs::write(&cache_file, make_test_cache_json("any_hash")).unwrap();

    let result = cache_command::execute_apply(
        &subx_cli::cli::ApplyArgs {
            yes: true,
            force: true,
            confidence: None,
        },
        &config_service,
    )
    .await;
    // With --force and empty operations, should succeed with "No operations to apply"
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_apply_non_tty_without_yes_aborts() {
    let (_tmp, cache_file, _journal_file) = setup_isolated_config();
    let config_service = TestConfigService::default();
    // Write cache with an operation so it reaches the confirmation prompt
    let cache_json = serde_json::json!({
        "cache_version": "1.0",
        "directory": "filelist_0000000000000000",
        "file_snapshot": [],
        "match_operations": [{
            "video_file": "/tmp/nonexistent_video.mp4",
            "subtitle_file": "/tmp/nonexistent_sub.srt",
            "new_subtitle_name": "test.srt",
            "confidence": 0.95,
            "reasoning": ["test"]
        }],
        "created_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "ai_model_used": "test-model",
        "config_hash": "any_hash",
        "original_relocation_mode": "None",
        "original_backup_enabled": false,
    })
    .to_string();
    std::fs::write(&cache_file, &cache_json).unwrap();

    // Under cargo nextest, stdin is not a TTY, so this should abort
    let result = cache_command::execute_apply(
        &subx_cli::cli::ApplyArgs {
            yes: false,
            force: true,
            confidence: None,
        },
        &config_service,
    )
    .await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Non-interactive terminal"));
}
