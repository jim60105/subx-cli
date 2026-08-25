//! Cross-command stdout/stderr discipline test (tasks.md §11.2).
//!
//! Runs every covered subcommand with `--output json` against minimal
//! fixtures and asserts via the shared
//! [`crate::common::json_output`] helpers that:
//!
//! * stdout parses as exactly one JSON document terminated by `\n`,
//! * the envelope `schema_version`/`command`/`status` invariants hold,
//! * `data` and `error` fields are mutually exclusive per status.
//!
//! Cache is intentionally excluded — it is owned by a parallel
//! sub-agent and will gain its own discipline coverage there.
//!
//! Wired into the test crate via
//! `tests/output_format_cross_command_tests.rs`.

use crate::common::cli_helpers::CLITestHelper;
use crate::common::json_output::{assert_envelope, assert_json_stdout_clean};
use assert_cmd::Command;
use serde_json::json;
use std::fs;
use subx_cli::core::matcher::FileDiscovery;
use tempfile::TempDir;
use wiremock::matchers::{method, path as wmpath};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n\
                          2\n00:00:02,500 --> 00:00:03,500\nSecond cue\n\n";

/// Build a clap [`Command`] with an isolated XDG/HOME so the test
/// cannot read or mutate the user's real `~/.config/subx`.
fn isolated_cmd(workdir: &std::path::Path) -> Command {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
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
        .timeout(std::time::Duration::from_secs(60));
    cmd
}

#[test]
fn convert_json_mode_is_clean_and_well_formed() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.srt");
    let output = dir.path().join("a.ass");
    fs::write(&input, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args([
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

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "convert", "ok");
}

#[test]
fn detect_encoding_json_mode_is_clean_and_well_formed() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("utf8.srt");
    fs::write(&p, SAMPLE_SRT).unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "detect-encoding", p.to_str().unwrap()])
        .assert()
        .success();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "detect-encoding", "ok");
}

#[test]
fn config_get_json_mode_is_clean_and_well_formed() {
    let dir = TempDir::new().unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "get", "ai.provider"])
        .assert()
        .success();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "config", "ok");
}

#[test]
fn config_list_json_mode_is_clean_and_well_formed() {
    let dir = TempDir::new().unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "config", "list"])
        .assert()
        .success();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "config", "ok");
}

#[test]
fn translate_empty_inputs_json_mode_is_clean_and_well_formed() {
    // The empty-success branch covers `translate` without depending on
    // a mocked AI server.
    let dir = TempDir::new().unwrap();
    let empty = dir.path().join("empty");
    fs::create_dir(&empty).unwrap();

    let assert = isolated_cmd(dir.path())
        .args([
            "--output",
            "json",
            "translate",
            empty.to_str().unwrap(),
            "--target-language",
            "zh-TW",
            "--recursive",
        ])
        .assert()
        .success();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "translate", "ok");
}

#[test]
fn generate_completion_json_mode_emits_clean_error_envelope() {
    // generate-completion intentionally rejects JSON mode.
    let dir = TempDir::new().unwrap();

    let assert = isolated_cmd(dir.path())
        .args(["--output", "json", "generate-completion", "bash"])
        .assert()
        .failure();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "generate-completion", "error");
}

#[tokio::test]
async fn sync_manual_offset_json_mode_is_clean_and_well_formed() {
    let mut helper = CLITestHelper::new();
    let subtitle = helper
        .create_subtitle_file("manual.srt", SAMPLE_SRT)
        .await
        .expect("create subtitle");
    let output = helper.temp_dir_path().join("manual.synced.srt");

    let result = helper
        .run_command_with_config(&[
            "--output",
            "json",
            "sync",
            "--subtitle",
            subtitle.to_str().unwrap(),
            "--offset",
            "1.5",
            "--output",
            output.to_str().unwrap(),
            "--force",
        ])
        .await;

    assert!(
        result.success,
        "sync command failed: stdout={} stderr={}",
        result.stdout, result.stderr
    );

    let env = assert_json_stdout_clean(result.stdout.as_bytes());
    assert_envelope(&env, "sync", "ok");
}

#[tokio::test]
async fn match_dry_run_json_mode_is_clean_and_well_formed() {
    let dir = TempDir::new().unwrap();
    let video = dir.path().join("Movie.mp4");
    let subtitle = dir.path().join("random.srt");
    fs::write(&video, b"fake video bytes").unwrap();
    fs::write(&subtitle, SAMPLE_SRT).unwrap();

    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(dir.path(), false).unwrap();
    let video_id = files
        .iter()
        .find(|f| f.name.ends_with(".mp4"))
        .map(|f| f.id.clone())
        .expect("video discovered");
    let subtitle_id = files
        .iter()
        .find(|f| f.name.ends_with(".srt"))
        .map(|f| f.id.clone())
        .expect("subtitle discovered");

    let response_content = json!({
        "matches": [
            {
                "video_file_id": video_id,
                "subtitle_file_id": subtitle_id,
                "confidence": 0.95,
                "match_factors": ["filename_similarity"]
            }
        ],
        "confidence": 0.95,
        "reasoning": "Stable mock"
    })
    .to_string();

    let mock_server = MockServer::start().await;
    let body = json!({
        "choices": [
            { "message": { "content": response_content }, "finish_reason": "stop" }
        ],
        "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
        "model": "gpt-4.1-mini"
    });
    Mock::given(method("POST"))
        .and(wmpath("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let assert = isolated_cmd(dir.path())
        .env("SUBX_AI_PROVIDER", "local")
        .env("LOCAL_LLM_API_KEY", "sk-mock-test-key")
        .env("LOCAL_LLM_BASE_URL", mock_server.uri())
        .args([
            "--output",
            "json",
            "match",
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let env = assert_json_stdout_clean(&assert.get_output().stdout);
    assert_envelope(&env, "match", "ok");
}
