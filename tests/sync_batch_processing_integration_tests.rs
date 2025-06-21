//! 批次處理整合測試：批次模式與選項組合
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 基本批次處理 - 只有字幕文件應該被跳過
#[tokio::test]
async fn test_basic_batch_processing() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "--batch", "media"])
        .await;

    // 驗證命令成功執行，但跳過了沒有對應視頻的字幕文件
    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("subtitle.srt")
            && output.contains("no video files found in directory"),
        "Should skip subtitle file when no video files exist in directory. Output: {}",
        output
    );
    assert!(
        !ws.join("media/subtitle_synced.srt").exists(),
        "Should not create synced file for subtitle without video"
    );
}

/// 遞歸批次處理 - 只有字幕文件應該被跳過
#[tokio::test]
async fn test_recursive_batch_processing() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let nested = ws.join("media/x");
    fs::create_dir_all(&nested).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, nested.join("subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--recursive"])
        .await;

    // 驗證命令成功執行，但跳過了沒有對應視頻的字幕文件
    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("subtitle.srt")
            && output.contains("no video files found in directory"),
        "Should skip subtitle file when no video files exist in directory. Output: {}",
        output
    );
    assert!(
        !nested.join("subtitle_synced.srt").exists(),
        "Should not create synced file for subtitle without video"
    );
}

/// 大型目錄批次處理（模擬大量文件）- 只有字幕文件應該被跳過
#[tokio::test]
async fn test_large_directory_batch_processing() {
    let helper = CLITestHelper::new();
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
    let result = helper
        .run_command_with_config(&["sync", "--batch", "bulk", "--recursive"])
        .await;

    // 驗證命令成功執行，但跳過了沒有對應視頻的字幕文件
    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 檢查所有字幕文件都被跳過，並且訊息說明沒有視頻文件
    for i in 0..5 {
        assert!(
            output.contains(&format!("subtitle{}.srt", i))
                && output.contains("no video files found in directory"),
            "Should mention subtitle{}.srt in skip message with 'no video files found'. Output: {}",
            i,
            output
        );
    }

    // 確保沒有生成任何同步後的文件
    for i in 0..5 {
        assert!(
            !dir.join(format!("subtitle{}_synced.srt", i)).exists(),
            "Should not create synced file for subtitle{}.srt",
            i
        );
    }
}

/// 批次 + 試運行
#[tokio::test]
async fn test_batch_dry_run_combination() {
    let helper = CLITestHelper::new();
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

/// 批次 + 詳細輸出 - 只有字幕文件應該被跳過
#[tokio::test]
async fn test_batch_verbose_combination() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--verbose"])
        .await;

    // 驗證命令成功執行，但跳過了沒有對應視頻的字幕文件
    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("subtitle.srt")
            && output.contains("no video files found in directory"),
        "Should skip subtitle file when no video files exist in directory. Output: {}",
        output
    );
    assert!(
        !ws.join("media/subtitle_synced.srt").exists(),
        "Should not create synced file for subtitle without video"
    );
}

/// 批次 + 方法選擇 - 只有字幕文件應該被跳過
#[tokio::test]
async fn test_batch_method_selection_combination() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--method", "vad"])
        .await;

    // 驗證命令成功執行，但跳過了沒有對應視頻的字幕文件
    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("subtitle.srt")
            && output.contains("no video files found in directory"),
        "Should skip subtitle file when no video files exist in directory. Output: {}",
        output
    );
    assert!(
        !ws.join("media/subtitle_synced.srt").exists(),
        "Should not create synced file for subtitle without video"
    );
}
