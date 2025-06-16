//! Integration tests for match command cache reuse with copy/move modes.

use log::debug;
use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// Using async mutex to avoid environment variable race conditions while avoiding clippy::await_holding_lock warning
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions while avoiding await while holding lock
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_cache_reuse_preserves_copy_mode");

    // Use TempDir for consistent cache path
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Created temp directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }
    debug!("Set XDG_CONFIG_HOME to: {:?}", test_root);

    fs::write(test_root.join("movie.mp4"), "video").unwrap();
    fs::write(test_root.join("movie.srt"), "sub").unwrap();
    debug!("Created test files: movie.mp4 and movie.srt");

    // Verify files exist and get their actual sizes
    let video_path = test_root.join("movie.mp4");
    let subtitle_path = test_root.join("movie.srt");
    debug!(
        "Video file exists: {}, size: {:?}",
        video_path.exists(),
        fs::metadata(&video_path).map(|m| m.len())
    );
    debug!(
        "Subtitle file exists: {}, size: {:?}",
        subtitle_path.exists(),
        fs::metadata(&subtitle_path).map(|m| m.len())
    );

    // Scan files to get actual file IDs (non-recursive to match the command args)
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, false).unwrap();
    debug!("Scanned directory, found {} files", files.len());
    for file in &files {
        debug!(
            "  File: {} (id: {}, extension: {:?})",
            file.name, file.id, file.extension
        );
    }

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();
    debug!(
        "Found video file: {} (id: {})",
        video_file.name, video_file.id
    );
    debug!(
        "Found subtitle file: {} (id: {})",
        subtitle_file.name, subtitle_file.id
    );

    // Create mock AI service using actual file IDs, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Created mock AI helper at: {}", mock_helper.base_url());
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;
    debug!("Set up mock expectation for 1 API call");

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    debug!("Built config service with mock AI server");

    // First execution dry-run to create cache
    debug!("Executing first match command (dry-run) to create cache");
    let args_preview = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();
    debug!("First match command execution completed");

    // Second execution of the same dry-run operation, should use cache (same directory)
    debug!("Executing second match command (should use cache)");
    let args_second = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(test_root.to_path_buf()), // Use the same directory
        dry_run: true,                       // Keep the same mode
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();
    debug!("Second match command execution completed");

    // Verify mock server received only one request
    debug!("Verifying mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectations verified successfully");
}

#[tokio::test]
async fn test_cache_reuse_preserves_move_mode() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions while avoiding await while holding lock
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_cache_reuse_preserves_move_mode");

    // Use TempDir for consistent cache path
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Created temp directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }
    debug!("Set XDG_CONFIG_HOME to: {:?}", test_root);

    fs::write(test_root.join("movie2.mp4"), "video").unwrap();
    fs::write(test_root.join("movie2.srt"), "sub").unwrap();
    debug!("Created test files: movie2.mp4 and movie2.srt");

    // Verify files exist and get their actual sizes
    let video_path = test_root.join("movie2.mp4");
    let subtitle_path = test_root.join("movie2.srt");
    debug!(
        "Video file exists: {}, size: {:?}",
        video_path.exists(),
        fs::metadata(&video_path).map(|m| m.len())
    );
    debug!(
        "Subtitle file exists: {}, size: {:?}",
        subtitle_path.exists(),
        fs::metadata(&subtitle_path).map(|m| m.len())
    );

    // Scan files to get actual file IDs (non-recursive to match the command args)
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, false).unwrap();
    debug!("Scanned directory, found {} files", files.len());
    for file in &files {
        debug!(
            "  File: {} (id: {}, extension: {:?})",
            file.name, file.id, file.extension
        );
    }

    let video_file = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();
    debug!(
        "Found video file: {} (id: {})",
        video_file.name, video_file.id
    );
    debug!(
        "Found subtitle file: {} (id: {})",
        subtitle_file.name, subtitle_file.id
    );

    // Create mock AI service using actual file IDs, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Created mock AI helper at: {}", mock_helper.base_url());
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&video_file.id, &subtitle_file.id),
            1,
        )
        .await;
    debug!("Set up mock expectation for 1 API call");

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    debug!("Built config service with mock AI server");

    // First execution dry-run to create cache
    debug!("Executing first match command (dry-run) to create cache");
    let args_preview = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();
    debug!("First match command execution completed");

    // Second execution of the same dry-run operation, should use cache
    debug!("Executing second match command (should use cache)");
    let args_second = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(test_root.to_path_buf()),
        dry_run: true, // Keep the same mode
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_second, &config_service)
        .await
        .unwrap();
    debug!("Second match command execution completed");

    // Verify mock server received only one request
    debug!("Verifying mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectations verified successfully");
}
