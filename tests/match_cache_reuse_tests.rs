//! Integration tests for match command cache reuse with copy/move modes.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;
mod common;
use common::{MatchResponseGenerator, MockOpenAITestHelper};

#[tokio::test]
async fn test_cache_reuse_preserves_copy_mode() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    // 清除先前測試的快取檔案以確保測試隔離
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("subx");
        dir.push("match_cache.json");
        let _ = std::fs::remove_file(dir);
    }
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "sub").unwrap();

    // 建立 mock AI 服務，設定只期望一次 API 呼叫（第二次應使用快取）
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_single_match(),
            1,
        )
        .await;

    let args_preview = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    let args_execute = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_execute, &config_service)
        .await
        .unwrap();

    // 驗證 mock server 只收到一次請求
    mock_helper.verify_expectations().await;
}

#[tokio::test]
async fn test_cache_reuse_preserves_move_mode() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    // 清除先前測試的快取檔案以確保測試隔離
    if let Some(mut dir) = dirs::config_dir() {
        dir.push("subx");
        dir.push("match_cache.json");
        let _ = std::fs::remove_file(dir);
    }
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "sub").unwrap();

    // 建立 mock AI 服務，設定只期望一次 API 呼叫（第二次應使用快取）
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper
        .mock_chat_completion_with_expectation(
            &MatchResponseGenerator::successful_single_match(),
            1,
        )
        .await;

    let args_preview = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(&mock_helper.base_url())
        .build_service();
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    let args_execute = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_execute, &config_service)
        .await
        .unwrap();

    // 驗證 mock server 只收到一次請求
    mock_helper.verify_expectations().await;
}
