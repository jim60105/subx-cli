//! Integration tests for copy mode behavior ensuring original files are preserved.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::{
    mock_openai_helper::MockOpenAITestHelper, test_data_generators::MatchResponseGenerator,
};

#[tokio::test]
async fn test_copy_mode_preserves_original_file() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    let subtitle_path = subtitle_dir.join("sub.srt");
    fs::write(&subtitle_path, b"content").unwrap();

    // First scan files to get actual file IDs
    let discovery = subx_cli::core::matcher::FileDiscovery::new();
    let files = discovery.scan_directory(root, true).unwrap();
    let video_file = files
        .iter()
        .find(|f| matches!(f.file_type, subx_cli::core::matcher::MediaFileType::Video))
        .unwrap();
    let subtitle_file = files
        .iter()
        .find(|f| {
            matches!(
                f.file_type,
                subx_cli::core::matcher::MediaFileType::Subtitle
            )
        })
        .unwrap();

    // Create mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::successful_match_with_ids(
            &video_file.id,
            &subtitle_file.id,
        ))
        .await;

    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    match_command::execute(args, &config_service).await.unwrap();

    let target = video_dir.join("movie.srt");
    assert!(subtitle_path.exists(), "Original file should be preserved");
    assert!(target.exists(), "Target location should have a copy");
    assert_eq!(
        fs::read(&subtitle_path).unwrap(),
        fs::read(&target).unwrap(),
        "Copied content should be the same as the original"
    );
}

#[tokio::test]
async fn test_copy_mode_with_rename() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    let subtitle_path = subtitle_dir.join("sub.srt");
    fs::write(&subtitle_path, b"content").unwrap();

    // First scan files to get actual file IDs
    let discovery = subx_cli::core::matcher::FileDiscovery::new();
    let files = discovery.scan_directory(root, true).unwrap();
    let video_file = files
        .iter()
        .find(|f| matches!(f.file_type, subx_cli::core::matcher::MediaFileType::Video))
        .unwrap();
    let subtitle_file = files
        .iter()
        .find(|f| {
            matches!(
                f.file_type,
                subx_cli::core::matcher::MediaFileType::Subtitle
            )
        })
        .unwrap();

    // Create mock AI service
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::successful_match_with_ids(
            &video_file.id,
            &subtitle_file.id,
        ))
        .await;

    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    match_command::execute(args, &config_service).await.unwrap();

    let original_subtitle = subtitle_dir.join("sub.srt");
    let copied_to_video_dir = video_dir.join("movie.srt");

    // In Copy mode, the original file should remain unchanged
    assert!(
        original_subtitle.exists(),
        "Original file should remain unchanged"
    );
    assert!(
        copied_to_video_dir.exists(),
        "Target location should have a copy"
    );

    // Check if the copied content is correct
    assert_eq!(
        fs::read(&original_subtitle).unwrap(),
        fs::read(&copied_to_video_dir).unwrap(),
        "Copied content should be the same as the original file"
    );
}
