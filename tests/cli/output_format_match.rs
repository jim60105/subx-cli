//! Integration tests for `subx-cli --output json match ...`.
//!
//! These tests spawn the compiled binary (via `assert_cmd`) and assert
//! that stdout contains exactly one JSON envelope conforming to the
//! `machine-readable-output` and `subtitle-matching` specs.
//!
//! Wired into the test crate via `tests/output_format_match_tests.rs`.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

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

fn assert_envelope_shape(env: &Value, command: &str, status: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], command);
    assert_eq!(env["status"], status);
}

/// Build a sample directory with one video + one subtitle.
///
/// IDs are no longer pre-derived because the matcher now generates fresh
/// UUIDv7 file IDs on every scan; tests use [`mock_server_with_echo_match`]
/// to echo whichever IDs the matcher actually emits.
fn build_sample_pair() -> TempDir {
    let dir = TempDir::new().unwrap();
    let video = dir.path().join("Movie.mp4");
    let subtitle = dir.path().join("random.srt");
    fs::write(&video, b"fake video bytes").unwrap();
    fs::write(&subtitle, SAMPLE_SRT).unwrap();
    dir
}

/// Wiremock responder that synthesises a single-pair `MatchResult` whose
/// `video_file_id` and `subtitle_file_id` are taken from the captured
/// request prompt. Confidence and reasoning are caller-provided so tests
/// can exercise both above- and below-threshold scenarios.
struct EchoMatchResponder {
    confidence: f64,
    reasoning: &'static str,
    factor: &'static str,
}

impl Respond for EchoMatchResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value =
            serde_json::from_slice(&request.body).expect("OpenAI request body must be valid JSON");
        let prompt = body
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.last())
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let id_pattern = regex::Regex::new(
            r"file_[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}",
        )
        .expect("static regex compiles");
        let ids: Vec<String> = id_pattern
            .find_iter(prompt)
            .map(|m| m.as_str().to_string())
            .collect();
        assert!(
            ids.len() >= 2,
            "expected at least one video and one subtitle ID in prompt, got {}",
            ids.len()
        );
        let content = json!({
            "matches": [
                {
                    "video_file_id": ids[0],
                    "subtitle_file_id": ids[1],
                    "confidence": self.confidence,
                    "match_factors": [self.factor],
                }
            ],
            "confidence": self.confidence,
            "reasoning": self.reasoning,
        })
        .to_string();
        let response_body = json!({
            "choices": [
                { "message": { "content": content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
        });
        ResponseTemplate::new(200).set_body_json(response_body)
    }
}

async fn mock_server_with_echo_match(
    confidence: f64,
    reasoning: &'static str,
    factor: &'static str,
) -> MockServer {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(EchoMatchResponder {
            confidence,
            reasoning,
            factor,
        })
        .mount(&mock_server)
        .await;
    mock_server
}

fn run_match(
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
        // Isolate config so the user's real ~/.config/subx is not touched.
        .env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        // Disable progress bars defensively.
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .args(["--output", "json", "match"])
        .args(extra_args)
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

#[tokio::test]
async fn match_dry_run_emits_envelope() {
    let dir = build_sample_pair();
    let server = mock_server_with_echo_match(0.95, "Stable mock", "filename_similarity").await;

    let assert = run_match(
        &server.uri(),
        &[
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ],
        dir.path(),
    )
    .success();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "match", "ok");

    let data = &env["data"];
    assert_eq!(data["dry_run"], true);
    assert_eq!(data["confidence_threshold"], 80);
    let candidates = data["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["accepted"], true);
    assert_eq!(candidates[0]["confidence"], 95);
    let ops = data["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0]["status"], "ok");
    assert_eq!(ops[0]["applied"], false);
    assert_eq!(ops[0]["kind"], "rename");
    let summary = &data["summary"];
    assert_eq!(summary["total_candidates"], 1);
    assert_eq!(summary["accepted"], 1);
    assert_eq!(summary["applied"], 0);
    assert_eq!(summary["skipped"], 0);
    assert_eq!(summary["failed"], 0);
}

#[tokio::test]
async fn match_live_run_emits_envelope_with_applied_true() {
    let dir = build_sample_pair();
    let server = mock_server_with_echo_match(0.95, "Stable mock", "filename_similarity").await;

    let assert = run_match(
        &server.uri(),
        &["--confidence", "80", dir.path().to_str().unwrap()],
        dir.path(),
    )
    .success();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "match", "ok");

    let data = &env["data"];
    assert_eq!(data["dry_run"], false);
    let ops = data["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0]["applied"], true);
    assert_eq!(ops[0]["status"], "ok");
    let summary = &data["summary"];
    assert_eq!(summary["applied"], 1);
    assert_eq!(summary["failed"], 0);

    // Verify the rename actually happened on disk.
    let renamed = dir.path().join("Movie.srt");
    assert!(renamed.exists(), "subtitle should have been renamed");
}

#[tokio::test]
async fn match_subthreshold_candidate_is_rejected() {
    let dir = build_sample_pair();
    let server = mock_server_with_echo_match(0.50, "Stable mock low", "weak_signal").await;

    let assert = run_match(
        &server.uri(),
        &[
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ],
        dir.path(),
    )
    .success();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "match", "ok");

    let data = &env["data"];
    let candidates = data["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0]["accepted"], false);
    assert_eq!(candidates[0]["reason"], "below_threshold");
    assert_eq!(candidates[0]["confidence"], 50);
    let ops = data["operations"].as_array().unwrap();
    assert!(ops.is_empty(), "no operations expected for sub-threshold");
    let summary = &data["summary"];
    assert_eq!(summary["total_candidates"], 1);
    assert_eq!(summary["accepted"], 0);
    assert_eq!(summary["skipped"], 1);
    assert_eq!(summary["applied"], 0);
}

#[tokio::test]
async fn match_ai_failure_emits_error_envelope() {
    let (dir, _vid_id, _sub_id) = (build_sample_pair(), (), ());

    // Mock server returns 500 for the chat completion endpoint.
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&mock_server)
        .await;

    let assert = run_match(
        &mock_server.uri(),
        &[
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ],
        dir.path(),
    )
    .failure();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "match", "error");

    let err = &env["error"];
    assert_eq!(err["category"], "ai_service");
    assert_eq!(err["code"], "E_AI_SERVICE");
    assert!(err["message"].is_string());
    assert!(err["exit_code"].is_number());
}
