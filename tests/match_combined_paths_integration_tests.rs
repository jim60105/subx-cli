//! Integration tests for match command with combined path and input_paths parameters.

use log::debug;
use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::mock_openai_helper::MockOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

// Using async mutex to avoid environment variable race conditions
static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn test_match_with_combined_path_and_input_paths_simple() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_with_combined_path_and_input_paths_simple");

    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Created temp directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Create main directory structure
    let main_dir = test_root.join("main");
    let input_dir = test_root.join("input");

    fs::create_dir_all(&main_dir).unwrap();
    fs::create_dir_all(&input_dir).unwrap();

    // Create files in main directory only
    fs::write(main_dir.join("main_video.mp4"), "main video content").unwrap();
    fs::write(main_dir.join("main_subtitle.srt"), "main subtitle content").unwrap();

    debug!("Created test files in main directory");

    // Scan main directory to get file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let main_files = discovery.scan_directory(&main_dir, false).unwrap();

    debug!("Found {} files in main directory", main_files.len());

    // Get file information for mock setup
    let main_video = main_files
        .iter()
        .find(|f| f.name.ends_with(".mp4"))
        .unwrap();
    let main_subtitle = main_files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .unwrap();

    // Create mock AI service with response for main directory
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Created mock AI helper at: {}", mock_helper.base_url());

    // Mock response for main directory matching
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_match_with_ids(&main_video.id, &main_subtitle.id),
            1, // Expecting one call for main directory
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // Execute match command with both path and input_paths (empty input_dir)
    debug!("Executing match command with combined paths");
    let args = MatchArgs {
        path: Some(main_dir.clone()),         // Main directory as path
        input_paths: vec![input_dir.clone()], // Empty input directory
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_ok(),
        "Match command should succeed with combined paths"
    );

    debug!("Match command execution completed successfully");

    // Verify mock server received expected requests
    debug!("Verifying mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectations verified successfully");
}

#[tokio::test]
async fn test_match_with_only_input_paths() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_with_only_input_paths");

    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Created temp directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Create input directories
    let input1_dir = test_root.join("input1");
    let input2_dir = test_root.join("input2");

    fs::create_dir_all(&input1_dir).unwrap();
    fs::create_dir_all(&input2_dir).unwrap();

    // Create files in input directories
    fs::write(input1_dir.join("video1.mp4"), "video1 content").unwrap();
    fs::write(input1_dir.join("subtitle1.srt"), "subtitle1 content").unwrap();
    fs::write(input2_dir.join("video2.mkv"), "video2 content").unwrap();
    fs::write(input2_dir.join("subtitle2.srt"), "subtitle2 content").unwrap();

    debug!("Created test files in two input directories");

    // Scan directories to get file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let input1_files = discovery.scan_directory(&input1_dir, false).unwrap();
    let input2_files = discovery.scan_directory(&input2_dir, false).unwrap();

    debug!(
        "Found {} files in input1, {} in input2",
        input1_files.len(),
        input2_files.len()
    );

    // Get file information for mock setup
    let video1 = input1_files
        .iter()
        .find(|f| f.name.ends_with(".mp4"))
        .unwrap();
    let subtitle1 = input1_files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .unwrap();
    let video2 = input2_files
        .iter()
        .find(|f| f.name.ends_with(".mkv"))
        .unwrap();
    let subtitle2 = input2_files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .unwrap();

    // Create mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Created mock AI helper at: {}", mock_helper.base_url());

    // Mock a single response for combined input paths
    mock_helper
        .mock_chat_completion_with_expectation(&MatchResponseGenerator::multiple_matches(), 1)
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // Execute match command with only input_paths (no path parameter)
    debug!("Executing match command with only input_paths");
    let args = MatchArgs {
        path: None,                                                // No main path
        input_paths: vec![input1_dir.clone(), input2_dir.clone()], // Only input paths
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_ok(),
        "Match command should succeed with only input_paths"
    );

    debug!("Match command execution completed successfully");

    // Verify mock server received expected requests
    debug!("Verifying mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectations verified successfully");
}

#[tokio::test]
async fn test_match_with_file_and_directory_inputs() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_with_file_and_directory_inputs");

    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();
    debug!("Created temp directory: {:?}", test_root);

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Create main directory and input directory
    let main_dir = test_root.join("main");
    let input_dir = test_root.join("input");

    fs::create_dir_all(&main_dir).unwrap();
    fs::create_dir_all(&input_dir).unwrap();

    // Create files
    fs::write(main_dir.join("video1.mp4"), "video1 content").unwrap();
    fs::write(main_dir.join("subtitle1.srt"), "subtitle1 content").unwrap();
    let input_video_file = input_dir.join("video2.mkv");
    fs::write(&input_video_file, "video2 content").unwrap();
    fs::write(input_dir.join("subtitle2.srt"), "subtitle2 content").unwrap();

    debug!("Created test files in main directory and input directory");

    // Scan directories to get file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let main_files = discovery.scan_directory(&main_dir, false).unwrap();
    let input_files = discovery.scan_directory(&input_dir, false).unwrap();

    debug!(
        "Found {} files in main, {} in input",
        main_files.len(),
        input_files.len()
    );

    // Get file information for mock setup
    let video1 = main_files
        .iter()
        .find(|f| f.name.ends_with(".mp4"))
        .unwrap();
    let subtitle1 = main_files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .unwrap();
    let video2 = input_files
        .iter()
        .find(|f| f.name.ends_with(".mkv"))
        .unwrap();
    let subtitle2 = input_files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .unwrap();

    // Create mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    debug!("Created mock AI helper at: {}", mock_helper.base_url());

    // Mock a single response for mixed file and directory inputs
    mock_helper
        .mock_chat_completion_with_expectation(&MatchResponseGenerator::multiple_matches(), 1)
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();

    // Execute match command with directory as path and specific file as input_path
    debug!("Executing match command with directory path and file input_path");
    let args = MatchArgs {
        path: Some(main_dir.clone()),                // Directory as main path
        input_paths: vec![input_video_file.clone()], // Specific file as input path
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(
        result.is_ok(),
        "Match command should succeed with mixed directory and file inputs"
    );

    debug!("Match command execution completed successfully");

    // Verify mock server received expected requests
    debug!("Verifying mock expectations");
    mock_helper.verify_expectations().await;
    debug!("Mock expectations verified successfully");
}
