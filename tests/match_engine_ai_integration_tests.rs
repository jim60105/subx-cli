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
use common::{MatchResponseGenerator, MockOpenAITestHelper};

#[tokio::test]
async fn test_parallel_match_operations_with_mock() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create multiple test video and subtitle files
    create_multiple_test_files(&root, 5);

    // Mock AI service to return multiple matches
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::multiple_matches())
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .with_parallel_settings(4, 100)
        .build_service();

    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 50,
        recursive: true,
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

    verify_parallel_processing_results(&root, 5);
}

#[tokio::test]
async fn test_confidence_threshold_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a single test file
    create_test_files(&root);

    // Mock AI service to return low-confidence response
    let low_confidence_response = json!({
        "choices": [
            { "message": { "content": MatchResponseGenerator::no_matches_found() }, "finish_reason": "stop" }
        ],
        "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
        "model": "gpt-4o-mini"
    })
    .to_string();

    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&low_confidence_response)
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 80,
        recursive: true,
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
        .any(|e| e.path().extension().map_or(false, |ext| ext == "srt"));
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
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "srt"))
        .count();
    assert_eq!(
        processed, count,
        "Expected {} subtitle files copied, found {}",
        count, processed
    );
}
