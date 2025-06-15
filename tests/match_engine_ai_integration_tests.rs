//! Advanced integration tests for match command with Wiremock mock AI server.

use serde_json::json;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use tempfile::TempDir;

use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;

mod common;
use common::{
    mock_openai_helper::MockOpenAITestHelper, test_data_generators::MatchResponseGenerator,
};

#[tokio::test]
async fn test_parallel_match_operations_with_mock() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create multiple test video and subtitle files
    create_multiple_test_files(root, 5);

    // Scan files to obtain actual file IDs
    let discovery = subx_cli::core::matcher::FileDiscovery::new();
    let files = discovery.scan_directory(root, true).unwrap();
    let video_files: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type, subx_cli::core::matcher::MediaFileType::Video))
        .collect();
    let subtitle_files: Vec<_> = files
        .iter()
        .filter(|f| {
            matches!(
                f.file_type,
                subx_cli::core::matcher::MediaFileType::Subtitle
            )
        })
        .collect();

    // Create multiple dynamic match responses
    let matches_json = json!({
        "matches": [
            {
                "video_file_id": video_files[0].id,
                "subtitle_file_id": subtitle_files[0].id,
                "confidence": 0.92,
                "match_factors": ["filename_similarity"]
            },
            {
                "video_file_id": video_files[1].id,
                "subtitle_file_id": subtitle_files[1].id,
                "confidence": 0.87,
                "match_factors": ["content_correlation", "language_match"]
            },
            {
                "video_file_id": video_files[2].id,
                "subtitle_file_id": subtitle_files[2].id,
                "confidence": 0.85,
                "match_factors": ["filename_similarity"]
            },
            {
                "video_file_id": video_files[3].id,
                "subtitle_file_id": subtitle_files[3].id,
                "confidence": 0.83,
                "match_factors": ["content_correlation"]
            },
            {
                "video_file_id": video_files[4].id,
                "subtitle_file_id": subtitle_files[4].id,
                "confidence": 0.81,
                "match_factors": ["filename_similarity", "language_match"]
            }
        ],
        "confidence": 0.85,
        "reasoning": "Multiple high-confidence matches identified."
    });

    // Mock AI service to return multiple matches
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&matches_json.to_string())
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .with_parallel_settings(4, 100)
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 50,
        backup: true,
        copy: true,
        move_files: false,
    };

    let start = Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start.elapsed();
    assert!(
        result.is_ok(),
        "Parallel match operation failed: {:?}",
        result
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "Parallel execution too slow: {:?}",
        elapsed
    );

    verify_parallel_processing_results(root, 5);
}

#[tokio::test]
async fn test_confidence_threshold_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a single test file
    create_test_files(root);

    // Mock AI service to return low-confidence response
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::no_matches_found())
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: true,
        copy: true,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok(), "Execution failed: {:?}", result);

    // Verify that no files were copied due to low confidence
    let video_dir = root.join("videos");
    let copied = fs::read_dir(&video_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.path().extension().is_some_and(|ext| ext == "srt"));
    assert!(!copied, "Low confidence matches should be rejected");
}

fn create_multiple_test_files(root: &Path, count: usize) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    for i in 1..=count {
        fs::write(
            video_dir.join(format!("movie{}.mp4", i)),
            format!("video{}", i),
        )
        .unwrap();
        fs::write(
            subtitle_dir.join(format!("movie{}.srt", i)),
            format!("1\n00:00:01,000 --> 00:00:02,000\nSubtitle {}\n", i),
        )
        .unwrap();
    }
}

fn create_test_files(root: &Path) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();
    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "subtitle").unwrap();
}

fn verify_parallel_processing_results(root: &Path, count: usize) {
    let video_dir = root.join("videos");
    let processed = fs::read_dir(&video_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "srt"))
        .count();
    assert_eq!(
        processed, count,
        "Expected {} subtitle files copied, found {}",
        count, processed
    );
}
