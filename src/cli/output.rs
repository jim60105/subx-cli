//! Machine-readable output renderer for SubX-CLI.
//!
//! This module defines the versioned JSON envelope contract used by
//! `--output json` mode, the [`OutputMode`] enum surfaced as a top-level
//! CLI flag, and the [`OutputRenderer`] abstraction routed through the
//! command dispatcher. The text-mode renderer is a thin shim that
//! preserves today's interactive UX; the JSON renderer owns stdout
//! exclusively and emits exactly one JSON document per invocation.
//!
//! # Stdout / stderr discipline
//!
//! - **JSON mode (`--output json`)**: stdout SHALL receive *exactly* one
//!   `serde_json` document terminated by a trailing `\n`. No other writes
//!   to stdout from any command, helper, or library code are permitted —
//!   `print_success`/`print_warning`/`display_match_results`/progress
//!   bars are silenced for the lifetime of the process. Stderr MAY still
//!   carry free-form diagnostic logs, but ANSI styling and status symbols
//!   are stripped (see [`crate::cli::ui`]). With `--quiet`, stderr
//!   chatter is additionally suppressed except for fatal errors emitted
//!   by the renderer itself.
//! - **Text mode (default)**: behavior is unchanged from prior releases.
//!
//! # Schema versioning
//!
//! [`SCHEMA_VERSION`] follows semver. Additive payload changes are
//! minor bumps; renames or removals are major bumps and require a new
//! OpenSpec change proposal.

use crate::error::SubXError;
use serde::Serialize;
use std::io::{self, Write};
use std::sync::OnceLock;

/// Schema version emitted in every JSON envelope.
pub const SCHEMA_VERSION: &str = "1.0";

/// Output mode selected by the user via `--output` or `SUBX_OUTPUT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum OutputMode {
    /// Human-oriented colored text output (default, unchanged contract).
    #[default]
    Text,
    /// Machine-readable JSON envelope on stdout.
    Json,
}

impl OutputMode {
    /// Parse a string token (case-insensitive) into an [`OutputMode`].
    ///
    /// Returns `None` for unrecognized tokens; callers default to
    /// [`OutputMode::Text`].
    pub fn from_token(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "text" => Some(OutputMode::Text),
            "json" => Some(OutputMode::Json),
            _ => None,
        }
    }

    /// Returns true when machine-readable mode is active.
    pub fn is_json(self) -> bool {
        matches!(self, OutputMode::Json)
    }
}

// ─── Process-global active mode (set once during dispatch) ──────────────
//
// `OnceLock` (never `static mut` or `Lazy<Mutex>`) holds the resolved
// output mode for the lifetime of the process so UI helpers in
// `crate::cli::ui` can suppress stdout chatter and strip ANSI without
// threading the mode through every callsite.

static ACTIVE_MODE: OnceLock<OutputMode> = OnceLock::new();
static QUIET: OnceLock<bool> = OnceLock::new();

/// Install the resolved output mode and quiet flag globally.
///
/// Calling this more than once has no effect — the first install wins.
/// `main.rs`/`run_with_config` SHALL invoke this once before any command
/// runs.
pub fn install_active_mode(mode: OutputMode, quiet: bool) {
    let _ = ACTIVE_MODE.set(mode);
    let _ = QUIET.set(quiet);
}

/// Returns the active output mode, defaulting to [`OutputMode::Text`]
/// when not yet installed.
pub fn active_mode() -> OutputMode {
    ACTIVE_MODE.get().copied().unwrap_or(OutputMode::Text)
}

/// Returns whether `--quiet` is active.
pub fn is_quiet() -> bool {
    QUIET.get().copied().unwrap_or(false)
}

// ─── Envelope types ──────────────────────────────────────────────────────

/// Top-level JSON envelope written to stdout in JSON mode.
///
/// Successful runs SHALL omit the `error` field; failed runs SHALL omit
/// the `data` field. Both omissions are achieved through `serde`'s
/// `skip_serializing_if = "Option::is_none"`.
#[derive(Debug, Serialize)]
pub struct Envelope<'a, T: Serialize> {
    /// Stable schema version (semver-style).
    pub schema_version: &'static str,
    /// Command name (e.g. `"match"`, `"sync"`, `"convert"`).
    pub command: &'a str,
    /// Either `"ok"` or `"error"`.
    pub status: &'static str,
    /// Command-specific payload. Omitted when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error envelope. Omitted when `status == "ok"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorEnvelope>,
    /// Optional non-fatal warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// Stable error payload carried in [`Envelope::error`].
///
/// Field naming is locked by the
/// `machine-readable-output`/`error-handling` specs.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    /// Stable snake_case category from the closed [`SubXError`] set or
    /// the synthetic `"argument_parsing"` category for clap failures.
    pub category: String,
    /// Stable upper-snake-case machine code (e.g. `E_AI_SERVICE`).
    pub code: String,
    /// Process exit code returned alongside the envelope.
    pub exit_code: i32,
    /// Human-readable message (English; matches `user_friendly_message`).
    pub message: String,
    /// Optional short remediation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Optional structured details (partial results, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorEnvelope {
    /// Build an envelope from a [`SubXError`].
    pub fn from_error(err: &SubXError) -> Self {
        Self {
            category: err.category().to_string(),
            code: err.machine_code().to_string(),
            exit_code: err.exit_code(),
            message: err.user_friendly_message(),
            hint: err.hint().map(str::to_string),
            details: None,
        }
    }

    /// Build the synthetic envelope used for clap argument-parsing
    /// failures. The category `argument_parsing` is intentionally NOT
    /// part of the closed [`SubXError`]-derived set.
    pub fn argument_parsing(message: String, exit_code: i32) -> Self {
        Self {
            category: "argument_parsing".to_string(),
            code: "E_ARGUMENT_PARSING".to_string(),
            message,
            exit_code,
            hint: None,
            details: None,
        }
    }
}

// ─── Renderer abstraction ────────────────────────────────────────────────

/// Renderer trait routing success/error envelopes to the active output
/// stream.
///
/// The trait uses generic methods (not object-safe). Commands typically
/// use the free [`emit_success`]/[`emit_error`] helpers instead of
/// constructing renderers directly.
pub trait OutputRenderer {
    /// Emit a success envelope.
    fn render_success<T: Serialize>(&self, command: &str, data: T) -> io::Result<()>;
    /// Emit an error envelope.
    fn render_error(&self, command: &str, err: &SubXError) -> io::Result<()>;
}

/// Text renderer — preserves today's UX as a no-op for the envelope.
///
/// Per-command text rendering is performed by the command itself through
/// the existing `crate::cli::ui` helpers. The text renderer's
/// `render_success` is therefore a no-op; `render_error` is also a no-op
/// because `main.rs` already prints `user_friendly_message()` via
/// `print_error` in text mode.
#[derive(Debug, Default, Clone, Copy)]
pub struct TextRenderer;

impl OutputRenderer for TextRenderer {
    fn render_success<T: Serialize>(&self, _command: &str, _data: T) -> io::Result<()> {
        Ok(())
    }
    fn render_error(&self, _command: &str, _err: &SubXError) -> io::Result<()> {
        Ok(())
    }
}

/// JSON renderer — owns stdout exclusively in JSON mode.
///
/// Each call writes EXACTLY one `serde_json` document followed by a
/// single `\n` and flushes the underlying writer. Multiple invocations
/// of the binary therefore stream as NDJSON when concatenated.
pub struct JsonRenderer<W: Write> {
    writer: std::cell::RefCell<W>,
}

impl JsonRenderer<io::Stdout> {
    /// Construct a renderer that writes to the process stdout handle.
    pub fn stdout() -> Self {
        Self {
            writer: std::cell::RefCell::new(io::stdout()),
        }
    }
}

impl<W: Write> JsonRenderer<W> {
    /// Construct a renderer wrapping any [`Write`] implementation.
    pub fn new(writer: W) -> Self {
        Self {
            writer: std::cell::RefCell::new(writer),
        }
    }

    fn write_envelope<T: Serialize>(&self, envelope: &Envelope<'_, T>) -> io::Result<()> {
        let mut w = self.writer.borrow_mut();
        serde_json::to_writer(&mut *w, envelope).map_err(io::Error::other)?;
        w.write_all(b"\n")?;
        w.flush()?;
        Ok(())
    }
}

impl<W: Write> OutputRenderer for JsonRenderer<W> {
    fn render_success<T: Serialize>(&self, command: &str, data: T) -> io::Result<()> {
        let envelope = Envelope::<T> {
            schema_version: SCHEMA_VERSION,
            command,
            status: "ok",
            data: Some(data),
            error: None,
            warnings: None,
        };
        self.write_envelope(&envelope)
    }

    fn render_error(&self, command: &str, err: &SubXError) -> io::Result<()> {
        let envelope = Envelope::<serde_json::Value> {
            schema_version: SCHEMA_VERSION,
            command,
            status: "error",
            data: None,
            error: Some(ErrorEnvelope::from_error(err)),
            warnings: None,
        };
        self.write_envelope(&envelope)
    }
}

/// Emit a success envelope through the appropriate renderer.
///
/// In [`OutputMode::Text`] this is a no-op. In [`OutputMode::Json`] this
/// writes exactly one JSON document followed by `\n` to stdout and
/// flushes. I/O errors are silently ignored at the boundary because the
/// process exits immediately after; callers wanting to surface I/O
/// errors should use a [`JsonRenderer`] directly.
pub fn emit_success<T: Serialize>(mode: OutputMode, command: &str, data: T) {
    match mode {
        OutputMode::Text => {}
        OutputMode::Json => {
            let _ = JsonRenderer::stdout().render_success(command, data);
        }
    }
}

/// Emit a success envelope with optional non-fatal warnings attached.
///
/// Behaves like [`emit_success`] in [`OutputMode::Text`] (no-op) and in
/// [`OutputMode::Json`] writes a JSON envelope whose `warnings` field is
/// set to `Some(warnings)` when the supplied vector is non-empty, or
/// `None` (omitted via `skip_serializing_if`) when the vector is empty.
/// This keeps the JSON document byte-equivalent to the no-warnings shape
/// for callers that pass in an empty list.
pub fn emit_success_with_warnings<T: Serialize>(
    mode: OutputMode,
    command: &str,
    data: T,
    warnings: Vec<String>,
) {
    match mode {
        OutputMode::Text => {}
        OutputMode::Json => {
            let warnings = if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            };
            let envelope = Envelope::<T> {
                schema_version: SCHEMA_VERSION,
                command,
                status: "ok",
                data: Some(data),
                error: None,
                warnings,
            };
            let _ = JsonRenderer::stdout().write_envelope(&envelope);
        }
    }
}

/// Emit an error envelope through the appropriate renderer.
///
/// In [`OutputMode::Text`] this is a no-op (the existing `print_error`
/// path in `main.rs` is responsible for stderr rendering). In
/// [`OutputMode::Json`] this writes the JSON error envelope to stdout.
pub fn emit_error(mode: OutputMode, command: &str, err: &SubXError) {
    match mode {
        OutputMode::Text => {}
        OutputMode::Json => {
            let _ = JsonRenderer::stdout().render_error(command, err);
        }
    }
}

/// Emit a synthetic argument-parsing error envelope (clap failures).
///
/// Used by `main.rs` when `Cli::try_parse()` returns an error other
/// than help/version display.
pub fn emit_argument_parsing_error(command: Option<&str>, message: String, exit_code: i32) {
    let envelope = Envelope::<serde_json::Value> {
        schema_version: SCHEMA_VERSION,
        command: command.unwrap_or(""),
        status: "error",
        data: None,
        error: Some(ErrorEnvelope::argument_parsing(message, exit_code)),
        warnings: None,
    };
    let renderer = JsonRenderer::stdout();
    let _ = renderer.write_envelope(&envelope);
}

/// Strip ANSI CSI escape sequences from a borrowed string.
///
/// Used by clap-error rendering and by stderr UI helpers in JSON mode.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // CSI: ESC [ ... <final byte 0x40..0x7e>
            i += 2;
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7e).contains(&b) {
                    break;
                }
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

// ─── Test-only helpers ──────────────────────────────────────────────────

/// Test-only assertion: stdout in JSON mode contains exactly one JSON
/// document followed by a single trailing `\n` and no ANSI sequences.
///
/// Returns `Ok(parsed)` on success or a descriptive error string.
#[cfg(test)]
pub fn assert_json_stdout_clean(stdout: &[u8]) -> Result<serde_json::Value, String> {
    if stdout.is_empty() {
        return Err("stdout was empty".to_string());
    }
    if !stdout.ends_with(b"\n") {
        return Err("stdout did not end with newline".to_string());
    }
    if stdout.contains(&0x1b) {
        return Err("stdout contained ANSI escape sequence".to_string());
    }
    // Trim trailing newline, refuse multi-document output.
    let body = &stdout[..stdout.len() - 1];
    if body.contains(&b'\n') {
        return Err("stdout contained more than one line".to_string());
    }
    serde_json::from_slice(body).map_err(|e| format!("stdout did not parse as JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn output_mode_from_token_is_case_insensitive() {
        assert_eq!(OutputMode::from_token("json"), Some(OutputMode::Json));
        assert_eq!(OutputMode::from_token("JSON"), Some(OutputMode::Json));
        assert_eq!(OutputMode::from_token(" Text "), Some(OutputMode::Text));
        assert_eq!(OutputMode::from_token("yaml"), None);
    }

    #[test]
    fn json_renderer_emits_single_document_with_newline() {
        let mut buf = Vec::new();
        let renderer = JsonRenderer::new(&mut buf);
        renderer
            .render_success("match", Sample { value: 42 })
            .expect("write");
        // Drop renderer to release borrow before reading buf.
        drop(renderer);
        let parsed = assert_json_stdout_clean(&buf).expect("clean JSON");
        assert_eq!(parsed["schema_version"], SCHEMA_VERSION);
        assert_eq!(parsed["command"], "match");
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["data"]["value"], 42);
        assert!(parsed.get("error").is_none(), "error must be omitted on ok");
    }

    #[test]
    fn json_renderer_omits_data_on_error() {
        let mut buf = Vec::new();
        let renderer = JsonRenderer::new(&mut buf);
        let err = SubXError::config("bad");
        renderer.render_error("convert", &err).expect("write");
        drop(renderer);
        let parsed = assert_json_stdout_clean(&buf).expect("clean JSON");
        assert_eq!(parsed["status"], "error");
        assert!(
            parsed.get("data").is_none(),
            "data must be omitted on error"
        );
        assert_eq!(parsed["error"]["category"], "config");
        assert_eq!(parsed["error"]["code"], "E_CONFIG");
        assert_eq!(parsed["error"]["exit_code"], 2);
    }

    #[test]
    fn argument_parsing_envelope_shape() {
        let env = ErrorEnvelope::argument_parsing("unknown flag --foo".into(), 2);
        assert_eq!(env.category, "argument_parsing");
        assert_eq!(env.code, "E_ARGUMENT_PARSING");
        assert_eq!(env.exit_code, 2);
    }

    #[test]
    fn strip_ansi_removes_csi_sequences() {
        let input = "\x1b[31m\x1b[1mfailed\x1b[0m";
        assert_eq!(strip_ansi(input), "failed");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn text_renderer_is_noop() {
        let r = TextRenderer;
        r.render_success("x", Sample { value: 1 }).unwrap();
        r.render_error("x", &SubXError::config("y")).unwrap();
    }
}
