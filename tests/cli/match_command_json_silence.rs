//! Regression tests for `tasks.md` §7: JSON-mode stderr silence on the
//! `match` command.
//!
//! These tests spawn the compiled `subx-cli` binary so they exercise
//! the same stderr discipline a real user would observe, including
//! warnings emitted from `resolve_filename_conflict` and the AI
//! analysis result block.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n";

/// A wiremock responder that synthesises a multi-pair AI match response
/// using whichever UUIDv7 file IDs appear in the captured prompt.
///
/// `pairs` is the maximum number of (video, subtitle) pairs to emit;
/// the responder pairs the first N video IDs with the next N subtitle
/// IDs in prompt order (which mirrors how the matcher emits its blocks).
struct EchoMatchesResponder {
    pairs: usize,
    confidence: f64,
}

impl Respond for EchoMatchesResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&request.body).expect("valid JSON request");
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

        let total = ids.len();
        let videos = total / 2;
        let take = self.pairs.min(videos).min(total - videos);
        let matches: Vec<Value> = (0..take)
            .map(|i| {
                json!({
                    "video_file_id": ids[i],
                    "subtitle_file_id": ids[videos + i],
                    "confidence": self.confidence,
                    "match_factors": ["filename_similarity"],
                })
            })
            .collect();
        let content = json!({
            "matches": matches,
            "confidence": self.confidence,
            "reasoning": "Echoed multi-pair match",
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

async fn mock_server_with_echo(pairs: usize, confidence: f64) -> MockServer {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(EchoMatchesResponder { pairs, confidence })
        .mount(&mock_server)
        .await;
    mock_server
}

fn build_pair_dir(video_name: &str, subtitle_name: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(video_name), b"fake video bytes").unwrap();
    fs::write(dir.path().join(subtitle_name), SAMPLE_SRT).unwrap();
    dir
}

fn run_match(
    base_url: &str,
    workdir: &std::path::Path,
    extra_args: &[&str],
    json_mode: bool,
) -> assert_cmd::assert::Assert {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_AI_PROVIDER", "local")
        .env("LOCAL_LLM_API_KEY", "sk-mock-test-key")
        .env("LOCAL_LLM_BASE_URL", base_url)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false");
    if json_mode {
        cmd.args(["--output", "json"]);
    }
    cmd.arg("match")
        .args(extra_args)
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

const FORBIDDEN_STDERR_MARKERS: &[&str] = &[
    "🔍",
    "Total matches:",
    "Preview:",
    "Warning: Skipping relocation",
    "Warning: Conflict resolution prompt not implemented",
];

fn assert_stderr_clean_in_json_mode(stderr: &[u8]) {
    let s = String::from_utf8_lossy(stderr);
    for marker in FORBIDDEN_STDERR_MARKERS {
        assert!(
            !s.contains(marker),
            "JSON-mode stderr leaked forbidden marker {marker:?}: {s}"
        );
    }
    for line in s.lines() {
        assert!(
            !line.starts_with("   - file_"),
            "JSON-mode stderr leaked AI candidate file line {line:?}"
        );
    }
}

/// 7.1 — `--output json --dry-run` keeps stderr free of all the
/// matcher's free-form chatter while still emitting a clean envelope on
/// stdout.
#[tokio::test]
async fn json_mode_dry_run_emits_no_stderr_chatter() {
    let dir = build_pair_dir("Movie.mp4", "random.srt");
    let server = mock_server_with_echo(1, 0.95).await;

    let assert = run_match(
        &server.uri(),
        dir.path(),
        &[
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ],
        true,
    )
    .success();

    let output = assert.get_output();
    let env: Value =
        serde_json::from_slice(output.stdout.trim_ascii_end()).expect("stdout is JSON envelope");
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], "match");
    assert_eq!(env["status"], "ok");
    assert_stderr_clean_in_json_mode(&output.stderr);
}

/// 7.2 — Without `--output json`, the same flow still prints the
/// `🔍 AI Analysis Results:` block on stderr (proves the gate is
/// conditional, not a deletion).
#[tokio::test]
async fn human_mode_dry_run_still_prints_ai_analysis_results() {
    let dir = build_pair_dir("Movie.mp4", "random.srt");
    let server = mock_server_with_echo(1, 0.95).await;

    let assert = run_match(
        &server.uri(),
        dir.path(),
        &[
            "--dry-run",
            "--confidence",
            "80",
            dir.path().to_str().unwrap(),
        ],
        false,
    )
    .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    assert!(
        stderr.contains("🔍 AI Analysis Results:"),
        "human-mode stderr unexpectedly missing AI analysis block: {stderr}"
    );
}

/// 7.3 — Live (non-dry-run) path with a target file already present so
/// `ConflictResolution::Skip` fires; JSON mode must still suppress
/// `Warning: Skipping relocation`.
#[tokio::test]
async fn json_mode_live_run_suppresses_conflict_skip_warning() {
    let dir = build_pair_dir("Movie.mp4", "random.srt");
    // Pre-create the target name so the rename collides and the matcher
    // takes the Skip branch in `resolve_filename_conflict`.
    fs::write(dir.path().join("Movie.srt"), b"existing\n").unwrap();
    let server = mock_server_with_echo(1, 0.95).await;

    let assert = run_match(
        &server.uri(),
        dir.path(),
        &["--confidence", "80", dir.path().to_str().unwrap()],
        true,
    );
    let output = assert.get_output().clone();
    // The command may exit ok or fail depending on conflict policy; we
    // only care about stderr discipline here.
    assert_stderr_clean_in_json_mode(&output.stderr);
}
