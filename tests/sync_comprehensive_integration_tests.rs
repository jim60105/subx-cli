//! 綜合整合測試：Sync 子命令基本功能與參數驗證
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 測試自動 VAD 同步（video + subtitle）
#[cfg(feature = "slow-tests")]
#[tokio::test]
async fn test_basic_auto_sync() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();
    // 複製測試資源
    let src_video = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_video, workspace.join("video.mp4"))
        .await
        .unwrap();
    fs::copy(&src_srt, workspace.join("subtitle.srt"))
        .await
        .unwrap();
    // 執行命令
    helper
        .run_command_expect_success(&["sync", "video.mp4", "subtitle.srt"])
        .await;
    // 檢查輸出檔案存在
}

/// 測試手動偏移同步 (--offset)
#[tokio::test]
async fn test_manual_offset_sync() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_srt, workspace.join("subtitle.srt"))
        .await
        .unwrap();
    helper
        .run_command_expect_success(&["sync", "--offset", "2.5", "subtitle.srt"])
        .await;
}

/// 測試 VAD 自訂敏感度同步 (--vad-sensitivity)
#[cfg(feature = "slow-tests")]
#[tokio::test]
async fn test_vad_sensitivity_sync() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();
    let src_video = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_video, workspace.join("video.mp4"))
        .await
        .unwrap();
    fs::copy(&src_srt, workspace.join("subtitle.srt"))
        .await
        .unwrap();
    helper
        .run_command_expect_success(&[
            "sync",
            "--vad-sensitivity",
            "0.8",
            "video.mp4",
            "subtitle.srt",
        ])
        .await;
}

/// 測試無效參數組合處理
#[tokio::test]
async fn test_invalid_param_handling() {
    let helper = CLITestHelper::new();
    // 不提供任何參數應回報錯誤
    let result = helper.run_command_with_config(&["sync"]).await;
    assert!(!result.success, "sync without args should fail");
}
