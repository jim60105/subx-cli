//! Integration tests for copy mode behavior ensuring original files are preserved.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::{MatchResponseGenerator, MockOpenAITestHelper};

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

    // 建立 mock AI 服務
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
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
    assert!(subtitle_path.exists(), "原始檔案應保留");
    assert!(target.exists(), "目標位置應有副本");
    assert_eq!(
        fs::read(&subtitle_path).unwrap(),
        fs::read(&target).unwrap(),
        "副本內容應與原始一致"
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

    // 建立 mock AI 服務
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_success(&MatchResponseGenerator::successful_single_match())
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

    // 在 Copy 模式下，原始檔案應保持不變
    assert!(original_subtitle.exists(), "原始檔案應保持不變");
    assert!(copied_to_video_dir.exists(), "目標位置應有副本");

    // 檢查副本內容是否正確
    assert_eq!(
        fs::read(&original_subtitle).unwrap(),
        fs::read(&copied_to_video_dir).unwrap(),
        "副本內容應與原始檔案一致"
    );
}
