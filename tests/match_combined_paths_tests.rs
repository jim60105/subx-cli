//! Integration tests for combined path and input_paths functionality in match command.

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
async fn test_match_command_file_discovery_with_combined_paths() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_command_file_discovery_with_combined_paths");

    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // Create main path directory
    let main_dir = test_root.join("main");
    fs::create_dir_all(&main_dir).unwrap();

    // Create input path directories
    let input1_dir = test_root.join("input1");
    let input2_dir = test_root.join("input2");
    fs::create_dir_all(&input1_dir).unwrap();
    fs::create_dir_all(&input2_dir).unwrap();

    // Create test files in each directory
    fs::write(main_dir.join("main_video.mp4"), "video").unwrap();
    fs::write(main_dir.join("main_subtitle.srt"), "subtitle").unwrap();

    fs::write(input1_dir.join("input1_video.mkv"), "video").unwrap();
    fs::write(input1_dir.join("input1_subtitle.ass"), "subtitle").unwrap();

    fs::write(input2_dir.join("input2_video.avi"), "video").unwrap();
    fs::write(input2_dir.join("input2_subtitle.vtt"), "subtitle").unwrap();

    debug!("Created test directory structure with files");

    // Test the input handler logic directly
    let args = MatchArgs {
        path: Some(main_dir.clone()),                              // Main path
        input_paths: vec![input1_dir.clone(), input2_dir.clone()], // Additional input paths
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    // Test that get_input_handler combines all paths correctly
    let input_handler = args.get_input_handler().unwrap();
    let collected_files = input_handler.collect_files().unwrap();

    debug!(
        "Collected {} files from combined paths",
        collected_files.len()
    );

    // Verify that we found files from all three directories
    assert_eq!(collected_files.len(), 6); // 2 files per directory × 3 directories

    // Check that files from all directories are present
    let file_names: std::collections::HashSet<String> = collected_files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains("main_video.mp4"));
    assert!(file_names.contains("main_subtitle.srt"));
    assert!(file_names.contains("input1_video.mkv"));
    assert!(file_names.contains("input1_subtitle.ass"));
    assert!(file_names.contains("input2_video.avi"));
    assert!(file_names.contains("input2_subtitle.vtt"));

    debug!("Verified that all expected files were discovered");
}

#[tokio::test]
async fn test_match_command_simple_execution_with_combined_paths() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_command_simple_execution_with_combined_paths");

    // Create a simple test with just one directory for working AI mock
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // Create main path directory only
    let main_dir = test_root.join("main");
    fs::create_dir_all(&main_dir).unwrap();

    // Create test files
    fs::write(main_dir.join("video.mp4"), "video").unwrap();
    fs::write(main_dir.join("subtitle.srt"), "subtitle").unwrap();

    debug!("Created simple test directory structure");

    // Set cache directory
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_root);
    }

    // Scan to get actual file IDs
    use subx_cli::core::matcher::FileDiscovery;
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(&main_dir, false).unwrap();

    let video_file = files.iter().find(|f| f.name.contains("video")).unwrap();
    let subtitle_file = files.iter().find(|f| f.name.contains("subtitle")).unwrap();

    debug!("Found video: {} ({})", video_file.name, video_file.id);
    debug!(
        "Found subtitle: {} ({})",
        subtitle_file.name, subtitle_file.id
    );

    // Create mock AI service using actual file IDs
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

    // Execute match command with path parameter (traditional way)
    let args = MatchArgs {
        path: Some(main_dir.clone()),
        input_paths: vec![], // No additional input paths for this simple test
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    debug!("Executing match command with simple path");
    let result = match_command::execute(args, &config_service).await;

    match result {
        Ok(()) => {
            debug!("Match command executed successfully");
            mock_helper.verify_expectations().await;
            debug!("Mock expectations verified");
        }
        Err(e) => {
            debug!("Match command failed: {:?}", e);
            panic!(
                "Match command should have succeeded but failed with: {:?}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_match_command_with_individual_files_and_directories() {
    // Initialize logger for debugging
    let _ = env_logger::try_init();

    // Use async mutex to avoid environment variable race conditions
    let _guard = TEST_MUTEX.lock().await;
    debug!("Starting test_match_command_with_individual_files_and_directories");

    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let test_root = temp_dir.path();

    // Create directories
    let dir1 = test_root.join("dir1");
    let dir2 = test_root.join("dir2");
    fs::create_dir_all(&dir1).unwrap();
    fs::create_dir_all(&dir2).unwrap();

    // Create files in directories
    let video1 = dir1.join("video1.mp4");
    let subtitle1 = dir1.join("subtitle1.srt");
    let video2 = dir2.join("video2.mkv");
    let subtitle2 = dir2.join("subtitle2.srt");

    fs::write(&video1, "video1").unwrap();
    fs::write(&subtitle1, "subtitle1").unwrap();
    fs::write(&video2, "video2").unwrap();
    fs::write(&subtitle2, "subtitle2").unwrap();

    debug!("Created test files in separate directories");

    // Test the input handler logic with mixed file and directory inputs
    let args = MatchArgs {
        path: None, // No main path
        input_paths: vec![
            video1.clone(),    // Individual file (will use its parent directory)
            dir2.clone(),      // Directory
            subtitle1.clone(), // Another individual file (same parent as video1)
        ],
        recursive: false,
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
    };

    // Test that get_input_handler handles mixed inputs correctly
    let input_handler = args.get_input_handler().unwrap();
    let collected_files = input_handler.collect_files().unwrap();

    debug!(
        "Collected {} files from mixed inputs",
        collected_files.len()
    );

    // Verify that we found files from both directories
    // Should find: video1.mp4, subtitle1.srt (from dir1) and video2.mkv, subtitle2.srt (from dir2)
    assert_eq!(collected_files.len(), 4);

    // Check that files from both directories are present
    let file_names: std::collections::HashSet<String> = collected_files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains("video1.mp4"));
    assert!(file_names.contains("subtitle1.srt"));
    assert!(file_names.contains("video2.mkv"));
    assert!(file_names.contains("subtitle2.srt"));

    debug!("Verified that files from both directories were discovered via mixed inputs");
}
