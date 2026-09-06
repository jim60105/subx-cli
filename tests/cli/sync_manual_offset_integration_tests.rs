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

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("sync")
        .arg("--verbose")
        .arg("--offset")
        .arg("2.0")
        .arg(&subtitle_path)
        .assert()
        .success()
        // Characterised (B4 revival): the "Using manual offset:" line is only
        // printed under --verbose, together with the results block and the
        // save to the default `<stem>_synced.srt` output path.
        .stdout(predicate::str::contains("Using manual offset:"))
        .stdout(predicate::str::contains("Method used: Manual"))
        .stdout(predicate::str::contains("test_synced.srt"));
}

#[test]
fn test_auto_sync_missing_video_error() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "test").unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("sync")
        .arg(&subtitle_path)
        .assert()
        .failure()
        // Characterised (B4 revival): the sync command now rejects a
        // subtitle-only automatic invocation during configuration
        // validation, before reaching the "Video file path is required
        // for automatic sync" check inside the sync engine.
        .stderr(predicate::str::contains(
            "Invalid sync configuration: please specify video and subtitle files",
        ));
}
