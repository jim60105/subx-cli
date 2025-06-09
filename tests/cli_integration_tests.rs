// tests/cli_integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

/// CLI 主體整合測試: 版本、說明與錯誤指令行為驗證
#[tokio::test]
async fn test_version_display() {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("subx-cli"));
}

#[tokio::test]
async fn test_help_display() {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("智慧字幕處理 CLI 工具"));
}

#[tokio::test]
async fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[tokio::test]
async fn test_config_show_basic() {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.args(&["config", "show"]).assert().success();
}
