//! Smoke tests for `subx-cli --output json config ...`.
//!
//! Per `openspec/changes/add-machine-readable-output/tasks.md` §10.3,
//! this file verifies the four `config` subcommands (`get`, `set`,
//! `list`, plus an invalid-key error path) emit a single JSON envelope
//! conforming to the `machine-readable-output` spec.
//!
//! Wired into the test crate via `tests/output_format_config_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn parse_single_envelope(stdout: &[u8]) -> Value {
    assert!(!stdout.is_empty(), "stdout was empty");
    assert!(
        stdout.ends_with(b"\n"),
        "stdout did not end with newline: {:?}",
        String::from_utf8_lossy(stdout)
    );
    assert!(
        !stdout.contains(&0x1b),
        "stdout contained ANSI escape sequence: {:?}",
        String::from_utf8_lossy(stdout)
    );
    let body = &stdout[..stdout.len() - 1];
    assert!(
        !body.contains(&b'\n'),
        "stdout contained more than one line: {:?}",
        String::from_utf8_lossy(body)
    );
    serde_json::from_slice(body).expect("stdout parses as JSON")
}

fn assert_envelope_shape(env: &Value, status: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], "config");
    assert_eq!(env["status"], status);
}

/// Build an isolated `Command` whose configuration directory is a
/// temporary directory, so the test cannot read or mutate the user's
/// real `~/.config/subx/config.toml`.
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
fn config_get_emits_single_key_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "get", "ai.provider"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    let cfg = env["data"]["config"]
        .as_object()
        .expect("data.config object");
    assert_eq!(cfg.len(), 1, "config map should hold exactly one entry");
    assert!(
        cfg.contains_key("ai.provider"),
        "config map missing `ai.provider`: {cfg:?}"
    );
    assert!(env.get("error").is_none());
}

#[test]
fn config_set_emits_key_value_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "set", "ai.provider", "openai"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    let data = env["data"].as_object().expect("data object");
    assert_eq!(data["key"], "ai.provider");
    assert_eq!(data["value"], "openai");
    assert!(env.get("error").is_none());
}

#[test]
fn config_list_emits_full_config_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "list"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    let cfg = env["data"]["config"]
        .as_object()
        .expect("data.config object");
    // The full configuration map must include the well-known top-level
    // sections defined by `subx_cli::config::Config`.
    for section in ["ai", "general", "formats", "sync"] {
        assert!(
            cfg.contains_key(section),
            "config list missing section `{section}`: keys = {:?}",
            cfg.keys().collect::<Vec<_>>()
        );
    }
    assert!(env.get("error").is_none());
}

#[test]
fn config_get_invalid_key_emits_error_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args([
            "--output",
            "json",
            "config",
            "get",
            "this.key.does.not.exist",
        ])
        .assert()
        .failure();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "error");
    assert!(env.get("data").is_none());
    let err = env["error"].as_object().expect("error object");
    assert!(err.get("category").is_some());
    assert!(err.get("code").is_some());
    assert!(err.get("message").is_some());
    assert!(
        err.get("exit_code").is_some(),
        "error envelope must carry exit_code"
    );
}
