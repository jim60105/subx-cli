//! Integration tests for copy mode behavior ensuring original files are preserved.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::mock_openai_helper::MockOpenAITestHelper;

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

    // The matcher generates fresh UUIDv7 IDs on every scan, so use the
    // request-echoing mock to pair the discovered video against the
    // discovered subtitle without us having to pre-derive their IDs.
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_echoing_request_ids(1, 1, 0.95)
        .await;

    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
        no_extract: false,
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

    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_echoing_request_ids(1, 1, 0.95)
        .await;

    let args = MatchArgs {
        input_paths: vec![],
        recursive: true,
        path: Some(root.to_path_buf()),
        dry_run: false,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
        no_extract: false,
    };
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    match_command::execute(args, &config_service).await.unwrap();

    let original_subtitle = subtitle_dir.join("sub.srt");
    let copied_to_video_dir = video_dir.join("movie.srt");

    assert!(
        original_subtitle.exists(),
        "Original file should remain unchanged"
    );
    assert!(
        copied_to_video_dir.exists(),
        "Target location should have a copy"
    );

    assert_eq!(
        fs::read(&original_subtitle).unwrap(),
        fs::read(&copied_to_video_dir).unwrap(),
        "Copied content should be the same as the original file"
    );
}
