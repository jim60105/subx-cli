use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_manual_sync_cli_interface() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");

    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
測試內容
"#;
    fs::write(&subtitle_path, srt_content).unwrap();

    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("sync")
        .arg("--offset")
        .arg("2.0")
        .arg(&subtitle_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("手動偏移套用完成"));
}

#[test]
fn test_auto_sync_missing_video_error() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "test").unwrap();

    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("sync")
        .arg(&subtitle_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("視頻檔案在自動同步模式下是必填的"));
}
