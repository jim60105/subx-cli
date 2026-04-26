//! `jq`-based scripting smoke test (tasks.md §11.3).
//!
//! Verifies that a successful `--output json` invocation produces an
//! envelope queryable via `jq -e '.status == "ok"'`, and that a failing
//! invocation produces an envelope queryable via
//! `jq -e '.status == "error" and .error.code != null'`.
//!
//! When `jq` is not installed on the test host (CI matrix may vary),
//! the test prints a skip notice on stderr and passes — the schema
//! itself is exercised by the other discipline tests.
//!
//! Wired into the test crate via `tests/output_format_jq_tests.rs`.

use assert_cmd::Command;
use std::fs;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use tempfile::TempDir;

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n";

fn jq_available() -> bool {
    StdCommand::new("jq")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Pipe `stdin_bytes` through `jq -e <filter>` and return whether jq
/// exited successfully (i.e. the filter produced a truthy value).
fn jq_test(filter: &str, stdin_bytes: &[u8]) -> bool {
    let mut child = StdCommand::new("jq")
        .arg("-e")
        .arg(filter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn jq");
    child
        .stdin
        .as_mut()
        .expect("jq stdin")
        .write_all(stdin_bytes)
        .expect("write to jq stdin");
    let output = child.wait_with_output().expect("wait jq");
    output.status.success()
}

fn isolated_cmd(workdir: &std::path::Path) -> Command {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(30));
    cmd
}

#[test]
fn jq_status_ok_on_successful_envelope() {
    if !jq_available() {
        eprintln!("jq not available, skipping jq_status_ok_on_successful_envelope");
        return;
    }

    let dir = TempDir::new().unwrap();
    let p = dir.path().join("utf8.srt");
    fs::write(&p, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "detect-encoding", p.to_str().unwrap()])
        .assert()
        .success();

    let stdout = assert.get_output().stdout.clone();
    assert!(
        jq_test(r#".status == "ok""#, &stdout),
        "jq filter `.status == \"ok\"` failed; stdout: {}",
        String::from_utf8_lossy(&stdout)
    );
}

#[test]
fn jq_status_error_with_code_on_failed_envelope() {
    if !jq_available() {
        eprintln!("jq not available, skipping jq_status_error_with_code_on_failed_envelope");
        return;
    }

    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("does_not_exist.srt");

    let assert = isolated_cmd(dir.path())
        .args([
            "--output",
            "json",
            "detect-encoding",
            missing.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stdout = assert.get_output().stdout.clone();
    assert!(
        jq_test(r#".status == "error" and .error.code != null"#, &stdout,),
        "jq filter for error envelope failed; stdout: {}",
        String::from_utf8_lossy(&stdout)
    );
}
