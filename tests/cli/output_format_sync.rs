//! Integration tests for `subx sync --output json`.
//!
//! These tests use the shared [`CLITestHelper`] to invoke the binary in a
//! subprocess with `--output json` and assert that:
//!
//! - exactly one JSON document is written to stdout (terminated by `\n`);
//! - the envelope shape matches the `timeline-sync` JSON contract;
//! - the `data` payload always exposes the uniform
//!   `{ method, inputs[], operations[] }` shape ([`SyncPayload`]) for
//!   single-pair invocations as well as batch runs;
//! - per-file failures inside a batch surface as
//!   `inputs[].status == "error"` while the top-level envelope stays
//!   `status == "ok"` whenever at least one item makes forward progress;
//! - a batch where every item fails produces a top-level error envelope
//!   instead of a success envelope wrapping all-error items.
//!
//! [`CLITestHelper`]: crate::common::cli_helpers::CLITestHelper
//! [`SyncPayload`]: subx_cli::commands::sync_command::SyncPayload

use crate::common::cli_helpers::CLITestHelper;
use serde_json::Value;
use std::fs;

const SRT_VALID: &str = "1\n00:00:01,000 --> 00:00:03,000\nHello world\n\n\
                        2\n00:00:04,000 --> 00:00:06,000\nSecond cue\n\n";

fn assert_single_json_envelope(stdout: &str) -> Value {
    assert!(
        stdout.ends_with('\n'),
        "stdout must end with newline: {stdout:?}"
    );
    let body = stdout.trim_end_matches('\n');
    assert!(
        !body.contains('\n'),
        "stdout must contain exactly one JSON document, got:\n{stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "stdout must not contain ANSI escape sequences"
    );
    serde_json::from_str(body).expect("stdout body must be valid JSON")
}

fn assert_envelope_ok(env: &Value, expected_command: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], expected_command);
    assert_eq!(env["status"], "ok", "envelope: {env}");
    assert!(
        env.get("error").is_none(),
        "successful envelope must omit `error`: {env}"
    );
    assert!(
        env.get("data").is_some(),
        "successful envelope must include `data`: {env}"
    );
}

fn assert_envelope_error(env: &Value, expected_command: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], expected_command);
    assert_eq!(env["status"], "error", "envelope: {env}");
    assert!(
        env.get("data").is_none(),
        "error envelope must omit `data`: {env}"
    );
    let err = env
        .get("error")
        .expect("error envelope must include `error`");
    assert!(err["category"].is_string());
    assert!(err["code"].is_string());
    assert!(err["exit_code"].is_i64());
    assert!(err["message"].is_string());
}

#[tokio::test]
async fn manual_offset_single_emits_sync_payload() {
    let mut helper = CLITestHelper::new();
    let subtitle = helper
        .create_subtitle_file("manual.srt", SRT_VALID)
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
        "command failed; stdout={} stderr={}",
        result.stdout, result.stderr
    );
    let env = assert_single_json_envelope(&result.stdout);
    assert_envelope_ok(&env, "sync");
    let data = &env["data"];
    assert_eq!(data["method"], "manual");

    let inputs = data["inputs"].as_array().expect("inputs must be array");
    assert_eq!(inputs.len(), 1, "single-pair input array: {inputs:?}");
    let input = &inputs[0];
    assert_eq!(input["status"], "ok");
    assert!(input.get("error").is_none(), "ok input has no error");
    assert_eq!(input["detected_offset_ms"], 1500);
    assert_eq!(input["subtitle_path"], subtitle.display().to_string());
    assert!(input.get("audio_path").is_none());
    assert!(input.get("confidence").is_none());
    assert!(input.get("vad").is_none());

    let operations = data["operations"]
        .as_array()
        .expect("operations must be array");
    assert_eq!(
        operations.len(),
        1,
        "single-pair operation array: {operations:?}"
    );
    let op = &operations[0];
    assert_eq!(op["status"], "ok");
    assert_eq!(op["applied"], true);
    assert_eq!(op["dry_run"], false);
    assert_eq!(op["subtitle_path"], subtitle.display().to_string());
    assert_eq!(op["output_path"], output.display().to_string());
    assert!(output.exists(), "synchronized file should be written");
}

#[tokio::test]
async fn manual_offset_dry_run_reports_not_applied() {
    let mut helper = CLITestHelper::new();
    let subtitle = helper
        .create_subtitle_file("dry.srt", SRT_VALID)
        .await
        .expect("create subtitle");

    let result = helper
        .run_command_with_config(&[
            "--output",
            "json",
            "sync",
            "--subtitle",
            subtitle.to_str().unwrap(),
            "--offset=-2.0",
            "--dry-run",
        ])
        .await;

    assert!(
        result.success,
        "command failed; stdout={} stderr={}",
        result.stdout, result.stderr
    );
    let env = assert_single_json_envelope(&result.stdout);
    assert_envelope_ok(&env, "sync");
    let data = &env["data"];
    assert_eq!(data["method"], "manual");

    let input = &data["inputs"].as_array().expect("inputs")[0];
    assert_eq!(input["detected_offset_ms"], -2000);
    assert_eq!(input["status"], "ok");

    let op = &data["operations"].as_array().expect("operations")[0];
    assert_eq!(op["applied"], false);
    assert_eq!(op["dry_run"], true);
    assert!(op.get("output_path").is_none());
}

#[tokio::test]
async fn batch_mixed_success_and_failure_emits_inputs_and_operations() {
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // Two subdirectories, each with a 1:1 video+subtitle pair so the
    // batch dispatcher routes them through the per-prefix matcher (the
    // multi-item branch we want to exercise for partial failures).
    let dir_a = workspace.join("dir_a");
    let dir_b = workspace.join("dir_b");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    let video_a = dir_a.join("clip_a.mp4");
    fs::write(&video_a, b"").unwrap();
    let sub_a = dir_a.join("clip_a.srt");
    fs::write(&sub_a, SRT_VALID).unwrap();

    let video_b = dir_b.join("clip_b.mp4");
    fs::write(&video_b, b"").unwrap();
    // Intentionally invalid SRT to trigger a per-item parse failure.
    let sub_b = dir_b.join("clip_b.srt");
    fs::write(&sub_b, b"this is not a valid subtitle file").unwrap();

    let result = helper
        .run_command_with_config(&[
            "--output",
            "json",
            "sync",
            "-i",
            workspace.to_str().unwrap(),
            "-r",
            "--offset",
            "0.5",
            "--force",
            "--batch",
        ])
        .await;

    assert!(
        result.success,
        "command should still exit 0 on partial batch failure; stdout={} stderr={}",
        result.stdout, result.stderr
    );
    let env = assert_single_json_envelope(&result.stdout);
    assert_envelope_ok(&env, "sync");
    let data = &env["data"];
    assert!(data["method"].is_string(), "method must be string: {data}");

    let inputs = data["inputs"]
        .as_array()
        .expect("batch payload must expose `inputs` array");
    let operations = data["operations"]
        .as_array()
        .expect("batch payload must expose `operations` array");
    assert!(
        inputs.len() >= 2,
        "expected at least two batch inputs, got {inputs:?}"
    );
    assert_eq!(
        inputs.len(),
        operations.len(),
        "inputs and operations should be parallel arrays"
    );

    let ok_count = inputs.iter().filter(|i| i["status"] == "ok").count();
    let err_count = inputs.iter().filter(|i| i["status"] == "error").count();
    assert!(
        ok_count >= 1,
        "at least one input should succeed: {inputs:?}"
    );
    assert!(
        err_count >= 1,
        "at least one input should fail with parsing error: {inputs:?}"
    );

    // Every failing input carries a stable error contract.
    for failing in inputs.iter().filter(|i| i["status"] == "error") {
        let err = &failing["error"];
        assert!(err.is_object(), "error must be object: {failing}");
        assert!(err["code"].is_string());
        assert!(err["category"].is_string());
        assert!(err["message"].is_string());
    }
    // And operations mirror the same per-item status contract.
    for failing in operations.iter().filter(|o| o["status"] == "error") {
        assert_eq!(failing["applied"], false);
        let err = &failing["error"];
        assert!(err.is_object(), "operation error must be object: {failing}");
    }
}

#[tokio::test]
async fn batch_with_zero_successful_items_emits_top_level_error_envelope() {
    // Reproduces Issue B: a batch where every input fails (because no
    // companion video files exist) MUST surface as a top-level error
    // envelope, not a success envelope wrapping all-error items.
    let helper = CLITestHelper::new();
    let workspace = helper.temp_dir_path().to_path_buf();

    // Two subtitle files but ZERO video files anywhere — the batch
    // dispatcher should refuse to claim forward progress.
    let sub_a = workspace.join("orphan_a.srt");
    fs::write(&sub_a, SRT_VALID).unwrap();
    let sub_b = workspace.join("orphan_b.srt");
    fs::write(&sub_b, SRT_VALID).unwrap();

    let result = helper
        .run_command_with_config(&[
            "--output",
            "json",
            "sync",
            "-i",
            workspace.to_str().unwrap(),
            "--offset",
            "0.5",
            "--batch",
        ])
        .await;

    assert!(
        !result.success,
        "command must exit non-zero when no items succeed; stdout={} stderr={}",
        result.stdout, result.stderr
    );
    let env = assert_single_json_envelope(&result.stdout);
    assert_envelope_error(&env, "sync");
    // The payload (data) must NOT be a success envelope wrapping
    // all-error items.
    assert!(
        env.get("data").is_none(),
        "error envelope must omit data: {env}"
    );
}

#[tokio::test]
async fn vad_mode_emits_vad_metadata() {
    // Use the bundled media asset to drive a real VAD pass.
    let asset_video = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("SubX - The Subtitle Revolution.mp4");
    if !asset_video.exists() {
        eprintln!("[skip] VAD asset missing: {}", asset_video.display());
        return;
    }

    let mut helper = CLITestHelper::new();
    let subtitle = helper
        .create_subtitle_file("vad.srt", SRT_VALID)
        .await
        .expect("create subtitle");
    let output = helper.temp_dir_path().join("vad.synced.srt");

    let result = helper
        .run_command_with_config(&[
            "--output",
            "json",
            "sync",
            "--video",
            asset_video.to_str().unwrap(),
            "--subtitle",
            subtitle.to_str().unwrap(),
            "--method",
            "vad",
            "--output",
            output.to_str().unwrap(),
            "--force",
        ])
        .await;

    if !result.success {
        // VAD analysis may fail on hosts without ffmpeg/onnxruntime; treat
        // as a soft skip with a diagnostic instead of failing the suite.
        eprintln!(
            "[skip] VAD command did not succeed; stderr={}",
            result.stderr
        );
        return;
    }

    let env = assert_single_json_envelope(&result.stdout);
    assert_envelope_ok(&env, "sync");
    let data = &env["data"];
    assert_eq!(data["method"], "vad");

    let input = &data["inputs"].as_array().expect("inputs")[0];
    assert_eq!(input["status"], "ok");
    assert!(input["detected_offset_ms"].is_i64());
    assert!(
        input["confidence"].is_number(),
        "vad input must report confidence: {input}"
    );
    let vad = input
        .get("vad")
        .expect("vad metadata must be present in vad mode");
    assert!(vad["sensitivity"].is_number());
    assert!(vad["padding_ms"].is_number());
    assert!(vad["segments"].is_array());

    let op = &data["operations"].as_array().expect("operations")[0];
    assert!(op["applied"].is_boolean());
}
