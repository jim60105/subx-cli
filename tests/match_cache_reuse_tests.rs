//! Integration tests for match command cache reuse with copy/move modes.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// Using async mutex to avoid environment variable race conditions while avoiding clippy::await_holding_lock warning
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    // Use async mutex to avoid environment variable race conditions while avoiding await while holding lock
    let _guard = TEST_MUTEX.lock().await;

    // Use a fixed test root directory to ensure consistent cache path
    let test_root = std::path::Path::new("/tmp/subx_cache_test");
    if test_root.exists() {
        fs::remove_dir_all(test_root).unwrap();
    }
    fs::create_dir_all(test_root).unwrap();

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("video_copy.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("subtitle_copy.srt"), "sub").unwrap();

    // Scan files to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // Create mock AI service using actual file IDs, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // First execution dry-run to create cache
    let args_preview = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    // Second execution of the same dry-run operation, should use cache (same directory)
    let args_second = MatchArgs {
        path: test_root.to_path_buf(), // Use the same directory
        dry_run: true,                 // Keep the same mode
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();

    // Verify mock server received only one request
    mock_helper.verify_expectations().await;
}

#[tokio::test]
async fn test_cache_reuse_preserves_move_mode() {
    // Use async mutex to avoid environment variable race conditions while avoiding await while holding lock
    let _guard = TEST_MUTEX.lock().await;

    // Use a fixed test root directory to ensure consistent cache path
    let test_root = std::path::Path::new("/tmp/subx_cache_test_move");
    if test_root.exists() {
        fs::remove_dir_all(test_root).unwrap();
    }
    fs::create_dir_all(test_root).unwrap();

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("video_move.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("subtitle_move.srt"), "sub").unwrap();

    // Scan files to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // Create mock AI service using actual file IDs, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // First execution dry-run to create cache
    let args_preview = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    // Second execution of the same dry-run operation, should use cache
    let args_second = MatchArgs {
        path: test_root.to_path_buf(),
        dry_run: true, // Keep the same mode
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();

    // Verify mock server received only one request
    mock_helper.verify_expectations().await;
}
