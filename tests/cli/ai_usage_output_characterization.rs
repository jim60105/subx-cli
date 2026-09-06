//! Characterisation tests for the `🤖 AI API Call Details:` usage block
//! emitted after an AI API call (tasks A1 §1.4).
//!
//! Written against **unmodified** source (before the `Reporter` seam is
//! wired in) to pin today's observable behaviour: in `text` mode the
//! four-line block plus trailing blank line lands on **stdout**; in
//! `--output json` mode it appears on **neither** stream. These tests
//! must pass unchanged after the rewiring in phases 4–6 — a diff here
//! is a rewiring bug, not a test to update.
//!
//! Wired into the test crate via `tests/ai_usage_output_characterization_tests.rs`
//! (Cargo does not auto-discover `tests/cli/`).

use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n";

/// Response carrying an explicit usage object with distinctive counts so
/// the characterisation asserts pin the exact rendered numbers.
async fn mock_server_with_usage() -> MockServer {
    let mock_server = MockServer::start().await;
    let content = json!({
        "matches": [],
        "confidence": 0.5,
        "reasoning": "no matches",
    })
    .to_string();
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                { "message": { "content": content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 },
            "model": "gpt-4.1-mini"
        })))
        .mount(&mock_server)
        .await;
    mock_server
}

fn build_pair_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Movie.mp4"), b"fake video bytes").unwrap();
    fs::write(dir.path().join("random.srt"), SAMPLE_SRT).unwrap();
    dir
}

fn run_match(
    base_url: &str,
    workdir: &std::path::Path,
    json_mode: bool,
) -> assert_cmd::assert::Assert {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
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
        .args(["--dry-run", "--confidence", "80", workdir.to_str().unwrap()])
        .current_dir(workdir)
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

const USAGE_MARKERS: &[&str] = &[
    "🤖 AI API Call Details:",
    "   Model: gpt-4.1-mini",
    "   Prompt tokens: 10",
    "   Completion tokens: 5",
    "   Total tokens: 15",
];

/// A1 §1.4 — text mode: the usage block lands on stdout, byte-exact.
#[tokio::test]
async fn text_mode_prints_ai_usage_block_on_stdout() {
    let dir = build_pair_dir();
    let server = mock_server_with_usage().await;

    let assert = run_match(&server.uri(), dir.path(), false).success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    assert!(
        stdout.contains(
            "🤖 AI API Call Details:\n   Model: gpt-4.1-mini\n   Prompt tokens: 10\n   Completion tokens: 5\n   Total tokens: 15\n\n"
        ),
        "text-mode stdout missing the exact usage block:\n{stdout}"
    );
    assert!(
        !stderr.contains("🤖 AI API Call Details:"),
        "usage block leaked to stderr:\n{stderr}"
    );
}

/// A1 §1.4 — JSON mode: the usage block appears on neither stream.
#[tokio::test]
async fn json_mode_prints_no_ai_usage_block() {
    let dir = build_pair_dir();
    let server = mock_server_with_usage().await;

    let assert = run_match(&server.uri(), dir.path(), true).success();
    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

    for marker in USAGE_MARKERS {
        assert!(
            !stdout.contains(marker),
            "JSON-mode stdout leaked usage marker {marker:?}:\n{stdout}"
        );
        assert!(
            !stderr.contains(marker),
            "JSON-mode stderr leaked usage marker {marker:?}:\n{stderr}"
        );
    }
}
