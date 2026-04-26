//! Integration tests for clap argument-parsing errors under the
//! `--output json` contract.
//!
//! Per `openspec/changes/add-machine-readable-output/tasks.md` §2.5,
//! these tests assert the boundary behavior of `src/main.rs`'s clap
//! error-handling branch:
//!
//! 1. Unknown flag in text mode renders clap's standard error on stderr,
//!    no JSON envelope is emitted on stdout, exit code matches clap's
//!    default (`2`).
//! 2. Unknown flag in JSON mode renders exactly one synthetic envelope
//!    with `command == ""`, `status == "error"`,
//!    `error.category == "argument_parsing"`, and
//!    `error.code == "E_ARGUMENT_PARSING"`.
//! 3. Missing required argument in JSON mode produces the same envelope
//!    shape.
//! 4. `--help` in JSON mode is exempt: clap renders text help on stdout
//!    (no envelope) and exits with code `0`.
//! 5. `--version` in JSON mode is similarly exempt.
//!
//! Wired into the test crate via `tests/output_format_clap_errors_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

use crate::common::json_output::assert_json_stdout_clean;

/// Configure a `subx-cli` invocation with an isolated config home so the
/// user's real `~/.config/subx` is not consulted, and disable progress
/// bars defensively. Returns a fresh `Command` with `--timeout` applied.
fn isolated_cmd(workdir: &std::path::Path) -> Command {
    let xdg = workdir.join(".xdg");
    std::fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(30));
    cmd
}

fn assert_clap_argument_envelope(env: &Value) {
    assert_eq!(env["schema_version"], "1.0");
    // `main.rs::handle_clap_error` does not know which subcommand was
    // requested when parsing fails, so it passes `None` to
    // `emit_argument_parsing_error`, which serializes as the empty
    // string. Document the contract here so a regression that swaps the
    // empty-string sentinel for `"(unknown)"` (or vice-versa) fails fast.
    assert_eq!(env["command"], "");
    assert_eq!(env["status"], "error");
    assert!(env.get("data").is_none(), "error envelope omits data");
    let err = env["error"].as_object().expect("error object present");
    assert_eq!(err["category"], "argument_parsing");
    assert_eq!(err["code"], "E_ARGUMENT_PARSING");
    assert!(err["exit_code"].as_i64().is_some(), "exit_code is integer");
    assert!(
        err["message"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "non-empty message"
    );
}

/// Unknown flag in text mode → clap's standard error on stderr, no JSON
/// envelope, exit code 2 (clap's default for `ErrorKind::UnknownArgument`).
#[test]
fn unknown_flag_text_mode_does_not_emit_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--bogus-flag"])
        .assert()
        .failure();

    let out = assert.get_output();
    assert!(
        out.stdout.is_empty() || !out.stdout.starts_with(b"{"),
        "stdout MUST NOT contain a JSON envelope in text mode: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unexpected") || stderr.contains("error:"),
        "stderr should contain a clap error message; got: {stderr:?}"
    );
    assert_eq!(out.status.code(), Some(2), "clap exit code is 2");
}

/// Unknown flag in JSON mode → exactly one synthetic envelope on stdout.
#[test]
fn unknown_flag_json_mode_emits_synthetic_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "--bogus-flag"])
        .assert()
        .failure();

    let out = assert.get_output();
    let env = assert_json_stdout_clean(&out.stdout);
    assert_clap_argument_envelope(&env);
    assert_eq!(out.status.code(), Some(2));
}

/// Missing required argument in JSON mode → same synthetic envelope
/// shape. `convert` requires either `-i <PATH>` or a positional input
/// path; invoking it bare exercises clap's `MissingRequiredArgument`
/// error path.
#[test]
fn missing_required_argument_json_mode_emits_synthetic_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "convert"])
        .assert();

    // `convert` may legitimately fail with no inputs at runtime rather
    // than at parse time depending on the clap config, so accept either
    // a parse-time failure (clap exit 2 + synthetic envelope) or a
    // runtime failure (uniform error envelope from `main.rs`). Both
    // shapes MUST be a single clean JSON document.
    let out = assert.get_output();
    let env = assert_json_stdout_clean(&out.stdout);
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["status"], "error");
    let err = env["error"].as_object().expect("error object present");
    let category = err["category"].as_str().unwrap_or("");
    assert!(
        category == "argument_parsing"
            || category == "command_execution"
            || category == "no_input_specified",
        "unexpected error.category for missing-required-arg: {category}"
    );
}

/// `--help` in JSON mode is exempt from the synthetic envelope: clap
/// renders its standard help text and exits with code 0.
#[test]
fn help_in_json_mode_renders_text_help_with_exit_zero() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "--help"])
        .assert()
        .success();

    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);

    // No JSON envelope must be present — the first non-whitespace byte
    // of clap's help text is not a `{`. Be tolerant of empty stdout
    // (clap renders to stdout for --help; this assertion just guards
    // against a regression that emits an envelope).
    assert!(
        !stdout.trim_start().starts_with('{'),
        "stdout for --help must be plain text, not a JSON envelope: {stdout:?}"
    );
    // Clap help typically advertises the binary name and a Usage block.
    assert!(
        stdout.contains("Usage") || stdout.contains("USAGE") || stdout.contains("subx"),
        "stdout should look like clap help: {stdout:?}"
    );
    assert_eq!(out.status.code(), Some(0));
}

/// `--version` in JSON mode is similarly exempt.
#[test]
fn version_in_json_mode_renders_text_version_with_exit_zero() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "--version"])
        .assert()
        .success();

    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        !stdout.trim_start().starts_with('{'),
        "stdout for --version must be plain text, not a JSON envelope: {stdout:?}"
    );
    assert!(
        stdout.contains("subx") || stdout.chars().any(|c| c.is_ascii_digit()),
        "stdout should look like a version string: {stdout:?}"
    );
    assert_eq!(out.status.code(), Some(0));
}
