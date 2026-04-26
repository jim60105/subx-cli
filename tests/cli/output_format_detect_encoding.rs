//! Integration tests for `subx-cli --output json detect-encoding ...`.
//!
//! These tests spawn the compiled binary (via `assert_cmd`) and assert
//! that stdout contains exactly one JSON envelope conforming to the
//! `encoding-detection` and `machine-readable-output` specs.
//!
//! Wired into the test crate via
//! `tests/output_format_detect_encoding_tests.rs`.

use assert_cmd::Command;
use encoding_rs::GBK;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

const SAMPLE_SRT_UTF8: &str = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n\
                               2\n00:00:02,500 --> 00:00:03,500\nSecond cue\n\n";
const SAMPLE_SRT_GBK_TEXT: &str = "1\n00:00:01,000 --> 00:00:02,000\n\
                                   你好世界，这是中文字幕的测试内容。\n\n\
                                   2\n00:00:02,500 --> 00:00:03,500\n\
                                   汉字编码检测样本，用于验证 GBK 解码路径。\n\n";

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

fn write_utf8_bom(path: &std::path::Path, body: &str) {
    let mut bytes = Vec::with_capacity(body.len() + 3);
    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    bytes.extend_from_slice(body.as_bytes());
    fs::write(path, bytes).unwrap();
}

fn write_utf16le_bom(path: &std::path::Path, body: &str) {
    let mut bytes = Vec::with_capacity(2 + body.len() * 2);
    bytes.extend_from_slice(&[0xFF, 0xFE]);
    for unit in body.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn write_gbk(path: &std::path::Path, body: &str) {
    let (encoded, _, had_errors) = GBK.encode(body);
    assert!(!had_errors, "GBK encoder produced unmappable characters");
    fs::write(path, encoded.as_ref()).unwrap();
}

fn run_detect_encoding<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Vec<u8> {
    let mut cmd = Command::cargo_bin("subx-cli").unwrap();
    cmd.arg("--output").arg("json").arg("detect-encoding");
    for a in args {
        cmd.arg(a);
    }
    let assert = cmd.assert().success();
    assert.get_output().stdout.clone()
}

#[test]
fn detect_encoding_utf8_single_file_emits_ok_envelope() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("utf8.srt");
    fs::write(&p, SAMPLE_SRT_UTF8).unwrap();

    let stdout = run_detect_encoding(&[p.to_str().unwrap()]);
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "ok");
    assert!(env.get("error").is_none());

    let files = env["data"]["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    let item = &files[0];
    assert_eq!(item["status"], "ok");
    let encoding = item["encoding"].as_str().unwrap().to_ascii_uppercase();
    assert_eq!(encoding, "UTF-8");
    assert_eq!(item["has_bom"], false);
    let confidence = item["confidence"].as_f64().expect("confidence number");
    assert!((0.0..=1.0).contains(&confidence));
    assert!(
        item["bytes_sampled"].as_u64().unwrap() > 0,
        "bytes_sampled must be positive"
    );
    assert!(item.get("error").is_none());
}

#[test]
fn detect_encoding_utf8_with_bom_reports_has_bom() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("bom.srt");
    write_utf8_bom(&p, SAMPLE_SRT_UTF8);

    let stdout = run_detect_encoding(&[p.to_str().unwrap()]);
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "ok");

    let item = &env["data"]["files"][0];
    assert_eq!(item["status"], "ok");
    let encoding = item["encoding"].as_str().unwrap().to_ascii_uppercase();
    assert_eq!(encoding, "UTF-8");
    assert_eq!(item["has_bom"], true);
}

#[test]
fn detect_encoding_utf16le_with_bom_reports_utf16le() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("u16le.srt");
    write_utf16le_bom(&p, SAMPLE_SRT_UTF8);

    let stdout = run_detect_encoding(&[p.to_str().unwrap()]);
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "ok");

    let item = &env["data"]["files"][0];
    assert_eq!(item["status"], "ok");
    let encoding = item["encoding"].as_str().unwrap().to_ascii_uppercase();
    assert_eq!(encoding, "UTF-16LE");
    assert_eq!(item["has_bom"], true);
}

#[test]
fn detect_encoding_gbk_reports_non_utf8_charset() {
    let dir = TempDir::new().unwrap();
    let p = dir.path().join("gbk.srt");
    write_gbk(&p, SAMPLE_SRT_GBK_TEXT);

    let stdout = run_detect_encoding(&[p.to_str().unwrap()]);
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "ok");

    let item = &env["data"]["files"][0];
    assert_eq!(item["status"], "ok");
    let encoding = item["encoding"].as_str().unwrap().to_ascii_uppercase();
    assert_ne!(encoding, "UTF-8", "GBK input must not be reported as UTF-8");
    assert_eq!(item["has_bom"], false);
}

#[test]
fn detect_encoding_batch_mixed_emits_one_item_per_file() {
    let dir = TempDir::new().unwrap();
    let utf8 = dir.path().join("plain.srt");
    let bom = dir.path().join("bom.srt");
    let u16 = dir.path().join("u16.srt");
    let gbk = dir.path().join("gbk.srt");
    fs::write(&utf8, SAMPLE_SRT_UTF8).unwrap();
    write_utf8_bom(&bom, SAMPLE_SRT_UTF8);
    write_utf16le_bom(&u16, SAMPLE_SRT_UTF8);
    write_gbk(&gbk, SAMPLE_SRT_GBK_TEXT);

    let stdout = run_detect_encoding(&[
        utf8.to_str().unwrap(),
        bom.to_str().unwrap(),
        u16.to_str().unwrap(),
        gbk.to_str().unwrap(),
    ]);
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "ok");
    let files = env["data"]["files"].as_array().expect("files array");
    assert_eq!(files.len(), 4, "all four files reported");

    for item in files {
        assert_eq!(item["status"], "ok");
        assert!(item["path"].is_string());
        assert!(item["encoding"].is_string());
        assert!(item["confidence"].is_number());
        assert!(item["has_bom"].is_boolean());
        assert!(item["bytes_sampled"].is_number());
        assert!(item.get("error").is_none());
    }
}

#[test]
fn detect_encoding_batch_with_unreadable_path_yields_per_item_error() {
    let dir = TempDir::new().unwrap();
    let good = dir.path().join("good.srt");
    fs::write(&good, SAMPLE_SRT_UTF8).unwrap();
    let missing = dir.path().join("does_not_exist.srt");

    let stdout = run_detect_encoding(&[good.to_str().unwrap(), missing.to_str().unwrap()]);
    let env = parse_single_envelope(&stdout);
    // Top-level still ok per spec — partial per-item error.
    assert_envelope_shape(&env, "detect-encoding", "ok");

    let files = env["data"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let oks = files.iter().filter(|i| i["status"] == "ok").count();
    let errs = files.iter().filter(|i| i["status"] == "error").count();
    assert_eq!(oks, 1);
    assert_eq!(errs, 1);

    let err_item = files.iter().find(|i| i["status"] == "error").unwrap();
    let err = err_item["error"].as_object().expect("per-item error");
    let category = err["category"].as_str().unwrap();
    assert!(
        category == "path_not_found" || category == "file_not_found",
        "unexpected category: {category}"
    );
    assert!(err["code"].as_str().unwrap().starts_with("E_"));
    assert!(
        err.get("exit_code").is_none(),
        "per-item error must omit exit_code"
    );
}

#[test]
fn detect_encoding_single_missing_path_emits_top_level_error() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.srt");

    let assert = Command::cargo_bin("subx-cli")
        .unwrap()
        .args([
            "--output",
            "json",
            "detect-encoding",
            missing.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stdout = assert.get_output().stdout.clone();
    let env = parse_single_envelope(&stdout);
    assert_envelope_shape(&env, "detect-encoding", "error");
    assert!(
        env.get("data").is_none(),
        "error envelope must omit data field"
    );
    let err = env["error"].as_object().expect("top-level error");
    let category = err["category"].as_str().unwrap();
    assert!(
        category == "path_not_found" || category == "file_not_found",
        "unexpected category: {category}"
    );
    assert!(err["code"].as_str().unwrap().starts_with("E_"));
    assert!(err["exit_code"].as_i64().unwrap() != 0);
}
