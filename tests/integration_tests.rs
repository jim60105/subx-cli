use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// 建立測試用的媒體與字幕檔案
fn create_test_media_files(dir: &Path) {
    // 建立 dummy 影片檔案
    let video_path = dir.join("video.mp4");
    fs::write(&video_path, b"").unwrap();
    // 建立簡單的 SRT 字幕檔案
    let srt_path = dir.join("video.srt");
    let srt_content = "\
1
00:00:00,000 --> 00:00:01,000
Hello
";
    fs::write(&srt_path, srt_content).unwrap();
}

/// 測試 match 命令的 dry-run 功能
fn test_match_command(dir: &Path) {
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.env("OPENAI_API_KEY", "test")
        .arg("match")
        .arg(dir)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("預覽模式"));
}

/// 測試 convert 命令將 SRT 轉換為 VTT
fn test_convert_command(dir: &Path) {
    let subtitle = dir.join("video.srt");
    let output = dir.join("video.vtt");
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("convert")
        .arg(&subtitle)
        .arg("--format")
        .arg("vtt")
        .assert()
        .success();
    // 檢查輸出檔案與內容
    assert!(output.exists(), "轉換後的 VTT 檔案不存在");
    let content = fs::read_to_string(output).unwrap();
    assert!(content.contains("WEBVTT"), "輸出內容不包含 WEBVTT 標頭");
}

/// 測試 sync 命令的手動偏移功能
fn test_sync_command(dir: &Path) {
    let video = dir.join("video.mp4");
    let subtitle = dir.join("video.srt");
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("sync")
        .arg(&video)
        .arg(&subtitle)
        .arg("--offset")
        .arg("1")
        .arg("--method")
        .arg("manual")
        .assert()
        .success()
        .stdout(predicate::str::contains("已應用手動偏移"));
}

#[test]
fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();

    create_test_media_files(dir);
    test_match_command(dir);
    test_convert_command(dir);
    test_sync_command(dir);
}
