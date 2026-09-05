//! Transport-agnostic reporting seam for `src/core/` and `src/services/`.
//!
//! Core engines need to say things to whoever is driving them: a diagnostic
//! about work in progress, a warning about a recovered problem, the token
//! accounting for one AI API call, a line on the long-running-work progress
//! stream. Historically each engine printed those messages itself and asked
//! the CLI process-global output mode whether it was allowed to. That made
//! `src/core/` and `src/services/` depend on the CLI layer — the one upward
//! edge that blocks extracting core into its own crate.
//!
//! This module inverts the dependency. Core owns the [`Reporter`] *trait* —
//! the sink it reports through — and never knows what transport, if any,
//! sits behind it. The CLI owns the only terminal implementation
//! (`TerminalReporter` in `src/cli/reporter.rs`), which is where all mode
//! gating (`--output json`, `--quiet`) lives. When `src/core/` moves into
//! the `subx-core` crate, this module travels with it unchanged.
//!
//! The layering rule this module exists to enforce: **no module under
//! `src/core/` or `src/services/` may reference the CLI layer.** The
//! `tests/core_cli_boundary.rs` guard test enforces it mechanically.
//!
//! Attachment is builder-style: engines and clients default their reporter
//! to [`noop()`] (a [`NoopReporter`], so library consumers such as the
//! desktop GUI see silence by default) and accept an `Arc<dyn Reporter>`
//! through a `with_reporter` method, leaving every existing constructor
//! signature untouched.

use std::sync::Arc;

/// A transport-agnostic sink for human-oriented core output.
///
/// Every method has a default no-op body, so an implementation opts into
/// exactly the channels it cares about — [`NoopReporter`]'s entire
/// implementation is empty. The trait is object-safe and `Send + Sync`
/// because engines hold it as `Arc<dyn Reporter>` across task boundaries.
///
/// Core code calls these methods with fully-formatted strings; deciding
/// *whether* and *where* a message is shown (stdout, stderr, a GUI event
/// channel, nowhere) belongs to the implementation.
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::Reporter;
///
/// /// Only interested in warnings; silent on everything else.
/// struct WarningCounter(std::sync::atomic::AtomicUsize);
///
/// impl Reporter for WarningCounter {
///     fn warn(&self, message: &str) {
///         assert!(!message.is_empty());
///         self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
///     }
/// }
///
/// let counter = WarningCounter(std::sync::atomic::AtomicUsize::new(0));
/// counter.diagnostic("hidden by this implementation");
/// counter.warn("counted");
/// assert_eq!(
///     counter.0.load(std::sync::atomic::Ordering::Relaxed),
///     1
/// );
/// ```
pub trait Reporter: Send + Sync {
    /// Human-oriented detail about work in progress. Not a failure.
    ///
    /// # Arguments
    ///
    /// * `message` - Fully-formatted message text; may contain embedded
    ///   `\n` separators, written by the transport as one atomic block.
    ///
    /// The terminal transport suppresses this channel in JSON output mode;
    /// `--quiet` does not.
    fn diagnostic(&self, message: &str) {
        let _ = message;
    }

    /// A non-fatal problem the operation recovered from or worked around.
    ///
    /// # Arguments
    ///
    /// * `message` - Fully-formatted message text; may contain embedded
    ///   `\n` separators, written by the transport as one atomic block.
    ///
    /// The terminal transport suppresses this channel in JSON output mode;
    /// `--quiet` deliberately does **not** silence warnings.
    fn warn(&self, message: &str) {
        let _ = message;
    }

    /// Token accounting for one completed AI API call.
    ///
    /// # Arguments
    ///
    /// * `usage` - The structured usage payload; implementations render or
    ///   aggregate it as they see fit.
    ///
    /// The terminal transport suppresses this channel in JSON output mode.
    fn ai_usage(&self, usage: &AiUsage) {
        let _ = usage;
    }

    /// An event on the long-running-work progress stream.
    ///
    /// # Arguments
    ///
    /// * `event` - The progress event; see [`ProgressEvent`] for coverage.
    ///
    /// The terminal transport suppresses this channel in JSON output mode
    /// **and** under `--quiet` — unlike diagnostics and warnings, progress
    /// chatter is exactly what `--quiet` exists to remove.
    fn progress(&self, event: &ProgressEvent<'_>) {
        let _ = event;
    }
}

/// The default [`Reporter`]: silently swallows everything.
///
/// Every attachable engine and client defaults to this, so consumers that
/// embed core as a library (e.g. the desktop GUI) never receive terminal
/// chatter they did not ask for.
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::{AiUsage, NoopReporter, ProgressEvent, Reporter};
///
/// let reporter = NoopReporter;
/// reporter.diagnostic("ignored");
/// reporter.warn("ignored");
/// reporter.ai_usage(&AiUsage {
///     model: "gpt-4.1-mini".to_string(),
///     prompt_tokens: 10,
///     completion_tokens: 5,
///     total_tokens: 15,
/// });
/// reporter.progress(&ProgressEvent::Message("ignored"));
/// ```
pub struct NoopReporter;

impl Reporter for NoopReporter {}

/// Shared handle to a [`NoopReporter`].
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::{ProgressEvent, Reporter, noop};
///
/// let reporter = noop();
/// // Compiles and does nothing.
/// reporter.progress(&ProgressEvent::Message("silence"));
/// ```
pub fn noop() -> Arc<dyn Reporter> {
    Arc::new(NoopReporter)
}

/// Token accounting for one completed AI API call.
///
/// The canonical usage payload for the [`Reporter::ai_usage`] channel.
/// `services::ai::AiUsageStats` is a legacy alias of this type.
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::AiUsage;
///
/// let usage = AiUsage {
///     model: "gpt-4.1-mini".to_string(),
///     prompt_tokens: 120,
///     completion_tokens: 45,
///     total_tokens: 165,
/// };
/// assert_eq!(usage.prompt_tokens + usage.completion_tokens, usage.total_tokens);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiUsage {
    /// Model identifier reported by the provider.
    pub model: String,
    /// Tokens billed for the prompt.
    pub prompt_tokens: u32,
    /// Tokens billed for the completion.
    pub completion_tokens: u32,
    /// Total tokens billed for the call.
    pub total_tokens: u32,
}

/// An event on the long-running-work progress stream.
///
/// Today this covers free-form status chatter emitted while long work
/// advances: worker-pool drain notices, per-batch translation progress,
/// and retry notices. The enum is `#[non_exhaustive]` so
/// `expose-core-orchestration-apis` (D2) can add structured variants
/// (started / advanced / finished, cancellation) without a breaking
/// change; every consumer `match` therefore needs a wildcard arm.
///
/// # Examples
///
/// ```
/// use subx_cli::core::report::ProgressEvent;
///
/// let event = ProgressEvent::Message("📊 Translation Progress:\n   Processed cues: 2/2");
/// match &event {
///     ProgressEvent::Message(message) => assert!(message.starts_with("📊")),
///     _ => {}
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent<'a> {
    /// Free-form status line emitted while long-running work advances.
    Message(&'a str),
}

/// `dyn Reporter` crosses thread boundaries inside engines; pin the bound.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<dyn Reporter>>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Per-test recording double (never global state — AGENTS.md).
    #[derive(Default)]
    struct RecordingReporter {
        events: Mutex<Vec<String>>,
    }

    impl RecordingReporter {
        fn recorded(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl Reporter for RecordingReporter {
        fn diagnostic(&self, message: &str) {
            self.events
                .lock()
                .unwrap()
                .push(format!("diagnostic:{message}"));
        }
        fn warn(&self, message: &str) {
            self.events.lock().unwrap().push(format!("warn:{message}"));
        }
        fn ai_usage(&self, usage: &AiUsage) {
            self.events.lock().unwrap().push(format!(
                "ai_usage:{}:{}:{}:{}",
                usage.model, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            ));
        }
        fn progress(&self, event: &ProgressEvent<'_>) {
            // Wildcard keeps the double compiling when non_exhaustive
            // variants land (statically unreachable today).
            #[allow(unreachable_patterns)]
            match event {
                ProgressEvent::Message(message) => {
                    self.events
                        .lock()
                        .unwrap()
                        .push(format!("progress:{message}"));
                }
                _ => self.events.lock().unwrap().push("progress:_".to_string()),
            }
        }
    }

    fn sample_usage() -> AiUsage {
        AiUsage {
            model: "gpt-4.1-mini".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        }
    }

    #[test]
    fn recording_reporter_captures_every_channel_verbatim() {
        let reporter = RecordingReporter::default();
        reporter.diagnostic("diag");
        reporter.warn("warn");
        reporter.ai_usage(&sample_usage());
        reporter.progress(&ProgressEvent::Message("tick"));

        assert_eq!(
            reporter.recorded(),
            vec![
                "diagnostic:diag",
                "warn:warn",
                "ai_usage:gpt-4.1-mini:100:50:150",
                "progress:tick",
            ],
            "each channel must receive exactly what core gave it"
        );
    }

    #[test]
    fn noop_reporter_swallows_all_channels() {
        // No observable output is possible; reaching the end without a
        // panic is the assertion.
        let reporter = noop();
        reporter.diagnostic("x");
        reporter.warn("x");
        reporter.ai_usage(&sample_usage());
        reporter.progress(&ProgressEvent::Message("x"));
    }

    #[test]
    fn default_trait_impl_is_all_noop() {
        // Implementors opting into no methods must compile via defaults.
        struct Silent;
        impl Reporter for Silent {}
        let silent = Silent;
        silent.diagnostic("x");
        silent.warn("x");
        silent.ai_usage(&sample_usage());
        silent.progress(&ProgressEvent::Message("x"));
    }

    #[test]
    fn progress_event_matches_with_wildcard_arm() {
        // Proves #[non_exhaustive] usability: exhaustive matching without
        // `_` is rejected across crate boundaries, so consumers carry `_`.
        // Inside the defining crate `_` is statically unreachable.
        let event = ProgressEvent::Message("status");
        #[allow(unreachable_patterns)]
        let text = match &event {
            ProgressEvent::Message(m) => *m,
            _ => "unknown",
        };
        assert_eq!(text, "status");
        assert_eq!(event, ProgressEvent::Message("status"));
    }

    #[test]
    fn ai_usage_round_trips_its_fields() {
        let usage = sample_usage();
        assert_eq!(usage.model, "gpt-4.1-mini");
        assert_eq!((usage.prompt_tokens, usage.completion_tokens), (100, 50));
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.clone(), usage);
    }

    /// A1 §7.2 — a `MatchEngine` with **no** reporter attached must emit
    /// zero stdout/stderr bytes on a path that used to print directly.
    ///
    /// In-process capture of `eprintln!` would require global-state hacks
    /// (forbidden), so the child process re-executes this test binary in a
    /// mode that runs the previously-printing analysis path through a
    /// default (no-op) reporter while the parent asserts the child's
    /// captured streams are empty.
    #[test]
    fn match_engine_without_reporter_prints_nothing() {
        if std::env::var_os("SUBX_TEST_NOOP_ENGINE_CHILD").is_some() {
            // Child: run the previously-printing path with no reporter.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let dir =
                    std::env::temp_dir().join(format!("subx-noop-child-{}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join("Movie.mp4"), b"v").unwrap();
                std::fs::write(
                    dir.join("movie.srt"),
                    "1\n00:00:01,000 --> 00:00:02,000\nhello\n\n",
                )
                .unwrap();
                let files: Vec<std::path::PathBuf> =
                    vec![dir.join("Movie.mp4"), dir.join("movie.srt")];
                use crate::core::matcher::engine::{ConflictResolution, FileRelocationMode};
                let engine = crate::core::matcher::engine::MatchEngine::new(
                    Box::new(NoopProvider),
                    crate::core::matcher::engine::MatchConfig {
                        confidence_threshold: 0.8,
                        max_sample_length: 2000,
                        enable_content_analysis: false,
                        backup_enabled: false,
                        relocation_mode: FileRelocationMode::None,
                        conflict_resolution: ConflictResolution::Skip,
                        ai_model: "noop".to_string(),
                        max_subtitle_bytes: 52_428_800,
                    },
                );
                // match_file_list previously printed the AI analysis
                // block, the available-files dump and the no-matches
                // dump directly to the terminal.
                let _ = engine.match_file_list(&files).await;
                let _ = std::fs::remove_dir_all(&dir);
            });
            return;
        }

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("core::report::tests::match_engine_without_reporter_prints_nothing")
            .arg("--exact")
            .env("SUBX_TEST_NOOP_ENGINE_CHILD", "1")
            .env("RUST_TEST_THREADS", "1")
            .output()
            .expect("re-exec test binary");
        assert!(
            output.status.success(),
            "child test failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        // The child's own harness may print its status lines; the engine
        // must add nothing: no engine chatter markers at all.
        for marker in [
            "🔍",
            "❌",
            "Available",
            "AI Analysis Results",
            "No matching files found",
        ] {
            assert!(
                !stdout.contains(marker) && !stderr.contains(marker),
                "reporter-less MatchEngine leaked {marker:?} — stdout:\n{stdout}\nstderr:\n{stderr}"
            );
        }
    }

    /// AI provider stub: returns zero matches so the previously-printing
    /// no-matches path runs without needing a network or mock server.
    struct NoopProvider;

    #[async_trait::async_trait]
    impl crate::services::ai::AIProvider for NoopProvider {
        async fn analyze_content(
            &self,
            _request: crate::services::ai::AnalysisRequest,
        ) -> crate::Result<crate::services::ai::MatchResult> {
            Ok(crate::services::ai::MatchResult {
                matches: Vec::new(),
                confidence: 0.0,
                reasoning: "noop".to_string(),
            })
        }

        async fn verify_match(
            &self,
            _verification: crate::services::ai::VerificationRequest,
        ) -> crate::Result<crate::services::ai::ConfidenceScore> {
            unimplemented!("unused by the noop-engine test")
        }
    }
}
