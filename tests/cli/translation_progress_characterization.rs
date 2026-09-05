//! Characterisation tests for the `📊 Translation Progress:` block emitted
//! by `TranslationEngine` during a real AI translation run (tasks A1 §1.5).
//!
//! Written against **unmodified** source (before the `Reporter` seam is
//! wired in). Pins today's observable behaviour: in `text` mode the block
//! lands on **stderr**; under `--quiet` and under `--output json` it
//! appears on neither stream. Must pass unchanged after the rewiring.
//!
//! Scope note (tasks §1.5): the `FileManager::rollback` warning and the
//! `WorkerPool::shutdown` progress message are deliberately NOT
//! characterised end-to-end — no command path reaches them (rollback
//! requires an execution failure mid-pipeline; `WorkerPool::shutdown` has
//! no command caller). They are covered post-refactor by direct
//! recording-reporter unit tests (tasks §6.2) plus the stream-matrix unit
//! tests (tasks §7.3).
//!
//! Wired into the test crate via `tests/translation_progress_characterization_tests.rs`
//! (Cargo does not discover `tests/cli/`).

use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const SAMPLE_SRT: &str = "1
00:00:01,000 --> 00:00:02,000
Hello world

2
00:00:03,000 --> 00:00:04,000
Second cue
";

/// Two-pass responder mirroring `tests/cli/output_format_translate_success.rs`:
/// terminology JSON for the first pass, per-cue translations for the second.
struct TranslationResponder;

impl Respond for TranslationResponder {
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
            json!({ "terms": [] }).to_string()
        };

        ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                { "message": { "content": content }, "finish_reason": "stop" }
            ],
            "model": "gpt-4.1-mini"
        }))
    }
}

async fn mock_translation_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(TranslationResponder)
        .expect(1..)
        .mount(&server)
        .await;
    server
}

fn run_translate(
    base_url: &str,
    workdir: &std::path::Path,
    global_args: &[&str],
) -> assert_cmd::assert::Assert {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let input = workdir.join("movie.en.srt");
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("SUBX_AI_PROVIDER", "local")
        .env("LOCAL_LLM_API_KEY", "sk-mock-test-key")
        .env("LOCAL_LLM_BASE_URL", base_url)
        .env("SUBX_TRANSLATION_BATCH_SIZE", "2")
        .env("XDG_CONFIG_HOME", &xdg)
        .env("HOME", workdir)
        .args(global_args)
        .arg("translate")
        .arg(input.as_os_str().to_str().unwrap())
        .args(["--target-language", "zh-TW", "--source-language", "en"])
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false")
        .timeout(std::time::Duration::from_secs(60));
    cmd.assert()
}

fn build_workdir() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("movie.en.srt"), SAMPLE_SRT).unwrap();
    dir
}

/// A1 §1.5 — text mode: the progress block lands on stderr.
#[tokio::test]
async fn text_mode_prints_translation_progress_on_stderr() {
    let dir = build_workdir();
    let server = mock_translation_server().await;

    let assert = run_translate(&server.uri(), dir.path(), &[]).success();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    assert!(
        stderr.contains("📊 Translation Progress:\n   Processed cues:"),
        "text-mode stderr missing the progress block:\n{stderr}"
    );
    assert!(
        !stdout.contains("📊 Translation Progress:"),
        "progress block leaked to stdout:\n{stdout}"
    );
}

/// A1 §1.5 — `--quiet` silences the progress block on both streams.
#[tokio::test]
async fn quiet_mode_prints_no_translation_progress() {
    let dir = build_workdir();
    let server = mock_translation_server().await;

    let assert = run_translate(&server.uri(), dir.path(), &["--quiet"]).success();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    assert!(
        !stderr.contains("📊 Translation Progress:")
            && !stdout.contains("📊 Translation Progress:"),
        "--quiet must suppress the progress block:\nstdout:{stdout}\nstderr:{stderr}"
    );
}

/// A1 §1.5 — `--output json` silences the progress block on both streams.
#[tokio::test]
async fn json_mode_prints_no_translation_progress() {
    let dir = build_workdir();
    let server = mock_translation_server().await;

    let assert = run_translate(&server.uri(), dir.path(), &["--output", "json"]).success();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();

    assert!(
        !stderr.contains("📊 Translation Progress:")
            && !stdout.contains("📊 Translation Progress:"),
        "--output json must suppress the progress block:\nstdout:{stdout}\nstderr:{stderr}"
    );
}
