//! Integration tests for AI service error handling with Wiremock mock AI server.

use std::{fs, path::Path};
use tempfile::TempDir;

use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;

mod common;
use common::mock_openai_helper::MockOpenAITestHelper;

/// Basic file setup for error handling tests.
fn create_test_files(root: &Path) {
    // Create files in the same directory to ensure they can be matched
    fs::write(root.join("movie.mp4"), "video").unwrap();
    fs::write(
        root.join("movie.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle content\n",
    )
    .unwrap();
}

#[tokio::test]
async fn test_unauthorized_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    create_test_files(root);

    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper.setup_error_response(401, "Unauthorized").await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 50,
        backup: false,
        copy: true,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_err(),
        "Expected unauthorized error, got success: {:?}",
        result
    );
}

#[tokio::test]
async fn test_rate_limit_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    create_test_files(root);

    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .setup_error_response(429, "Too Many Requests")
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 50,
        backup: false,
        copy: true,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_err(),
        "Expected rate limit error, got success: {:?}",
        result
    );
}

#[tokio::test]
async fn test_internal_server_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    create_test_files(root);

    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .setup_error_response(500, "Internal Server Error")
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 50,
        backup: false,
        copy: true,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_err(),
        "Expected internal server error, got success: {:?}",
        result
    );
}
