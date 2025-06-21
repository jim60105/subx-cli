//! 參數組合整合測試：覆蓋 README 中記錄的 sync 用法
mod common;
use common::cli_helpers::CLITestHelper;
use std::path::PathBuf;
use tokio::fs;

/// subx-cli sync video.mp4 subtitle.srt
#[tokio::test]
async fn test_basic_video_subtitle_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let src_video = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_video, ws.join("video.mp4")).await.unwrap();
    fs::copy(&src_srt, ws.join("subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "video.mp4", "subtitle.srt"])
        .await;
    assert!(ws.join("subtitle_synced.srt").exists());
}

/// subx-cli sync --offset 2.5 subtitle.srt
#[tokio::test]
async fn test_manual_offset_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_srt, ws.join("subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&["sync", "--offset", "2.5", "subtitle.srt"])
        .await;
    assert!(ws.join("subtitle_synced.srt").exists());
}

/// subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt
#[tokio::test]
async fn test_vad_sensitivity_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let src_video = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let src_srt = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src_video, ws.join("video.mp4")).await.unwrap();
    fs::copy(&src_srt, ws.join("subtitle.srt")).await.unwrap();
    helper
        .run_command_expect_success(&[
            "sync",
            "--vad-sensitivity",
            "0.8",
            "video.mp4",
            "subtitle.srt",
        ])
        .await;
    assert!(ws.join("subtitle_synced.srt").exists());
}

/// subx-cli sync --batch <directory>
#[tokio::test]
async fn test_batch_directory_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    // copy assets folder to temp
    let src_dir = PathBuf::from("assets");
    fs::create_dir_all(ws.join("media")).await.unwrap();
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.mp4"),
        ws.join("media/video.mp4"),
    )
    .await
    .unwrap();
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.srt"),
        ws.join("media/subtitle.srt"),
    )
    .await
    .unwrap();
    helper
        .run_command_expect_success(&["sync", "--batch", "media"])
        .await;
    assert!(ws.join("media/subtitle_synced.srt").exists());
}

/// subx-cli sync -i ./movies_directory --batch
#[tokio::test]
async fn test_batch_input_path_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    fs::create_dir_all(ws.join("movies")).await.unwrap();
    let src_dir = PathBuf::from("assets");
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.mp4"),
        ws.join("movies/video.mp4"),
    )
    .await
    .unwrap();
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.srt"),
        ws.join("movies/subtitle.srt"),
    )
    .await
    .unwrap();
    helper
        .run_command_expect_success(&["sync", "-i", "movies", "--batch"])
        .await;
    assert!(ws.join("movies/subtitle_synced.srt").exists());
}

/// subx-cli sync -i ./movies_directory --batch --recursive
#[tokio::test]
async fn test_batch_recursive_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let nested = ws.join("dir/a");
    fs::create_dir_all(&nested).await.unwrap();
    let src_dir = PathBuf::from("assets");
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.mp4"),
        nested.join("video.mp4"),
    )
    .await
    .unwrap();
    fs::copy(
        &src_dir.join("SubX - The Subtitle Revolution.srt"),
        nested.join("subtitle.srt"),
    )
    .await
    .unwrap();
    helper
        .run_command_expect_success(&["sync", "-i", "dir", "--batch", "--recursive"])
        .await;
    assert!(nested.join("subtitle_synced.srt").exists());
}

/// subx-cli sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad
#[tokio::test]
async fn test_multiple_input_batch_recursive_vad_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    for d in &["m1", "m2", "tv"] {
        let subdir = ws.join(d);
        fs::create_dir_all(&subdir).await.unwrap();
        let src = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
        let ss = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
        fs::copy(&src, subdir.join("video.mp4")).await.unwrap();
        fs::copy(&ss, subdir.join("subtitle.srt")).await.unwrap();
    }
    helper
        .run_command_expect_success(&[
            "sync",
            "-i",
            "m1",
            "-i",
            "m2",
            "-i",
            "tv",
            "--recursive",
            "--batch",
            "--method",
            "vad",
        ])
        .await;
    for d in &["m1", "m2", "tv"] {
        assert!(ws.join(d).join("subtitle_synced.srt").exists());
    }
}

/// subx-cli sync -i ./media --batch --recursive --dry-run --verbose
#[tokio::test]
async fn test_batch_recursive_dry_run_verbose_sync() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let nested = ws.join("media");
    fs::create_dir_all(&nested).await.unwrap();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let ss = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, nested.join("video.mp4")).await.unwrap();
    fs::copy(&ss, nested.join("subtitle.srt")).await.unwrap();
    let result = helper
        .run_command_with_config(&[
            "sync",
            "-i",
            "media",
            "--batch",
            "--recursive",
            "--dry-run",
            "--verbose",
        ])
        .await;
    assert!(result.success);
    // dry-run 不應產生檔案
    assert!(!nested.join("subtitle_synced.srt").exists());
}

#[tokio::test]
async fn test_maximal_parameter_combinations() {
    let helper = CLITestHelper::new();
    let ws = helper.temp_dir_path().to_path_buf();
    let src = PathBuf::from("assets/SubX - The Subtitle Revolution.mp4");
    let ss = PathBuf::from("assets/SubX - The Subtitle Revolution.srt");
    fs::copy(&src, ws.join("video.mp4")).await.unwrap();
    fs::copy(&ss, ws.join("subtitle.srt")).await.unwrap();
    let args = [
        "sync",
        "video.mp4",
        "subtitle.srt",
        "--offset",
        "1.0",
        "--vad-sensitivity",
        "0.5",
        "--method",
        "vad",
        "--dry-run",
        "--verbose",
        "--force",
    ];
    helper.run_command_expect_success(&args).await;
    // dry-run 模式不會寫檔
    assert!(!ws.join("subtitle_synced.srt").exists());
}

/// 測試衝突參數處理
#[tokio::test]
async fn test_conflicting_parameter_handling() {
    let helper = CLITestHelper::new();
    // 同時指定 --offset 與 --method vad 可能為衝突組合
    let result = helper
        .run_command_with_config(&["sync", "--offset", "2.0", "--method", "vad", "subtitle.srt"])
        .await;
    assert!(!result.success);
}
