//! 測試新的批次同步邏輯：基於 starts_with 的配對和各種情境的處理
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// 測試情境1：目錄中沒有任何視頻文件 - 跳過所有字幕文件
#[tokio::test]
async fn test_batch_no_video_files() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 只創建字幕文件
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/movie1.srt")).await.unwrap();
    fs::copy(&src, ws.join("media/movie2.srt")).await.unwrap();

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 應該顯示跳過訊息，說明沒有找到視頻文件
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("movie1.srt")
            && output.contains("no video files found in directory"),
        "Should skip subtitle files when no video files exist. Output: {}",
        output
    );
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("movie2.srt")
            && output.contains("no video files found in directory"),
        "Should skip all subtitle files when no video files exist. Output: {}",
        output
    );
}

/// 測試情境2：正好一個視頻和一個字幕 - 無論檔名是否匹配都應該同步
#[tokio::test]
async fn test_batch_one_video_one_subtitle_matched_names() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 創建一個視頻和一個字幕文件（檔名匹配）
    fs::write(ws.join("media/movie.mp4"), b"fake video")
        .await
        .unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/movie.srt")).await.unwrap();

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0", "--dry-run"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 應該執行同步，不應該有跳過訊息
    assert!(
        !output.contains("✗ Skip sync for"),
        "Should not skip when there's exactly one video and one subtitle. Output: {}",
        output
    );
    assert!(
        output.contains("Dry run mode"),
        "Should execute sync in dry run mode. Output: {}",
        output
    );
}

/// 測試情境2：正好一個視頻和一個字幕 - 檔名不匹配但仍應該同步
#[tokio::test]
async fn test_batch_one_video_one_subtitle_unmatched_names() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 創建一個視頻和一個字幕文件（檔名不匹配）
    fs::write(ws.join("media/video.mp4"), b"fake video")
        .await
        .unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/subtitle.srt")).await.unwrap();

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0", "--dry-run"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 應該執行同步，不應該有跳過訊息
    assert!(
        !output.contains("✗ Skip sync for"),
        "Should not skip when there's exactly one video and one subtitle, even with unmatched names. Output: {}",
        output
    );
    assert!(
        output.contains("Dry run mode"),
        "Should execute sync in dry run mode. Output: {}",
        output
    );
}

/// 測試情境3：多個文件 - 基於 starts_with 的配對
#[tokio::test]
async fn test_batch_multiple_files_starts_with_matching() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 創建多個視頻和字幕文件
    fs::write(ws.join("media/movie.mp4"), b"fake video")
        .await
        .unwrap();
    fs::write(ws.join("media/series.mkv"), b"fake video")
        .await
        .unwrap();

    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/movie.srt")).await.unwrap(); // 完全匹配
    fs::copy(&src, ws.join("media/movie_extended.srt"))
        .await
        .unwrap(); // starts_with 匹配
    fs::copy(&src, ws.join("media/series_s01e01.srt"))
        .await
        .unwrap(); // starts_with 匹配
    fs::copy(&src, ws.join("media/documentary.srt"))
        .await
        .unwrap(); // 無匹配

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0", "--dry-run"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 檢查配對成功的文件（通過檢查多個同步完成訊息）
    let sync_completed_count = output.matches("Sync completed").count();
    assert!(
        sync_completed_count == 3,
        "Should sync 3 files (movie.srt, movie_extended.srt, series_s01e01.srt). Found {} sync completions. Output: {}",
        sync_completed_count,
        output
    );

    // 檢查跳過的文件
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("documentary.srt")
            && output.contains("no matching video"),
        "documentary.srt should be skipped (no matching video). Output: {}",
        output
    );
}

/// 測試情境3：多個文件 - 影片沒有對應字幕的跳過訊息
#[tokio::test]
async fn test_batch_multiple_files_video_without_subtitle() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 創建多個視頻文件，但只有部分有對應字幕
    fs::write(ws.join("media/movie1.mp4"), b"fake video")
        .await
        .unwrap();
    fs::write(ws.join("media/movie2.mkv"), b"fake video")
        .await
        .unwrap();
    fs::write(ws.join("media/lonely_video.avi"), b"fake video")
        .await
        .unwrap(); // 沒有對應字幕

    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/movie1.srt")).await.unwrap();
    fs::copy(&src, ws.join("media/movie2_version1.srt"))
        .await
        .unwrap(); // starts_with 匹配

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0", "--dry-run"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 檢查配對成功的文件（通過檢查多個同步完成訊息）
    let sync_completed_count = output.matches("Sync completed").count();
    assert!(
        sync_completed_count == 2,
        "Should sync 2 files (movie1.srt, movie2_version1.srt). Found {} sync completions. Output: {}",
        sync_completed_count,
        output
    );

    // 檢查跳過的視頻文件
    assert!(
        output.contains("✗ Skip sync for")
            && output.contains("lonely_video.avi")
            && output.contains("no matching subtitle"),
        "lonely_video.avi should be skipped (no matching subtitle). Output: {}",
        output
    );
}

/// 測試邊界情況：空目錄
#[tokio::test]
async fn test_batch_empty_directory() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("empty")).await.unwrap();

    let result = helper
        .run_command_with_config(&["sync", "--batch", "empty"])
        .await;

    assert!(result.success);
    // 空目錄應該成功執行，沒有任何處理
}

/// 測試檔名包含特殊字符的情況
#[tokio::test]
async fn test_batch_special_characters_in_filenames() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("media")).await.unwrap();

    // 創建包含特殊字符的檔名
    fs::write(ws.join("media/movie-2023.mp4"), b"fake video")
        .await
        .unwrap();

    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("media/movie-2023_cn.srt"))
        .await
        .unwrap(); // starts_with 匹配

    let result = helper
        .run_command_with_config(&["sync", "--batch", "media", "--offset", "1.0", "--dry-run"])
        .await;

    assert!(result.success);
    let output = format!("{}\n{}", result.stdout, result.stderr);

    // 應該成功配對並同步
    assert!(
        output.contains("Sync completed"),
        "Should handle special characters in filenames correctly. Output: {}",
        output
    );
}
