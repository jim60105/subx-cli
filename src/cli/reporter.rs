//! The CLI's terminal implementation of the core reporting seam.
//!
//! [`TerminalReporter`] is the only consumer of the process-global output
//! mode ([`crate::cli::output::active_mode`]) and quiet flag
//! ([`crate::cli::output::is_quiet`]) **on behalf of core**: every message a
//! core engine or service client reports reaches the terminal through this
//! type, which decides per channel whether and where it is rendered. Core
//! itself never consults these globals (enforced by
//! `tests/core_cli_boundary.rs`).

use std::sync::Mutex;

use indicatif::{ProgressBar, ProgressDrawTarget};
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
///
/// # Batch progress bar ownership
///
/// This reporter is the **single owner** of the batch progress bar in the
/// CLI (see `expose-core-orchestration-apis`). A [`ProgressEvent::Started`]
/// constructs exactly one bar through [`crate::cli::ui::create_progress_bar`]
/// — which force-hides it in JSON mode — and additionally hides it when
/// `enable_progress_bar` (read once from the `ConfigService` at
/// construction) is `false`. `Advanced` sets the position and, when an
/// `item` is supplied, the message; `Finished` ends the bar and clears the
/// slot (the old "All tasks completed" `finish_with_message` text is
/// intentionally dropped — the final rendered frame shows `{pos}/{len}`).
/// A `Started` while a bar is open replaces it rather than nesting. A
/// `Message` received while a stream is open becomes the bar's message
/// (the parallel batch's `Active: … | Queued: …` ticker) instead of a
/// separate line; with no stream open, messages print as before.
/// Structured events are processed in every output mode — suppression
/// happens through the draw target, never by skipping the lifecycle.
pub struct TerminalReporter {
    /// `general.enable_progress_bar`, read once at construction.
    enable_progress_bar: bool,
    /// The one open stream's bar, if any.
    bar: Mutex<Option<ProgressBar>>,
}

impl Default for TerminalReporter {
    fn default() -> Self {
        Self::new(true)
    }
}

impl TerminalReporter {
    /// Build a reporter honouring `enable_progress_bar` for every batch
    /// stream it renders.
    pub fn new(enable_progress_bar: bool) -> Self {
        Self {
            enable_progress_bar,
            bar: Mutex::new(None),
        }
    }

    /// Render a structured-stream event against the currently open bar,
    /// owning the replace/advance/finish transitions.
    fn render_stream_event(&self, event: &ProgressEvent<'_>) {
        let mut slot = self.bar.lock().unwrap_or_else(|e| e.into_inner());
        match event {
            ProgressEvent::Started { total } => {
                // Replace, don't nest: end and remove any open bar first.
                if let Some(old) = slot.take() {
                    old.finish_and_clear();
                }
                let bar = crate::cli::ui::create_progress_bar(*total);
                if !self.enable_progress_bar {
                    bar.set_draw_target(ProgressDrawTarget::hidden());
                }
                *slot = Some(bar);
            }
            ProgressEvent::Advanced {
                done,
                total: _,
                item,
            } => {
                if let Some(bar) = slot.as_ref() {
                    bar.set_position(*done);
                    if let Some(item) = item {
                        bar.set_message(item.to_string());
                    }
                }
            }
            ProgressEvent::Finished { .. } => {
                if let Some(bar) = slot.take() {
                    bar.finish();
                }
            }
            _ => {}
        }
    }
}

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
        match event {
            ProgressEvent::Message(_) => {
                // Free-form chatter keeps A1's rules: json/quiet silence,
                // otherwise one verbatim stderr line — except while a
                // structured stream is open, where it renders as the
                // bar's {msg} segment instead of an extra line (the
                // parallel batch ticker; task 7.5 of
                // expose-core-orchestration-apis names this choice).
                if crate::cli::output::active_mode().is_json() || crate::cli::output::is_quiet() {
                    return;
                }
                let slot = self.bar.lock().unwrap_or_else(|e| e.into_inner());
                match (&*slot, event) {
                    (Some(bar), ProgressEvent::Message(message)) => {
                        bar.set_message(message.to_string());
                    }
                    (None, ProgressEvent::Message(message)) => eprintln!("{message}"),
                    _ => {}
                }
            }
            // Structured lifecycle: processed in every mode; JSON silence
            // and `enable_progress_bar = false` are enforced through the
            // draw target (create_progress_bar force-hides in JSON mode),
            // never by skipping construction.
            _ => self.render_stream_event(event),
        }
    }
}

/// Shared handle to the CLI's terminal reporter, with progress bars
/// enabled.
///
/// Command implementations attach this at component-construction sites
/// (`ComponentFactory::new(...)?.with_reporter(terminal_reporter())`) so
/// core output follows the CLI's output-mode rules. Commands that have a
/// loaded configuration SHALL use
/// [`terminal_reporter_with_progress_bar`] so `general.enable_progress_bar`
/// is honoured.
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
    terminal_reporter_with_progress_bar(true)
}

/// Shared handle to a terminal reporter honouring `general.enable_progress_bar`.
///
/// The flag is read once, here, at construction — never per event — from
/// the `ConfigService` at whichever place builds the reporter (the CLI
/// reporter is the single enforcement point for the flag).
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::Reporter;
///
/// let reporter = subx_cli::cli::terminal_reporter_with_progress_bar(false);
/// // A batch stream opened against this reporter renders no frames.
/// reporter.progress(&subx_cli::core::report::ProgressEvent::Started { total: 3 });
/// reporter.progress(&subx_cli::core::report::ProgressEvent::Finished { done: 3, total: 3 });
/// ```
pub fn terminal_reporter_with_progress_bar(
    enable_progress_bar: bool,
) -> std::sync::Arc<dyn subx_core::core::report::Reporter> {
    std::sync::Arc::new(TerminalReporter::new(enable_progress_bar))
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
        let reporter = TerminalReporter::new(true);
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
    fn structured_stream_lifecycle_renders_without_panicking() {
        // Text mode, bar enabled: the full Started/Advanced/Finished
        // lifecycle (including a replacement Started) must be panic-free
        // and end with the slot cleared.
        let reporter = TerminalReporter::new(true);
        reporter.progress(&ProgressEvent::Started { total: 2 });
        reporter.progress(&ProgressEvent::Message("Active: 1 | Queued: 1".into()));
        reporter.progress(&ProgressEvent::Advanced {
            done: 1,
            total: 2,
            item: Some("movie.srt"),
        });
        // A second Started replaces the open bar rather than nesting.
        reporter.progress(&ProgressEvent::Started { total: 1 });
        reporter.progress(&ProgressEvent::Finished { done: 1, total: 1 });
        assert!(
            reporter.bar.lock().unwrap().is_none(),
            "Finished clears the slot"
        );
    }

    #[test]
    fn noop_and_terminal_reporters_are_distinct_sinks() {
        // Both must be attachable interchangeably through the seam.
        let sinks: Vec<std::sync::Arc<dyn Reporter>> = vec![noop(), terminal_reporter()];
        assert_eq!(sinks.len(), 2);
    }

    #[test]
    fn disabled_flag_hides_the_bar_instead_of_skipping_the_lifecycle() {
        // `enable_progress_bar = false` is enforced through the draw target,
        // never by skipping construction: the bar must still exist (so
        // Advanced/Finished keep working against it) but be hidden, so it
        // can render no frames on any target.
        let reporter = TerminalReporter::new(false);
        reporter.progress(&ProgressEvent::Started { total: 2 });
        {
            let slot = reporter.bar.lock().unwrap();
            let bar = slot
                .as_ref()
                .expect("Started builds the bar even when disabled");
            assert!(bar.is_hidden(), "disabled flag must hide the draw target");
        }
        // The rest of the stream contract proceeds unchanged.
        reporter.progress(&ProgressEvent::Advanced {
            done: 1,
            total: 2,
            item: None,
        });
        {
            let slot = reporter.bar.lock().unwrap();
            assert_eq!(slot.as_ref().expect("bar still open").position(), 1);
        }
        reporter.progress(&ProgressEvent::Finished { done: 2, total: 2 });
        assert!(
            reporter.bar.lock().unwrap().is_none(),
            "Finished ends the hidden bar and clears the slot"
        );
    }
}
