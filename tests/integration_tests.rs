//! The match → convert → sync workflow, driven through the compiled binary.
//!
//! Every leg spawns `subx-cli` from one isolated child environment built by
//! [`workflow_command`]: a temporary config file named by `SUBX_CONFIG_PATH`,
//! `XDG_CONFIG_HOME` inside the test `TempDir`, loopback proxy pinning and the
//! removal of every ambient AI-provider variable. The match leg takes its AI
//! endpoint from a wiremock server the test starts on loopback, so the workflow
//! needs no credentials, no user configuration and no outbound network.

mod common;

use assert_cmd::Command;
use predicates::prelude::*;
use regex::Regex;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Proxy overrides that could reroute the loopback call to the test's mock.
///
/// Kept apart from [`common::cli_helpers::ISOLATED_AI_ENV_VARS`] because they
/// change *routing*, not provider selection; `reqwest`'s default client reads
/// them and loopback is not exempt in every configuration. The `NO_PROXY` pins
/// below cover name-based matching; removing the overrides covers everything
/// else.
const AMBIENT_PROXY_ENV_VARS: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
];

/// Create test media and subtitle files.
fn create_test_media_files(dir: &Path) {
    // Create dummy video file
    let video_path = dir.join("video.mp4");
    fs::write(&video_path, b"").unwrap();
    // Create simple SRT subtitle file
    let srt_path = dir.join("video.srt");
    let srt_content = "\
1
00:00:00,000 --> 00:00:01,000
Hello
";
    fs::write(&srt_path, srt_content).unwrap();
}

/// Build the child command every workflow leg runs through.
///
/// The temporary config file named by `SUBX_CONFIG_PATH` is the child's only
/// AI source; `XDG_CONFIG_HOME` moves the match cache, the match journal and
/// the batch lock file into the test `TempDir`. A spawned binary necessarily
/// runs the production config loader — the temp file *is* the isolation, which
/// is why the in-process `TestConfigService` rule does not apply here.
fn workflow_command(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.current_dir(dir)
        .env("SUBX_CONFIG_PATH", dir.join("test_config.toml"))
        .env("XDG_CONFIG_HOME", dir.join("config-home"))
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .env_remove("SUBX_OUTPUT");
    for var in common::cli_helpers::ISOLATED_AI_ENV_VARS {
        cmd.env_remove(*var);
    }
    for var in AMBIENT_PROXY_ENV_VARS {
        cmd.env_remove(*var);
    }
    cmd
}

/// Collect the file IDs an inventory block of the match prompt carries.
///
/// The slice is bounded by `open`/`close`, so samples elsewhere in the prompt
/// cannot leak in, and the `<file id="` anchor keeps the capture free of the
/// `subtitle_file_id="…"` attribute `<content_samples>` uses. No prefix is
/// assumed: an ID-format change must not break the mock.
fn ids_in_block(prompt: &str, open: &str, close: &str) -> Vec<String> {
    let Some(start) = prompt.find(open) else {
        return Vec::new();
    };
    let rest = &prompt[start + open.len()..];
    let Some(end) = rest.find(close) else {
        return Vec::new();
    };
    let pattern = Regex::new(r#"<file id="([^"]+)""#).expect("static regex compiles");
    pattern
        .captures_iter(&rest[..end])
        .map(|capture| capture[1].to_string())
        .collect()
}

/// Answer a chat-completions request by pairing the IDs the prompt carries.
///
/// The responder never invents a pair: both inventories come from the request
/// body, so a change to ID generation or prompt order keeps the answer valid.
/// It panics — rather than answering short — when either block is empty or the
/// counts differ, so a prompt-schema regression fails loudly.
fn echo_match_response(request: &Request) -> ResponseTemplate {
    let body: Value =
        serde_json::from_slice(&request.body).expect("chat-completions request body is JSON");
    let prompt = body
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|messages| messages.last())
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let videos = ids_in_block(prompt, "<video_files>", "</video_files>");
    let subtitles = ids_in_block(prompt, "<subtitle_files>", "</subtitle_files>");
    assert!(
        !videos.is_empty() && !subtitles.is_empty(),
        "prompt inventories are empty: {} video id(s), {} subtitle id(s)",
        videos.len(),
        subtitles.len()
    );
    assert_eq!(
        videos.len(),
        subtitles.len(),
        "prompt inventories are unbalanced: {} video id(s), {} subtitle id(s)",
        videos.len(),
        subtitles.len()
    );

    let matches: Vec<Value> = videos
        .iter()
        .zip(subtitles.iter())
        .map(|(video_id, subtitle_id)| {
            json!({
                "video_file_id": video_id,
                "subtitle_file_id": subtitle_id,
                "confidence": 0.95,
                "match_factors": ["filename_similarity"],
                "language": "en",
            })
        })
        .collect();
    let analysis = json!({
        "matches": matches,
        "confidence": 0.95,
        "reasoning": "Echoed from request",
    });

    ResponseTemplate::new(200).set_body_json(json!({
        "choices": [
            { "message": { "content": analysis.to_string() }, "finish_reason": "stop" }
        ],
        "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
        "model": "mock-model",
    }))
}

/// Test match command's dry-run functionality.
fn test_match_command(dir: &Path) {
    let runtime = Runtime::new().expect("tokio runtime");
    let server = runtime.block_on(MockServer::start());
    runtime.block_on(
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(echo_match_response)
            .mount(&server),
    );

    // Written after the server exists: its loopback port is the config value,
    // and the `local` provider is the only one that accepts a plain `http://`
    // endpoint. An API-key-less `[ai]` section is valid for it, and the
    // provider appends `/chat/completions` to `base_url` verbatim.
    fs::write(
        dir.join("test_config.toml"),
        format!(
            "[ai]\nprovider = \"local\"\nmodel = \"mock-model\"\nbase_url = \"{}/v1\"\n",
            server.uri()
        ),
    )
    .expect("write the workflow test config");

    workflow_command(dir)
        .arg("match")
        .arg(dir)
        .arg("--dry-run")
        .assert()
        .success()
        // Characterised (B4 revival, task 6.3): the dry-run banner is the
        // capitalised "Preview mode" line printed by display_match_results.
        .stdout(predicate::str::contains("Preview mode"));

    let received = runtime
        .block_on(server.received_requests())
        .expect("wiremock records requests by default");
    let urls: Vec<&str> = received
        .iter()
        .map(|request| request.url.as_str())
        .collect();
    assert_eq!(
        received.len(),
        1,
        "expected exactly one AI request, observed: {urls:?}"
    );
    assert_eq!(received[0].url.path(), "/v1/chat/completions");
}

/// Test convert command to convert SRT to VTT.
fn test_convert_command(dir: &Path) {
    let subtitle = dir.join("video.srt");
    let output = dir.join("video.vtt");
    workflow_command(dir)
        .arg("convert")
        .arg(&subtitle)
        // Characterised (B4 revival, task 6.3): convert deletes the source
        // unless --keep-original is given, which would starve the sync leg
        // below of its subtitle input.
        .arg("--keep-original")
        .arg("--format")
        .arg("vtt")
        .assert()
        .success();
    // Check output file and content
    assert!(output.exists(), "Converted VTT file does not exist");
    assert!(
        subtitle.exists(),
        "--keep-original must preserve the source"
    );
    let content = fs::read_to_string(output).unwrap();
    assert!(
        content.contains("WEBVTT"),
        "Output does not contain WEBVTT header"
    );
}

/// Test sync command's manual offset functionality.
fn test_sync_command(dir: &Path) {
    let video = dir.join("video.mp4");
    let subtitle = dir.join("video.srt");
    workflow_command(dir)
        .arg("sync")
        .arg(&video)
        .arg(&subtitle)
        .arg("--offset")
        .arg("1")
        .assert()
        .success()
        // Characterised (B4 revival, task 6.3): the manual-offset run reports
        // through the success summary; "manual offset applied" was never
        // printed — the verbose line is "Using manual offset:".
        .stdout(predicate::str::contains("Sync completed: offset 1.000s"));
}

// @covers cross-crate-testing/The Full-Workflow Test Is Hermetic#The dry-run match runs against a loopback mock
#[test]
fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();
    create_test_media_files(dir);
    test_match_command(dir);
    test_convert_command(dir);
    test_sync_command(dir);
}

// @covers cross-crate-testing/The Full-Workflow Test Is Hermetic#An ambient override cannot reach any leg
#[test]
fn workflow_children_strip_ambient_ai_and_proxy_overrides() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();
    let cmd = workflow_command(dir);
    let child_env: std::collections::HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|value| value.to_string_lossy().into_owned()),
            )
        })
        .collect();

    // Pinned count: a name silently dropped from the shared list would leave
    // the list shorter, and the ambient variable it forgot would reach the
    // child again.
    assert_eq!(
        common::cli_helpers::ISOLATED_AI_ENV_VARS.len(),
        13,
        "the shared isolation list lost or gained a name"
    );
    // The names the requirement calls out by hand, plus every name the shared
    // list carries: each must be *removed*, not merely unset.
    let named_by_the_requirement = [
        "OPENAI_API_KEY",
        "SUBX_AI_BASE_URL",
        "SUBX_OUTPUT",
        "LOCAL_LLM_BASE_URL",
    ];
    for var in named_by_the_requirement
        .iter()
        .copied()
        .chain(common::cli_helpers::ISOLATED_AI_ENV_VARS.iter().copied())
        .chain(AMBIENT_PROXY_ENV_VARS.iter().copied())
    {
        assert_eq!(
            child_env.get(var),
            Some(&None),
            "{var} must be removed from the child environment, not inherited"
        );
    }
    // The temporary file, not the developer's config, is the only AI source.
    assert_eq!(
        child_env.get("SUBX_CONFIG_PATH"),
        Some(&Some(
            dir.join("test_config.toml").to_string_lossy().into_owned()
        ))
    );
}

// @covers cross-crate-testing/The Full-Workflow Test Is Hermetic#A developer config file cannot fail the non-AI legs
#[test]
fn workflow_children_resolve_the_config_named_by_subx_config_path() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();
    let config_path = dir.join("test_config.toml");
    // The child reports where it writes: the temp file, never the developer's
    // user config. This is what lets convert and sync succeed in
    // `test_full_workflow` against a machine whose own config is absent or
    // unusable.
    workflow_command(dir)
        .args(["config", "set", "ai.provider", "local"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            config_path.to_string_lossy().to_string(),
        ));
    // And the value it just wrote is what a later child reads back — the same
    // path, not a fallback.
    workflow_command(dir)
        .args(["config", "get", "ai.provider"])
        .assert()
        .success()
        .stdout(predicate::str::contains("local"));
}

// @covers cross-crate-testing/The Full-Workflow Test Is Hermetic#A mock response survives a change to file IDs
#[test]
fn mock_ids_are_read_from_the_prompt_blocks() {
    // Deliberately unlike the UUIDv7 IDs the matcher generates today: the
    // extractor must not depend on `file_<uuid>` formatting. The decoy sample
    // carries an ID through `subtitle_file_id="…"`, which the `<file id="`
    // anchor excludes, and the two blocks are ordered so an unanchored scan of
    // the whole prompt would pair the wrong IDs.
    let prompt = concat!(
        "<video_files>\n",
        "  <file id=\"alpha-1\" name=\"a.mp4\" path=\"a.mp4\"/>\n",
        "</video_files>\n",
        "<subtitle_files>\n",
        "  <file id=\"beta-1\" name=\"a.srt\" path=\"a.srt\"/>\n",
        "</subtitle_files>\n",
        "<content_samples>\n",
        "  <sample subtitle_file_id=\"decoy-1\"><![CDATA[Hello]]></sample>\n",
        "</content_samples>\n",
    );
    assert_eq!(
        ids_in_block(prompt, "<video_files>", "</video_files>"),
        vec!["alpha-1".to_string()]
    );
    assert_eq!(
        ids_in_block(prompt, "<subtitle_files>", "</subtitle_files>"),
        vec!["beta-1".to_string()]
    );
    assert!(
        ids_in_block(prompt, "<missing_block>", "</missing_block>").is_empty(),
        "an absent block yields no IDs"
    );
}
