//! `--quiet` JSON-mode discipline test (tasks.md §11.4).
//!
//! Asserts that `--output json --quiet` produces:
//!
//! * stdout: exactly one valid JSON envelope (terminated by `\n`,
//!   ANSI-clean, indicatif-clean) — the existing JSON stdout contract;
//! * stderr: completely empty on the success path. `tracing` chatter
//!   that may appear without `--quiet` MUST be suppressed.
//!
//! A non-quiet baseline run is also performed to confirm the test
//! exercises a real difference: when `--quiet` is dropped, stderr MAY
//! carry tracing diagnostics (no strict assertion — the quiet path is
//! the contract).
//!
//! Wired into the test crate via `tests/output_format_quiet_tests.rs`.

use crate::common::json_output::{assert_envelope, assert_json_stdout_clean};
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n\
                          2\n00:00:02,500 --> 00:00:03,500\nSecond cue\n\n";

fn isolated_cmd(workdir: &std::path::Path) -> Command {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        .env_remove("RUST_LOG")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(30));
    cmd
}

#[test]
fn quiet_json_mode_suppresses_stderr_on_convert_success() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.srt");
    let output = dir.path().join("a.ass");
    fs::write(&input, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args([
            "--quiet",
            "--output",
            "json",
            "convert",
            input.to_str().unwrap(),
            "--format",
            "ass",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();

    let env = assert_json_stdout_clean(&out.stdout);
    assert_envelope(&env, "convert", "ok");

    assert!(
        out.stderr.is_empty(),
        "stderr must be empty under --quiet --output json on the \
         success path, got: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn quiet_json_mode_suppresses_stderr_on_detect_encoding_success() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("utf8.srt");
    fs::write(&p, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args([
            "--quiet",
            "--output",
            "json",
            "detect-encoding",
            p.to_str().unwrap(),
        ])
        .assert()
        .success();
    let out = assert.get_output();

    let env = assert_json_stdout_clean(&out.stdout);
    assert_envelope(&env, "detect-encoding", "ok");

    assert!(
        out.stderr.is_empty(),
        "stderr must be empty under --quiet --output json on the \
         success path, got: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Baseline: without `--quiet`, the JSON stdout envelope contract is
/// still honored. Stderr is intentionally NOT asserted here because
/// tracing chatter is permitted in the non-quiet path; this test
/// merely guarantees the `--quiet`-vs-no-`--quiet` test pair exercises
/// a real distinction.
#[test]
fn json_mode_without_quiet_still_emits_clean_envelope() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("utf8.srt");
    fs::write(&p, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "detect-encoding", p.to_str().unwrap()])
        .assert()
        .success();
    let out = assert.get_output();

    let env = assert_json_stdout_clean(&out.stdout);
    assert_envelope(&env, "detect-encoding", "ok");
}
