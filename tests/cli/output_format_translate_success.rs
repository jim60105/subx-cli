//! Mock-driven success path for `subx-cli --output json translate ...`.
//!
//! Per `openspec/changes/add-machine-readable-output/tasks.md` §1.7, this
//! test guards the "exactly one JSON document on stdout" contract for
//! `translate` against regression by rubber-duck-flagged stdout-corrupting
//! progress prints in `core::translation::engine` (e.g.
//! `log_translation_progress`). It spawns the compiled binary with a
//! wiremock-backed AI endpoint, runs a real translation through the
//! engine, and asserts via [`assert_json_stdout_clean`] that stdout is
//! exactly one JSON envelope with the expected `translate` payload
//! shape.
//!
//! Wired into the test crate via `tests/output_format_translate_success_tests.rs`.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::common::json_output::assert_json_stdout_clean;

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n2\n00:00:03,000 --> 00:00:04,000\nGoodbye world\n\n";

/// Mock responder that mirrors the dynamic responder used by
/// `tests/translate_command_integration_tests.rs`: it inspects the user
/// prompt, returns terminology JSON for the first pass and a per-cue
/// translation JSON for the second pass.
struct DynamicTranslationResponder;

impl Respond for DynamicTranslationResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: serde_json::Value =
            serde_json::from_slice(&request.body).expect("translation request body should be JSON");
        let prompt = body["messages"]
            .as_array()
            .and_then(|messages| messages.last())
            .and_then(|message| message["content"].as_str())
            .unwrap_or("");

        let content = if prompt.contains("Cues to translate:") {
            let translations: Vec<serde_json::Value> = prompt
                .lines()
                .filter_map(|line| line.trim().strip_prefix("- id: "))
                .enumerate()
                .map(|(idx, id)| {
                    json!({
                        "id": id.trim(),
                        "text": format!("譯文第{}句", idx + 1),
                    })
                })
                .collect();
            json!({ "translations": translations }).to_string()
        } else {
            // Terminology pass: no domain terms detected.
            json!({ "terms": [] }).to_string()
        };

        ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                {
                    "message": { "content": content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
        }))
    }
}

async fn mock_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(DynamicTranslationResponder)
        .mount(&server)
        .await;
    server
}

/// Drive the compiled binary with the mock AI endpoint and JSON output
/// mode, returning the captured `Assert`.
fn run_translate(
    base_url: &str,
    extra_args: &[&str],
    workdir: &std::path::Path,
) -> assert_cmd::assert::Assert {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_AI_PROVIDER", "local")
        .env("LOCAL_LLM_API_KEY", "sk-mock-test-key")
        .env("LOCAL_LLM_BASE_URL", base_url)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env_remove("SUBX_OUTPUT")
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .args(["--output", "json", "translate"])
        .args(extra_args)
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

fn assert_envelope_shape(env: &Value, command: &str, status: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], command);
    assert_eq!(env["status"], status);
}

/// Successful translation in JSON mode emits exactly one envelope on
/// stdout — the `core::translation::engine` progress chatter
/// (`log_translation_progress`) MUST be routed to stderr, not stdout.
#[tokio::test]
async fn translate_success_emits_single_clean_envelope() {
    let server = mock_server().await;

    let dir = TempDir::new().unwrap();
    let input = dir.path().join("movie.srt");
    fs::write(&input, SAMPLE_SRT).unwrap();
    let expected_output = dir.path().join("movie.zh-TW.srt");

    let assert = run_translate(
        &server.uri(),
        &[
            input.to_str().unwrap(),
            "--target-language",
            "zh-TW",
            "--source-language",
            "en",
        ],
        dir.path(),
    )
    .success();

    let stdout = assert.get_output().stdout.clone();
    // Discipline: exactly one JSON document, no ANSI, no progress bars,
    // no `\r` redraws — the rubber-duck-flagged regression.
    let env = assert_json_stdout_clean(&stdout);

    assert_envelope_shape(&env, "translate", "ok");
    assert!(env.get("error").is_none(), "success envelope has no error");

    let translated = env["data"]["translated_files"]
        .as_array()
        .expect("translated_files array");
    assert_eq!(translated.len(), 1, "exactly one translated file");
    let item = &translated[0];
    assert_eq!(item["applied"], true);
    assert_eq!(item["input"], input.display().to_string());
    assert_eq!(item["output"], expected_output.display().to_string());

    // The translated file should exist on disk with the mock response.
    let body = fs::read_to_string(&expected_output).expect("translated output file");
    assert!(
        body.contains("譯文第1句"),
        "first translation present: {body}"
    );
    assert!(
        body.contains("譯文第2句"),
        "second translation present: {body}"
    );
}
