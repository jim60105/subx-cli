//! Smoke tests for `subx-cli --output json translate ...`.
//!
//! Per `openspec/changes/add-machine-readable-output/tasks.md` §10.3,
//! this file ensures the `translate` command produces a valid envelope
//! on the success-empty path and a uniform error envelope on failure.
//! Full mock-driven AI translation success is intentionally out of
//! scope here — the engine wiring is exercised by unit tests.
//!
//! Wired into the test crate via `tests/output_format_translate_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n";

fn parse_single_envelope(stdout: &[u8]) -> Value {
    assert!(!stdout.is_empty(), "stdout was empty");
    assert!(
        stdout.ends_with(b"\n"),
        "stdout did not end with newline: {:?}",
        String::from_utf8_lossy(stdout)
    );
    assert!(
        !stdout.contains(&0x1b),
        "stdout contained ANSI escape sequence"
    );
    let body = &stdout[..stdout.len() - 1];
    assert!(
        !body.contains(&b'\n'),
        "stdout contained more than one line: {:?}",
        String::from_utf8_lossy(body)
    );
    serde_json::from_slice(body).expect("stdout parses as JSON")
}

fn isolated_env(cmd: &mut Command, workdir: &std::path::Path) {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
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
}

/// Empty `--target-language` triggers `TranslateArgs::validate()`,
/// which returns `SubXError::CommandExecution`. With `--output json`
/// active, `main.rs` renders the uniform error envelope on stdout.
#[test]
fn translate_validation_failure_emits_error_envelope() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.srt");
    fs::write(&input, SAMPLE_SRT).unwrap();

    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    isolated_env(&mut cmd, dir.path());
    let assert = cmd
        .args([
            "--output",
            "json",
            "translate",
            input.to_str().unwrap(),
            "--target-language",
            "   ",
        ])
        .assert()
        .failure();

    let output = assert.get_output();
    let env = parse_single_envelope(&output.stdout);
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], "translate");
    assert_eq!(env["status"], "error");
    assert!(env.get("data").is_none());

    let err = env["error"].as_object().expect("error object");
    assert_eq!(err["category"], "command_execution");
    assert!(err.get("code").is_some());
    assert!(err["message"].as_str().unwrap().contains("target-language"));
}

/// When no inputs match (after globbing through the input handler) and
/// JSON mode is active, the command emits a success envelope with an
/// empty `translated_files` array. This covers the success/empty
/// branch of the envelope contract without needing a mocked AI server.
#[test]
fn translate_no_inputs_emits_empty_success_envelope() {
    let dir = TempDir::new().unwrap();
    // Create an empty subdirectory and pass it as a recursive input —
    // the input handler returns an empty collection, which the command
    // treats as a success-with-no-work case.
    let empty = dir.path().join("empty");
    fs::create_dir(&empty).unwrap();

    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    isolated_env(&mut cmd, dir.path());
    let assert = cmd
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

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], "translate");
    assert_eq!(env["status"], "ok");
    assert!(env.get("error").is_none());
    let translated = env["data"]["translated_files"]
        .as_array()
        .expect("translated_files array");
    assert!(translated.is_empty(), "expected empty translated_files");
}
