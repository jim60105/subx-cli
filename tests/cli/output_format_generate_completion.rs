//! Integration tests for `subx-cli generate-completion` under the
//! machine-readable output contract.
//!
//! Per the `machine-readable-output` spec §"generate-completion Rejects
//! JSON Output Mode", invoking `generate-completion` with `--output
//! json` (or `SUBX_OUTPUT=json`) MUST refuse to emit a shell script and
//! SHALL produce a single JSON error envelope on stdout with
//! `error.category == "command_execution"`, `error.code ==
//! "E_OUTPUT_MODE_UNSUPPORTED"`, and exit code `1`
//! (== `SubXError::CommandExecution(_).exit_code()`).
//!
//! Wired into the test crate via
//! `tests/output_format_generate_completion_tests.rs`.

use assert_cmd::Command;
use serde_json::Value;

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

#[test]
fn generate_completion_with_output_json_emits_error_envelope() {
    let output = Command::cargo_bin("subx-cli")
        .unwrap()
        .arg("--output")
        .arg("json")
        .arg("generate-completion")
        .arg("bash")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert_eq!(
        output.status.code(),
        Some(1),
        "exit code must equal SubXError::CommandExecution(_).exit_code() (1); stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let env = parse_single_envelope(&output.stdout);
    assert_eq!(env["schema_version"], "1.0");
    assert_eq!(env["command"], "generate-completion");
    assert_eq!(env["status"], "error");
    assert!(
        env.get("data").is_none(),
        "data field must be absent on error envelopes"
    );

    let err = env["error"].as_object().expect("error object");
    assert_eq!(err["category"], "command_execution");
    assert_eq!(err["code"], "E_OUTPUT_MODE_UNSUPPORTED");
    assert_eq!(err["exit_code"], 1);
    let message = err["message"].as_str().expect("error message string");
    assert!(
        message.contains("generate-completion"),
        "message should reference subcommand: {message}"
    );

    // Ensure no shell-completion bytes leaked into stdout. The bash
    // completion script always contains the word `complete`; the JSON
    // envelope itself must not.
    let body = String::from_utf8_lossy(&output.stdout);
    assert!(
        !body.contains("complete -F"),
        "stdout must not contain bash completion bytes: {body}"
    );
}

#[test]
fn generate_completion_with_subx_output_env_emits_error_envelope() {
    let output = Command::cargo_bin("subx-cli")
        .unwrap()
        .env("SUBX_OUTPUT", "json")
        .arg("generate-completion")
        .arg("zsh")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert_eq!(output.status.code(), Some(1));
    let env = parse_single_envelope(&output.stdout);
    assert_eq!(env["command"], "generate-completion");
    assert_eq!(env["error"]["code"], "E_OUTPUT_MODE_UNSUPPORTED");
    assert_eq!(env["error"]["category"], "command_execution");
}

#[test]
fn generate_completion_text_mode_emits_shell_script() {
    let output = Command::cargo_bin("subx-cli")
        .unwrap()
        .env_remove("SUBX_OUTPUT")
        .arg("generate-completion")
        .arg("bash")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(output.stdout).expect("bash script is UTF-8");
    assert!(
        stdout.contains("_subx-cli")
            || stdout.contains("complete ")
            || stdout.contains("COMPREPLY"),
        "expected bash completion script tokens in stdout: first 200 bytes = {:?}",
        &stdout.chars().take(200).collect::<String>()
    );
    // Default mode must NOT emit a JSON envelope.
    assert!(
        !stdout.trim_start().starts_with('{'),
        "text-mode stdout must not be a JSON envelope"
    );
}
