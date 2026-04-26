//! Shared JSON-mode stdout/stderr discipline assertions for
//! `--output json` integration tests.
//!
//! Implements the helpers required by tasks.md §11.1: scan stdout for
//! ANSI escapes and `indicatif`-style progress-bar artifacts and assert
//! that exactly one `serde_json` document terminated by `\n` was
//! written. Returns the parsed [`serde_json::Value`] so callers can
//! perform further per-command assertions on the envelope.

use serde_json::Value;

/// ANSI / `indicatif` redraw byte/sequence patterns that MUST NEVER
/// appear on stdout in JSON mode.
const FORBIDDEN_SUBSTRINGS: &[&[u8]] = &[
    b"\x1b[",   // any ANSI CSI introducer
    b"\x1b[2K", // erase-line (indicatif redraw)
    b"\x1b[1A", // cursor-up (indicatif redraw)
    b"\x1b[0K", // erase-to-end-of-line
    b"\x1b[K",  // erase-line short form
];

/// Asserts that `stdout` from a `--output json` invocation:
///
/// * is valid UTF-8;
/// * contains EXACTLY one JSON document terminated by a single
///   trailing `\n`;
/// * contains no ANSI escape sequences;
/// * contains no `indicatif` partial-line redraw artifacts (carriage
///   returns anywhere except as part of a non-existent line ending —
///   plain `\r` bytes are forbidden because the JSON envelope itself
///   never contains them).
///
/// Returns the parsed [`serde_json::Value`] for further inspection.
///
/// # Panics
///
/// Panics with a descriptive message when any of the discipline rules
/// above are violated; this surfaces as a test failure.
pub fn assert_json_stdout_clean(stdout: &[u8]) -> Value {
    assert!(!stdout.is_empty(), "stdout was empty in JSON mode");

    let as_str =
        std::str::from_utf8(stdout).unwrap_or_else(|e| panic!("stdout was not valid UTF-8: {e}"));

    assert!(
        stdout.ends_with(b"\n"),
        "stdout did not end with a trailing newline: {as_str:?}"
    );

    for pattern in FORBIDDEN_SUBSTRINGS {
        assert!(
            !contains_subslice(stdout, pattern),
            "stdout contained forbidden ANSI/indicatif sequence {:?}: {:?}",
            String::from_utf8_lossy(pattern),
            as_str
        );
    }

    assert!(
        !stdout.contains(&b'\r'),
        "stdout contained a carriage return (indicatif partial-line \
         redraw artifact): {as_str:?}"
    );

    let body = &stdout[..stdout.len() - 1];
    assert!(
        !body.contains(&b'\n'),
        "stdout contained more than one JSON document (multiple \
         newlines): {:?}",
        String::from_utf8_lossy(body)
    );

    serde_json::from_slice(body).unwrap_or_else(|e| {
        panic!("stdout did not parse as a single JSON document: {e}; raw: {as_str:?}")
    })
}

/// Asserts envelope-level invariants on a parsed [`Value`]:
///
/// * `schema_version == "1.0"`,
/// * `command == expected_command`,
/// * `status == expected_status`,
/// * on `"ok"`: `data` MUST be present, `error` MUST be absent,
/// * on `"error"`: `error` MUST be present, `data` MUST be absent.
///
/// # Panics
///
/// Panics with a descriptive message when any envelope invariant fails.
pub fn assert_envelope(value: &Value, expected_command: &str, expected_status: &str) {
    assert_eq!(
        value["schema_version"], "1.0",
        "schema_version mismatch in envelope: {value}"
    );
    assert_eq!(
        value["command"], expected_command,
        "command mismatch in envelope: {value}"
    );
    assert_eq!(
        value["status"], expected_status,
        "status mismatch in envelope: {value}"
    );

    match expected_status {
        "ok" => {
            assert!(
                value.get("data").is_some(),
                "ok envelope must include `data`: {value}"
            );
            assert!(
                value.get("error").is_none(),
                "ok envelope must omit `error`: {value}"
            );
        }
        "error" => {
            assert!(
                value.get("error").is_some(),
                "error envelope must include `error`: {value}"
            );
            assert!(
                value.get("data").is_none(),
                "error envelope must omit `data`: {value}"
            );
        }
        other => panic!("expected_status must be \"ok\" or \"error\", got {other:?}"),
    }
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}
