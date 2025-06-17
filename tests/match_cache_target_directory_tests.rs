//! Integration tests: Match command cache target directory correctness
//!
//! Test scenarios:
//! 1. Execute dry-run to generate cache
//! 2. Execute actual copy/move operations
//! 3. Verify files are copied/moved to the correct directory (video file directory, not subtitle directory)

use log::debug;
use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// Use async mutex to avoid environment variable race conditions
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_match_cache_copy_mode_target_directory_correctness() {
    // Initialize logging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test: match cache copy mode target directory correctness");

    // Create test directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Create test directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }
    debug!("Set XDG_CONFIG_HOME to: {:?}", test_root);

    // Create separate video and subtitle directory structure
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create test files
    let video_file = video_dir.join("movie.mp4");
    let subtitle_file = subtitle_dir.join("movie.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    debug!("Create test files:");
    debug!("  Video file: {}", video_file.display());
    debug!("  Subtitle file: {}", subtitle_file.display());

    // Scan files to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();
    debug!("Scanned directory, found {} files", files.len());

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();
    debug!(
        "Found video file: {} (id: {})",
        video_file_info.name, video_file_info.id
    );
    debug!(
        "Found subtitle file: {} (id: {})",
        subtitle_file_info.name, subtitle_file_info.id
    );

    // Create mock AI service, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Create mock AI helper at: {}", mock_helper.base_url());

    // Set to expect only one API call (for the first execution)
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;
    debug!("Set mock expectation to 1 API call");

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    debug!("Create config service with mock AI server");

    // First execution: dry-run to create cache
    debug!("Execute first match command (dry-run) to create cache");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();
    debug!("First match command execution completed");

    // Verify original files still exist
    assert!(video_file.exists(), "Video file should still exist");
    assert!(subtitle_file.exists(), "Subtitle file should still exist");

    // Second execution: actual copy operation (using cache)
    debug!("Execute second match command (copy mode, should use cache)");
    let args_copy = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false, // Actual execution
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_copy, &config_service)
        .await
        .unwrap();
    debug!("Second match command execution completed");

    // Key verification: Check if files are copied to the correct directory
    let expected_copy_location = video_dir.join("movie.srt");
    let wrong_location = subtitle_dir.join("movie.srt"); // Original location (should not have new files)

    debug!("Verify file locations:");
    debug!(
        "  Expected copy location: {}",
        expected_copy_location.display()
    );
    debug!("  Wrong location: {}", wrong_location.display());

    // Main verification: File should be copied to video directory
    assert!(
        expected_copy_location.exists(),
        "Subtitle file should be copied to video file directory: {}",
        expected_copy_location.display()
    );

    // Verify original file still exists (copy mode)
    assert!(
        subtitle_file.exists(),
        "Original subtitle file should still exist: {}",
        subtitle_file.display()
    );

    // Verify copied file content is correct
    let original_content = fs::read_to_string(&subtitle_file).unwrap();
    let copied_content = fs::read_to_string(&expected_copy_location).unwrap();
    assert_eq!(
        original_content, copied_content,
        "Copied file content should match original file"
    );

    // Verify mock server received only one request
    debug!("Verify mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectation verification successful");

    debug!("✅ Match cache copy mode target directory correctness test successful");
}

#[tokio::test]
async fn test_match_cache_dry_run_vs_actual_execution_consistency() {
    // Initialize logging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test: match cache dry-run vs actual execution consistency");

    // Create test directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Create separate video and subtitle directory structure
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create test files
    let video_file = video_dir.join("test_movie.mp4");
    let subtitle_file = subtitle_dir.join("test_subtitle.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    // Scan files to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // Create mock AI service, set to expect only one API call
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // First execution: dry-run to create cache
    debug!("Execute dry-run to create cache");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();

    // Verify no files are actually moved or copied after dry-run
    let expected_copy_location = video_dir.join("test_movie.srt");
    assert!(
        !expected_copy_location.exists(),
        "dry-run should not create actual files"
    );

    // Second execution: actual copy operation (using cache)
    debug!("Execute actual copy operation");
    let args_actual = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_actual, &config_service)
        .await
        .unwrap();

    // Verify actual execution copied files to correct location
    assert!(
        expected_copy_location.exists(),
        "Actual execution should copy files to correct location: {}",
        expected_copy_location.display()
    );

    // Verify file content consistency
    let original_content = fs::read_to_string(&subtitle_file).unwrap();
    let copied_content = fs::read_to_string(&expected_copy_location).unwrap();
    assert_eq!(original_content, copied_content);

    // Verify original file still exists (copy mode)
    assert!(subtitle_file.exists(), "Original file should still exist");

    // Verify mock server received only one request
    mock_helper.verify_expectations().await;

    debug!("✅ Dry-run vs actual execution consistency verification successful");
}

#[tokio::test]
async fn test_match_cache_move_mode_target_directory_correctness() {
    // Initialize logging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test: match cache move mode target directory correctness");

    // Create test directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Create separate video and subtitle directory structure
    let video_dir = test_root.join("videos");
    let subtitle_dir = test_root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    // Create test files
    let video_file = video_dir.join("move_test.mp4");
    let subtitle_file = subtitle_dir.join("move_test.srt");
    fs::write(&video_file, "fake video content").unwrap();
    fs::write(
        &subtitle_file,
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    )
    .unwrap();

    // Scan files to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(test_root, true).unwrap();

    let video_file_info = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle_file_info = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    // Create mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(
                &video_file_info.id,
                &subtitle_file_info.id,
            ),
            1,
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // First execution: dry-run to create cache (move mode)
    debug!("Execute move mode dry-run to create cache");
    let args_dry_run = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_dry_run, &config_service)
        .await
        .unwrap();

    // Second execution: actual move operation (using cache)
    debug!("Execute actual move operation");
    let args_move = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(test_root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_move, &config_service)
        .await
        .unwrap();

    // Verify files are moved to correct location
    let expected_move_location = video_dir.join("move_test.srt");
    assert!(
        expected_move_location.exists(),
        "Subtitle file should be moved to video file directory: {}",
        expected_move_location.display()
    );

    // Verify original file no longer exists (move mode)
    assert!(
        !subtitle_file.exists(),
        "Original subtitle file should have been moved: {}",
        subtitle_file.display()
    );

    // Verify mock server received only one request
    mock_helper.verify_expectations().await;

    debug!("✅ Move mode cache target directory correctness verification successful");
}
