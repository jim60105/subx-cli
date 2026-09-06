// tests/cli_integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

/// CLI main integration tests: version, help and error command behavior validation
#[tokio::test]
async fn test_version_display() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("subx-cli"));
}

#[tokio::test]
async fn test_help_display() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Intelligent subtitle processing CLI tool",
        ));
}

#[tokio::test]
async fn test_invalid_command() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("invalid-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}
