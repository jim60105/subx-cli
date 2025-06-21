//! 輸入路徑處理測試：測試各種輸入路徑組合的處理方式
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 測試多個 -i 參數的使用，只有字幕文件應該被跳過
#[tokio::test]
async fn test_multiple_input_flag_usage() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    for d in &["one", "two"] {
        let dir = ws.join(d);
        fs::create_dir_all(&dir).await.unwrap();
        let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
        fs::copy(&src, dir.join("subtitle.srt")).await.unwrap();
    }
    let result = helper
        .run_command_with_config(&["sync", "-i", "one", "-i", "two", "--batch"])
        .await;
    assert!(result.success);

    // 驗證字幕文件被跳過，因為沒有對應的視頻文件
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for") && output.contains("no video files found in directory")
    );

    // 確保沒有創建同步文件
    assert!(!ws.join("one/subtitle_synced.srt").exists());
    assert!(!ws.join("two/subtitle_synced.srt").exists());
}

/// 測試 -i 與位置參數混合，只有字幕文件應該被跳過
#[tokio::test]
async fn test_input_flag_with_positional_args() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("in")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("subtitle.srt")).await.unwrap();
    fs::copy(&src, ws.join("in/subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "-i", "in", "subtitle.srt"])
        .await;
    assert!(result.success);

    // 驗證字幕文件被跳過，因為沒有對應的視頻文件
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for") && output.contains("no video files found in directory")
    );

    // 確保沒有創建同步文件
    assert!(!ws.join("subtitle_synced.srt").exists());
    assert!(!ws.join("in/subtitle_synced.srt").exists());
}

/// 測試檔案與目錄混合輸入，只有字幕文件應該被跳過
#[tokio::test]
async fn test_mixed_file_directory_inputs() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("dir")).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("dir/subtitle.srt")).await.unwrap();
    fs::copy(&src, ws.join("subtitle2.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&["sync", "dir", "subtitle2.srt"])
        .await;
    assert!(result.success);

    // 驗證字幕文件被跳過，因為沒有對應的視頻文件
    let output = format!("{}\n{}", result.stdout, result.stderr);
    assert!(
        output.contains("✗ Skip sync for") && output.contains("no video files found in directory")
    );

    // 確保沒有創建同步文件
    assert!(!ws.join("dir/subtitle_synced.srt").exists());
    assert!(!ws.join("subtitle2_synced.srt").exists());
}
