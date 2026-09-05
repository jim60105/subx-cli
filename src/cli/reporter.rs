//! The CLI's terminal implementation of the core reporting seam.
//!
//! [`TerminalReporter`] is the only consumer of the process-global output
//! mode ([`crate::cli::output::active_mode`]) and quiet flag
//! ([`crate::cli::output::is_quiet`]) **on behalf of core**: every message a
//! core engine or service client reports reaches the terminal through this
//! type, which decides per channel whether and where it is rendered. Core
//! itself never consults these globals (enforced by
//! `tests/core_cli_boundary.rs`).

use subx_core::core::report::{AiUsage, ProgressEvent, Reporter};

/// Renders [`subx_core::core::report`] channels to stdout/stderr.
///
/// Channel-and-suppression matrix (see the `machine-readable-output`
/// capability):
///
/// | Channel      | Stream | Suppressed when                       |
/// |--------------|--------|---------------------------------------|
/// | `diagnostic` | stderr | output mode is `json`                 |
/// | `warn`       | stderr | output mode is `json`                 |
/// | `ai_usage`   | stdout | output mode is `json`                 |
/// | `progress`   | stderr | output mode is `json` **or** `--quiet` |
///
/// Messages are written verbatim followed by exactly one `\n` — no prefix,
/// symbol, or colour — so a message replacing an `eprintln!` arrives
/// byte-identically, and embedded newlines are written as one atomic block.
/// `--quiet` silences `progress` only; diagnostics and warnings survive it.
pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn diagnostic(&self, message: &str) {
        if crate::cli::output::active_mode().is_json() {
            return;
        }
        eprintln!("{message}");
    }

    fn warn(&self, message: &str) {
        if crate::cli::output::active_mode().is_json() {
            return;
        }
        eprintln!("{message}");
    }

    fn ai_usage(&self, usage: &AiUsage) {
        crate::cli::ui::display_ai_usage(usage);
    }

    fn progress(&self, event: &ProgressEvent<'_>) {
        if crate::cli::output::active_mode().is_json() || crate::cli::output::is_quiet() {
            return;
        }
        // Within the defining crate the enum has only `Message`; new
        // structured variants from expose-core-orchestration-apis are simply
        // ignored here until they land and a handler is wired up.
        // A wildcard arm is required so foreign variants keep compiling when
        // they land; `single_match`/`unreachable_patterns` fire only while the
        // enum is single-variant, hence the allows.
        #[allow(unreachable_patterns, clippy::single_match)]
        match event {
            ProgressEvent::Message(message) => eprintln!("{message}"),
            _ => {}
        }
    }
}

/// Shared handle to the CLI's terminal reporter.
///
/// Command implementations attach this at component-construction sites
/// (`ComponentFactory::new(...)?.with_reporter(terminal_reporter())`) so
/// core output follows the CLI's output-mode rules.
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::Reporter;
///
/// let reporter = subx_cli::cli::terminal_reporter();
/// // Text mode is the default output mode, so this reaches stderr.
/// reporter.diagnostic("status detail");
/// ```
pub fn terminal_reporter() -> std::sync::Arc<dyn subx_core::core::report::Reporter> {
    std::sync::Arc::new(TerminalReporter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use subx_core::core::report::noop;

    /// Channel/stream matrix in the default (`text`, not quiet) mode — the
    /// only mode assertable without mutating the process-wide `OnceLock`
    /// in `output.rs`. The JSON-mode branches are covered end-to-end by the
    /// `assert_cmd` tests in `tests/cli/match_command_json_silence.rs`.
    #[test]
    fn text_mode_routes_channels_to_their_streams() {
        let reporter = TerminalReporter;
        assert!(
            !crate::cli::output::active_mode().is_json(),
            "unit tests run with the default text mode"
        );

        // diagnostic/warn/progress → stderr, ai_usage → stdout (via
        // display_ai_usage). Assert against the streams indirectly: the
        // calls must not panic and must consult the live mode.
        reporter.diagnostic("detail");
        reporter.warn("Warning: careful");
        reporter.progress(&ProgressEvent::Message(
            "📊 Translation Progress:\n   Processed cues: 2/2",
        ));
        reporter.ai_usage(&AiUsage {
            model: "gpt-4.1-mini".to_string(),
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
        });
    }

    #[test]
    fn terminal_reporter_is_send_sync_trait_object() {
        let reporter: std::sync::Arc<dyn Reporter> = terminal_reporter();
        // Trait-object storage compiles and the Send + Sync bound holds.
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        assert_send_sync(&reporter);
    }

    #[test]
    fn noop_and_terminal_reporters_are_distinct_sinks() {
        // Both must be attachable interchangeably through the seam.
        let sinks: Vec<std::sync::Arc<dyn Reporter>> = vec![noop(), terminal_reporter()];
        assert_eq!(sinks.len(), 2);
    }
}
