//! Integration tests for `subx-cli --output json cache ...` and the
//! legacy `cache status --json` alias.
//!
//! These tests spawn the compiled binary (via `assert_cmd`) with an
//! isolated `XDG_CONFIG_HOME` and assert that stdout contains exactly
//! one JSON envelope conforming to the `cache-management` and
//! `machine-readable-output` specs.
//!
//! Wired into the test crate via `tests/output_format_cache_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
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
    assert_eq!(env["command"], "cache");
    assert_eq!(env["status"], status);
}

/// Build an isolated `Command` whose `XDG_CONFIG_HOME` points at a
/// dedicated temp directory so the test cannot read or mutate the
/// user's real `~/.config/subx/` state.
fn isolated_cmd(workdir: &Path) -> Command {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(xdg.join("subx")).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        // Isolate from the developer's shell: the AI-provider env vars
        // (e.g. `OPENAI_BASE_URL=http://...`) must not leak in, otherwise
        // the strict config gate (hosted provider + `http://` base URL)
        // rejects the merged config and the CLI exits with a config error.
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
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(30));
    cmd
}

fn cache_dir(workdir: &Path) -> std::path::PathBuf {
    let dir = workdir.join(".xdg").join("subx");
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Write a minimal valid `match_cache.json` payload referencing real
/// (existing) source files so subsequent `cache apply` calls can
/// distinguish missing vs. apply-able operations.
fn write_sample_cache(cache_path: &Path, ai_model: &str, ops: &[(String, String, String)]) {
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let operations: Vec<serde_json::Value> = ops
        .iter()
        .map(|(video, sub, new_name)| {
            serde_json::json!({
                "video_file": video,
                "subtitle_file": sub,
                "new_subtitle_name": new_name,
                "confidence": 0.95,
                "reasoning": [],
            })
        })
        .collect();
    let payload = serde_json::json!({
        "cache_version": "1.0",
        "directory": "/tmp",
        "file_snapshot": [],
        "match_operations": operations,
        "created_at": created_at,
        "ai_model_used": ai_model,
        "config_hash": "deadbeefcafebabe",
        "original_relocation_mode": "None",
        "original_backup_enabled": false,
    });
    fs::write(cache_path, serde_json::to_string(&payload).unwrap()).unwrap();
}

// ─── cache status ───────────────────────────────────────────────────────

#[test]
fn cache_status_empty_emits_zero_counters_envelope() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "cache", "status"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    assert!(env.get("error").is_none());
    let data = &env["data"];
    assert_eq!(data["exists"], false);
    assert_eq!(data["total"], 0);
    assert_eq!(data["pending"], 0);
    assert_eq!(data["applied"], 0);
}

#[test]
fn cache_status_populated_emits_nonzero_total() {
    let dir = TempDir::new().unwrap();
    let cache_file = cache_dir(dir.path()).join("match_cache.json");
    write_sample_cache(
        &cache_file,
        "gpt-4",
        &[
            ("/tmp/v1.mkv".into(), "/tmp/s1.srt".into(), "v1.srt".into()),
            ("/tmp/v2.mkv".into(), "/tmp/s2.srt".into(), "v2.srt".into()),
        ],
    );

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "cache", "status"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    let data = &env["data"];
    assert_eq!(data["exists"], true);
    assert_eq!(data["total"], 2);
    assert_eq!(data["operation_count"], 2);
    assert_eq!(data["ai_model"], "gpt-4");
    assert!(data["size_bytes"].as_u64().unwrap() > 0);
}

/// §8.4 — Legacy `cache status --json` MUST produce the same JSON value
/// as `--output json cache status` against identical state.
#[test]
fn cache_status_legacy_json_alias_matches_global_output_json() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    // Both invocations operate on an empty cache so the only variance
    // would be in path strings (XDG isolation differs). We therefore
    // strip the `path` field — driven by per-test temp dir — before
    // comparing JSON values, but we keep every other field equal.
    let global = isolated_cmd(dir1.path())
        .args(["--output", "json", "cache", "status"])
        .assert()
        .success();
    let legacy = isolated_cmd(dir2.path())
        .args(["cache", "status", "--json"])
        .assert()
        .success();

    let mut env_global = parse_single_envelope(&global.get_output().stdout);
    let mut env_legacy = parse_single_envelope(&legacy.get_output().stdout);

    // Strip dir-dependent fields before equality check.
    env_global["data"]["path"] = Value::Null;
    env_legacy["data"]["path"] = Value::Null;

    assert_eq!(
        env_global, env_legacy,
        "legacy `--json` alias must produce the same JSON value as `--output json`"
    );
    // Sanity check: both envelopes carry the right shape.
    assert_envelope_shape(&env_global, "ok");
    assert_envelope_shape(&env_legacy, "ok");
}

// ─── cache clear ────────────────────────────────────────────────────────

#[test]
fn cache_clear_emits_removed_counter() {
    let dir = TempDir::new().unwrap();
    let cache_file = cache_dir(dir.path()).join("match_cache.json");
    let journal_file = cache_dir(dir.path()).join("match_journal.json");
    fs::write(&cache_file, "{}").unwrap();
    fs::write(&journal_file, "{}").unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "cache", "clear"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    let data = &env["data"];
    assert_eq!(data["removed"], 2);
    assert_eq!(data["cache_removed"], true);
    assert_eq!(data["journal_removed"], true);
    assert_eq!(data["kind"], "all");
    assert!(!cache_file.exists());
    assert!(!journal_file.exists());
}

#[test]
fn cache_clear_empty_emits_zero_removed() {
    let dir = TempDir::new().unwrap();
    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "cache", "clear"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    assert_eq!(env["data"]["removed"], 0);
    assert_eq!(env["data"]["cache_removed"], false);
    assert_eq!(env["data"]["journal_removed"], false);
}

// ─── cache apply (mixed success/failure) ────────────────────────────────

#[test]
fn cache_apply_mixed_success_and_missing_file_emits_per_item_status() {
    let dir = TempDir::new().unwrap();
    let cache_file = cache_dir(dir.path()).join("match_cache.json");

    // Build one apply-able op (real files) and one that will fail (missing
    // subtitle file).
    let media_dir = dir.path().join("media");
    fs::create_dir_all(&media_dir).unwrap();
    let video_ok = media_dir.join("good.mkv");
    let sub_ok = media_dir.join("good.srt");
    fs::write(&video_ok, b"fake mkv bytes").unwrap();
    fs::write(&sub_ok, b"1\n00:00:01,000 --> 00:00:02,000\nhi\n\n").unwrap();

    let video_missing = media_dir.join("missing.mkv");
    let sub_missing = media_dir.join("missing.srt");
    // Intentionally do not create these files.

    write_sample_cache(
        &cache_file,
        "gpt-4",
        &[
            (
                video_ok.to_string_lossy().into(),
                sub_ok.to_string_lossy().into(),
                "good.srt".into(),
            ),
            (
                video_missing.to_string_lossy().into(),
                sub_missing.to_string_lossy().into(),
                "missing.srt".into(),
            ),
        ],
    );

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "cache", "apply", "--yes", "--force"])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "ok");
    assert!(env.get("error").is_none());

    let data = &env["data"];
    let items = data["items"].as_array().expect("items array");
    assert_eq!(items.len(), 2, "should have one item per cache entry");

    let applied = data["applied"].as_u64().unwrap();
    let failed = data["failed"].as_u64().unwrap();
    assert_eq!(
        applied + failed,
        items.len() as u64,
        "applied + failed must equal items.len()"
    );
    assert!(failed >= 1, "missing-file entry must be counted as failed");

    // Locate the missing-file item and assert per-item error envelope.
    let missing_item = items
        .iter()
        .find(|i| i["id"].as_str().unwrap().contains("missing.srt"))
        .expect("missing-file item must be present");
    assert_eq!(missing_item["status"], "error");
    let err = &missing_item["error"];
    assert!(err.is_object(), "missing-file item must carry error object");
    assert_eq!(err["category"], "file_not_found");
    assert_eq!(err["code"], "E_FILE_NOT_FOUND");
    assert!(err["message"].is_string());
}
