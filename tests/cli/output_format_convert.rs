//! Integration tests for `subx-cli --output json convert ...`.
//!
//! These tests spawn the compiled binary (via `assert_cmd`) and assert
//! that stdout contains exactly one JSON envelope conforming to the
//! `format-conversion` and `machine-readable-output` specs.
//!
//! Wired into the test crate via `tests/output_format_convert_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

/// Build an `assert_cmd::Command` for `subx-cli` with HOME/XDG_CONFIG_HOME
/// pointed at the supplied tempdir so the developer's real
/// `~/.config/subx/config.toml` cannot leak into the test.
fn isolated_subx_cli(workdir: &std::path::Path) -> Command {
    let xdg = workdir.join(".xdg");
    fs::create_dir_all(&xdg).unwrap();
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.env("HOME", workdir)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("SUBX_GENERAL_ENABLE_PROGRESS_BAR", "false");
    cmd
}

const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n\
                          2\n00:00:02,500 --> 00:00:03,500\nSecond cue\n\n";
const SAMPLE_VTT: &str = "WEBVTT\n\n\
                          1\n00:00:01.000 --> 00:00:02.000\nHello world\n\n\
                          2\n00:00:02.500 --> 00:00:03.500\nSecond cue\n\n";
const CORRUPT_SRT: &str = "this is not a subtitle file at all\nno timestamps here\n";

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

fn assert_envelope_shape(env: &Value, command: &str, status: &str) {
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], command);
    assert_eq!(env["status"], status);
}

#[test]
fn convert_srt_to_ass_single_file_emits_ok_envelope() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.srt");
    let output = dir.path().join("a.ass");
    fs::write(&input, SAMPLE_SRT).unwrap();

    let assert = isolated_subx_cli(dir.path())
        .args([
            "--output",
            "json",
            "convert",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "ass",
            "--keep-original",
        ])
        .assert()
        .success();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "convert", "ok");
    assert!(env.get("error").is_none(), "ok envelope must omit error");

    let conversions = env["data"]["conversions"]
        .as_array()
        .expect("conversions array");
    assert_eq!(conversions.len(), 1);
    let item = &conversions[0];
    assert_eq!(item["status"], "ok");
    assert_eq!(item["applied"], true);
    assert_eq!(item["source_format"], "srt");
    assert_eq!(item["target_format"], "ass");
    assert_eq!(item["encoding"], "utf-8");
    assert!(item.get("error").is_none());
    assert!(output.exists(), "output file should be written");
}

#[test]
fn convert_vtt_to_srt_single_file_emits_ok_envelope() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("clip.vtt");
    let output = dir.path().join("clip.srt");
    fs::write(&input, SAMPLE_VTT).unwrap();

    let assert = isolated_subx_cli(dir.path())
        .args([
            "--output",
            "json",
            "convert",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "srt",
            "--keep-original",
        ])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "convert", "ok");
    let conversions = env["data"]["conversions"].as_array().unwrap();
    assert_eq!(conversions.len(), 1);
    assert_eq!(conversions[0]["source_format"], "vtt");
    assert_eq!(conversions[0]["target_format"], "srt");
    assert_eq!(conversions[0]["status"], "ok");
}

#[test]
fn convert_batch_mixed_success_and_failure_keeps_top_level_ok() {
    let dir = TempDir::new().unwrap();
    let good1 = dir.path().join("good1.srt");
    let good2 = dir.path().join("good2.srt");
    let bad = dir.path().join("bad.srt");
    fs::write(&good1, SAMPLE_SRT).unwrap();
    fs::write(&good2, SAMPLE_SRT).unwrap();
    fs::write(&bad, CORRUPT_SRT).unwrap();

    let out_dir = dir.path().join("out");
    fs::create_dir(&out_dir).unwrap();

    let assert = isolated_subx_cli(dir.path())
        .args([
            "--output",
            "json",
            "convert",
            dir.path().to_str().unwrap(),
            "--format",
            "vtt",
            "--output",
            out_dir.to_str().unwrap(),
            "--keep-original",
        ])
        .assert()
        .success(); // exit 0 per spec — partial failure is per-item

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "convert", "ok");
    let conversions = env["data"]["conversions"].as_array().unwrap();
    assert_eq!(conversions.len(), 3, "all three files reported");

    let oks = conversions.iter().filter(|c| c["status"] == "ok").count();
    let errs = conversions
        .iter()
        .filter(|c| c["status"] == "error")
        .count();
    assert_eq!(oks, 2, "two files convert successfully");
    assert_eq!(errs, 1, "one file errors");

    let err_item = conversions.iter().find(|c| c["status"] == "error").unwrap();
    assert_eq!(err_item["applied"], false);
    let err = err_item["error"].as_object().expect("per-item error");
    assert_eq!(err["category"], "subtitle_format");
    assert!(err["code"].as_str().unwrap().starts_with("E_"));
    assert!(
        err.get("exit_code").is_none(),
        "per-item error omits exit_code"
    );
}

#[test]
fn convert_with_encoding_override_reports_encoding() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("a.srt");
    let output = dir.path().join("a.vtt");
    fs::write(&input, SAMPLE_SRT).unwrap();

    let assert = isolated_subx_cli(dir.path())
        .args([
            "--output",
            "json",
            "convert",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--format",
            "vtt",
            "--encoding",
            "utf-16",
            "--keep-original",
        ])
        .assert()
        .success();

    let env = parse_single_envelope(&assert.get_output().stdout);
    assert_envelope_shape(&env, "convert", "ok");
    let item = &env["data"]["conversions"][0];
    assert_eq!(item["encoding"], "utf-16");
    assert_eq!(item["target_format"], "vtt");
}
