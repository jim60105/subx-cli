//! 輸入路徑處理整合測試：-i 與位置參數混合
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 測試多個 -i 參數
#[tokio::test]
async fn test_multiple_input_flag_usage() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    for d in &["one", "two"] {
        let dir = ws.join(d);
        fs::create_dir_all(&dir).await.unwrap();
        let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
        fs::copy(&src, dir.join("subtitle.srt")).await.unwrap();
    }
    helper
        .run_command_expect_success(&["sync", "-i", "one", "-i", "two", "--batch"])
        .await;
    assert!(ws.join("one/subtitle_synced.srt").exists());
    assert!(ws.join("two/subtitle_synced.srt").exists());
}

/// 測試 -i 與位置參數混合
#[tokio::test]
async fn test_input_flag_with_positional_args() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("in")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("subtitle.srt")).await.unwrap();
    fs::copy(&src, ws.join("in/subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "-i", "in", "subtitle.srt"])
        .await;
    assert!(ws.join("subtitle_synced.srt").exists());
    assert!(ws.join("in/subtitle_synced.srt").exists());
}

/// 測試檔案與目錄混合輸入
#[tokio::test]
async fn test_mixed_file_directory_inputs() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("dir")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("dir/subtitle.srt")).await.unwrap();
    fs::copy(&src, ws.join("subtitle2.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "dir", "subtitle2.srt"])
        .await;
    assert!(ws.join("dir/subtitle_synced.srt").exists());
    assert!(ws.join("subtitle2_synced.srt").exists());
}

/// 測試相對與絕對路徑處理
#[tokio::test]
async fn test_relative_absolute_path_handling() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("dir")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("dir/subtitle.srt")).await.unwrap();
    let abs = ws.join("dir");
    let rel = PathBuf::from("dir");
    // 模擬切換工作目錄
    std::env::set_current_dir(&ws).unwrap();
    helper
        .run_command_expect_success(&["sync", abs.to_str().unwrap()])
        .await;
    helper
        .run_command_expect_success(&["sync", "dir/subtitle.srt"])
        .await;
    assert!(ws.join("dir/subtitle_synced.srt").exists());
}
