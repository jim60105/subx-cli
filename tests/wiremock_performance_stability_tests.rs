mod common;
use common::{
    mock_openai_helper::MockOpenAITestHelper, test_data_generators::MatchResponseGenerator,
};

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

/// Load test: Simulate processing many files and verify performance
#[tokio::test]
async fn test_high_load_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create multiple test files
    create_multiple_test_files(root, 50);

    let mock_helper = MockOpenAITestHelper::new().await;
    // Set up response with delay to simulate network conditions
    mock_helper
        .setup_delayed_response(
            100, // 100ms delay
            &MatchResponseGenerator::multiple_matches(),
        )
        .await;

    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .with_parallel_settings(8, 200)
        .build_service();

    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: true,
        confidence: 50,
        backup: true,
        copy: true,
        move_files: false,
    };

    let start_time = std::time::Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_ok());
    println!("High load test completion time: {:?}", elapsed);
    // Verify completion within reasonable time (should be less than 30 seconds)
    assert!(elapsed < std::time::Duration::from_secs(30));
}

/// Memory stability test: Multiple executions to detect potential memory leaks
#[tokio::test]
async fn test_memory_stability() {
    for iteration in 1..=10 {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        create_test_files(root);

        let mock_helper = MockOpenAITestHelper::new().await;
        mock_helper
            .mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
            .await;

        let config_service = TestConfigBuilder::new()
            .with_mock_ai_server(&mock_helper.base_url())
            .build_service();

        let args = MatchArgs {
            input_paths: vec![],
            recursive: true,
            path: Some(root.to_path_buf()),
            dry_run: true,
            confidence: 50,
            backup: true,
            copy: false,
            move_files: false,
        };

        let result = match_command::execute(args, &config_service).await;
        assert!(result.is_ok(), "Iteration {} failed", iteration);

        // Clean up resources
        drop(mock_helper);
        drop(config_service);
        drop(temp_dir);
    }
}

fn create_multiple_test_files(root: &std::path::Path, count: usize) {
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

fn create_test_files(root: &std::path::Path) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();
    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "subtitle").unwrap();
}
