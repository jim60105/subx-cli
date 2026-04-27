//! Integration tests for the `config set/get/list` repair path.
//!
//! Per `openspec/changes/fix-config-set-bypass-on-invalid-existing-config/`:
//! when an existing `~/.config/subx/config.toml` fails cross-section
//! validation (for example because of a `provider=openai + http://`
//! pairing), the user must still be able to inspect (`get`, `list`)
//! and repair (`set`) the file. Non-`config` commands must still
//! abort at config load with the existing strict-validation error.
//!
//! Wired into the test crate via
//! `tests/config_set_repair_invalid_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn config_dir_for(workdir: &Path) -> std::path::PathBuf {
    workdir.join(".config-home").join("subx")
}

fn config_path_for(workdir: &Path) -> std::path::PathBuf {
    config_dir_for(workdir).join("config.toml")
}

/// Seed a full-shape `config.toml` under `workdir/.xdg/subx/config.toml`
/// with a strict-invalid `provider=openai + base_url=http://...`
/// pairing. Uses `config reset` to write a complete default file
/// first, then patches the offending fields by string substitution
/// so the fixture stays in sync with the current `Config` schema
/// without requiring this test to track every field.
fn seed_strict_invalid_config(workdir: &Path) -> std::path::PathBuf {
    let path = config_path_for(workdir);
    // Write a default-shaped file via `subx-cli config reset` (which
    // does not depend on the file pre-existing).
    isolated_cmd(workdir)
        .args(["config", "reset"])
        .assert()
        .success();

    // Patch in the offending pairing. Default config already uses
    // `provider = "openai"`, so we only need to flip `base_url` to
    // an `http://` value to break cross-section validation.
    let body = fs::read_to_string(&path).unwrap();
    let mut patched = String::with_capacity(body.len() + 64);
    let mut base_url_replaced = false;
    let mut api_key_replaced = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("base_url ") || trimmed.starts_with("base_url=") {
            patched.push_str(r#"base_url = "http://localhost:1234/v1""#);
            patched.push('\n');
            base_url_replaced = true;
        } else if trimmed.starts_with("api_key ") || trimmed.starts_with("api_key=") {
            patched.push_str(r#"api_key = "sk-test-1234567890abcdef""#);
            patched.push('\n');
            api_key_replaced = true;
        } else {
            patched.push_str(line);
            patched.push('\n');
        }
    }
    // If api_key was absent (default is None → omitted), insert it.
    if !api_key_replaced {
        // Insert directly under the [ai] section header.
        let with_api = patched.replace("[ai]\n", "[ai]\napi_key = \"sk-test-1234567890abcdef\"\n");
        patched = with_api;
    }
    assert!(
        base_url_replaced,
        "expected to find a base_url line to patch in default config"
    );
    fs::write(&path, &patched).unwrap();
    path
}

fn isolated_cmd(workdir: &Path) -> Command {
    let cfg_dir = config_dir_for(workdir);
    fs::create_dir_all(&cfg_dir).unwrap();
    let cfg_path = config_path_for(workdir);
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    // Pin the config file via `SUBX_CONFIG_PATH` (honored by both the
    // load and save paths in `ProductionConfigService`). This keeps the
    // test cross-platform: `XDG_CONFIG_HOME` is ignored by the `dirs`
    // crate on macOS (which resolves to `~/Library/Application Support`)
    // and on Windows (which uses `%APPDATA%`), so seeding through that
    // var leaves `config reset` writing into the real user config dir
    // and the assertion path empty.
    cmd.env("SUBX_CONFIG_PATH", &cfg_path)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        // Strip every env override that might paper over the file's
        // strict-invalid pairing or substitute different values.
        .env_remove("OPENAI_API_KEY")
        .env_remove("OPENAI_BASE_URL")
        .env_remove("OPENROUTER_API_KEY")
        .env_remove("AZURE_OPENAI_API_KEY")
        .env_remove("AZURE_OPENAI_ENDPOINT")
        .env_remove("AZURE_OPENAI_API_VERSION")
        .env_remove("AZURE_OPENAI_DEPLOYMENT_ID")
        .env_remove("LOCAL_LLM_BASE_URL")
        .env_remove("LOCAL_LLM_API_KEY")
        .env_remove("SUBX_AI_PROVIDER")
        .env_remove("SUBX_AI_APIKEY")
        .env_remove("SUBX_AI_BASE_URL")
        .env_remove("SUBX_AI_MODEL")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(30));
    cmd
}

fn parse_single_envelope(stdout: &[u8]) -> Value {
    assert!(!stdout.is_empty(), "stdout was empty");
    assert!(stdout.ends_with(b"\n"), "stdout did not end with newline");
    let body = &stdout[..stdout.len() - 1];
    serde_json::from_slice(body).expect("stdout parses as JSON")
}

#[test]
fn config_set_repairs_via_provider_switch() {
    let dir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(dir.path());

    isolated_cmd(dir.path())
        .args(["config", "set", "ai.provider", "local"])
        .assert()
        .success();

    let after = fs::read_to_string(&path).unwrap();
    assert!(
        after.contains(r#"provider = "local""#),
        "expected provider = \"local\" in file; got: {after}"
    );
    // base_url and api_key must be preserved verbatim.
    assert!(after.contains(r#"base_url = "http://localhost:1234/v1""#));
    assert!(after.contains(r#"api_key = "sk-test-1234567890abcdef""#));

    // Resulting file must pass strict validation: a follow-up read
    // through any non-config command must succeed (we exercise this
    // via `config list`, which goes through tolerant load now, so
    // instead exercise via `config get` strict path: re-run a
    // non-config command that loads strict and confirm it does NOT
    // emit the cross-section error.
    let out = isolated_cmd(dir.path())
        .args(["config", "get", "ai.provider"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("local"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("configuration is currently invalid"),
        "post-repair stderr should not contain advisory: {stderr}"
    );
}

#[test]
fn config_set_repairs_via_url_switch() {
    let dir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(dir.path());

    isolated_cmd(dir.path())
        .args(["config", "set", "ai.base_url", "https://api.openai.com/v1"])
        .assert()
        .success();

    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains(r#"base_url = "https://api.openai.com/v1""#));
    assert!(after.contains(r#"provider = "openai""#));
}

#[test]
fn config_set_non_repair_edit_is_rejected_and_file_unchanged() {
    let dir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(dir.path());
    let before = fs::read_to_string(&path).unwrap();

    isolated_cmd(dir.path())
        .args(["config", "set", "general.backup_enabled", "false"])
        .assert()
        .failure();

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(before, after, "file must remain byte-identical");
}

#[test]
fn config_get_on_invalid_file_emits_advisory_to_stderr() {
    let dir = TempDir::new().unwrap();
    seed_strict_invalid_config(dir.path());

    let out = isolated_cmd(dir.path())
        .args(["config", "get", "ai.base_url"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("http://localhost:1234/v1"),
        "stdout missing the URL value: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: configuration is currently invalid:"),
        "stderr missing advisory: {stderr}"
    );
}

#[test]
fn config_list_json_on_invalid_file_populates_warnings() {
    let dir = TempDir::new().unwrap();
    seed_strict_invalid_config(dir.path());

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "list"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_eq!(env["status"], "ok");
    let warnings = env["warnings"]
        .as_array()
        .expect("warnings array present on strict-invalid file");
    assert!(!warnings.is_empty(), "warnings array should be non-empty");
    let msg = warnings[0]
        .as_str()
        .expect("warning entry should be a string");
    assert!(
        msg.contains("configuration is currently invalid"),
        "warning text mismatch: {msg}"
    );
}

#[test]
fn config_list_json_on_valid_file_omits_warnings() {
    // A fresh, default-derived config (no strict-invalid pairing).
    let dir = TempDir::new().unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "list"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_eq!(env["status"], "ok");
    assert!(
        env.get("warnings").is_none() || env["warnings"].is_null(),
        "warnings field should be absent or null on strict-valid file: {}",
        env
    );
}

#[test]
fn non_config_command_still_aborts_on_strict_invalid_file() {
    let dir = TempDir::new().unwrap();
    seed_strict_invalid_config(dir.path());

    // `match --dry-run` requires a directory argument and would
    // otherwise reach AI client construction; we rely on the strict
    // validator failing at config load before any of that runs.
    let media_dir = dir.path().join("media");
    fs::create_dir_all(&media_dir).unwrap();

    let out = isolated_cmd(dir.path())
        .args(["match", "--dry-run", media_dir.to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        combined.contains("ai.base_url") || combined.contains("Configuration"),
        "non-config command should surface the strict validation error: {combined}"
    );
}

#[test]
fn config_reset_recovers_from_strict_invalid_file() {
    let dir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(dir.path());

    isolated_cmd(dir.path())
        .args(["config", "reset"])
        .assert()
        .success();

    let after = fs::read_to_string(&path).unwrap();
    // Default config has no api_key set, so the line should not
    // contain the seeded `sk-test` value.
    assert!(
        !after.contains("sk-test"),
        "reset should overwrite seeded fixture: {after}"
    );
    // Default provider is `openai`; default base_url is the official
    // hosted URL (https). Verify https.
    assert!(
        after.contains("https://"),
        "default config should use https base_url: {after}"
    );
}

#[test]
fn repair_does_not_bake_env_only_api_key_into_file() {
    let dir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(dir.path());

    let mut cmd = isolated_cmd(dir.path());
    cmd.env("OPENAI_API_KEY", "sk-fromenv")
        .args(["config", "set", "ai.provider", "local"])
        .assert()
        .success();

    let after = fs::read_to_string(&path).unwrap();
    assert!(
        after.contains(r#"api_key = "sk-test-1234567890abcdef""#),
        "file must keep the file-derived api_key 'sk-test-1234567890abcdef', not the env value 'sk-fromenv': {after}"
    );
    assert!(
        !after.contains("sk-fromenv"),
        "env-only value must NOT leak into the persisted file: {after}"
    );
}

/// A persisted file with a field-level malformed value (here: an
/// `ai.max_sample_length` outside the allowed `[100, 10000]` range)
/// must be rejected by every `config` subcommand. The strict validator
/// alone does not enforce this invariant, so the regression test
/// guards the field-level pass that `load_for_repair` now runs.
#[test]
fn config_set_rejects_field_level_malformed_persisted_value() {
    let workdir = TempDir::new().unwrap();
    let path = seed_strict_invalid_config(workdir.path());
    // Patch in an out-of-range max_sample_length on top of the
    // existing strict-invalid pairing.
    let body = fs::read_to_string(&path).unwrap();
    let patched = body.replace("max_sample_length = 3000", "max_sample_length = 10");
    fs::write(&path, &patched).unwrap();
    let before = fs::read_to_string(&path).unwrap();

    isolated_cmd(workdir.path())
        .args(["config", "set", "ai.provider", "local"])
        .assert()
        .failure();

    let after = fs::read_to_string(&path).unwrap();
    assert_eq!(before, after, "strict-invalid file must be byte-stable");
}
