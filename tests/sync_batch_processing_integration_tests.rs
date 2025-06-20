//! 批次處理整合測試：批次模式與選項組合
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 基本批次處理
#[tokio::test]
async fn test_basic_batch_processing() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "--batch", "media"])
        .await;
    assert!(ws.join("media/subtitle_synced.srt").exists());
}

/// 遞歸批次處理
#[tokio::test]
async fn test_recursive_batch_processing() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let nested = ws.join("media/x");
    fs::create_dir_all(&nested).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, nested.join("subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "--batch", "media", "--recursive"])
        .await;
    assert!(nested.join("subtitle_synced.srt").exists());
}

/// 大型目錄批次處理（模擬大量文件）
#[tokio::test]
async fn test_large_directory_batch_processing() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let dir = ws.join("bulk");
    fs::create_dir_all(&dir).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    // 模擬多個檔案
    for i in 0..5 {
        fs::copy(&src, dir.join(format!("subtitle{}.srt", i)))
            .await
            .unwrap();
    }
    helper
        .run_command_expect_success(&["sync", "--batch", "bulk", "--recursive"])
        .await;
    for i in 0..5 {
        assert!(dir.join(format!("subtitle{}_synced.srt", i)).exists());
    }
}

/// 批次 + 試運行
#[tokio::test]
async fn test_batch_dry_run_combination() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--dry-run"])
        .await;
    assert!(result.success);
    assert!(!ws.join("media/subtitle_synced.srt").exists());
}

/// 批次 + 詳細輸出
#[tokio::test]
async fn test_batch_verbose_combination() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "--batch", "media", "--verbose"])
        .await;
    assert!(ws.join("media/subtitle_synced.srt").exists());
}

/// 批次 + 方法選擇
#[tokio::test]
async fn test_batch_method_selection_combination() {
    let mut helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "--batch", "media", "--method", "vad"])
        .await;
    assert!(ws.join("media/subtitle_synced.srt").exists());
}
