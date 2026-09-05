//! Binary-owned presentation extensions for [`SubXError`].
//!
//! The `SubXError` taxonomy itself lives in [`crate::error`] and ships with
//! the library half of the split (`subx-core`): `category()`, `machine_code()`
//! and `hint()` are inherent methods there because machine-readable front
//! ends (e.g. the Tauri GUI) consume them directly.
//!
//! Process exit codes and multi-line terminal prose with `Hint:` lines,
//! however, are properties of running the `subx-cli` binary — a library
//! consumer embeds `SubXError` in its own presentation layer and has neither
//! a process exit code nor a terminal. Those two operations therefore live
//! here, as an extension trait owned by the binary:
//!
//! - [`SubXErrorExt::exit_code`] — the stable 1–6 process exit mapping.
//! - [`SubXErrorExt::user_friendly_message`] — the multi-line, hinted,
//!   terminal-facing message.
//!
//! Code under `src/core/` and `src/services/` SHALL NOT import this trait or
//! call either method; core code that needs a rendered message uses
//! `Display` (`to_string()`), optionally combined with `hint()`.

use crate::error::SubXError;

/// Presentation-layer extensions to [`SubXError`] owned by the binary.
pub trait SubXErrorExt {
    /// Return the corresponding exit code for this error variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// # use subx_cli::cli::SubXErrorExt;
    /// assert_eq!(SubXError::config("x").exit_code(), 2);
    /// ```
    fn exit_code(&self) -> i32;

    /// Return a user-friendly error message with suggested remedies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// # use subx_cli::cli::SubXErrorExt;
    /// let msg = SubXError::config("missing key").user_friendly_message();
    /// assert!(msg.contains("Configuration error:"));
    /// ```
    fn user_friendly_message(&self) -> String;
}

impl SubXErrorExt for SubXError {
    fn exit_code(&self) -> i32 {
        match self {
            SubXError::Io(_) => 1,
            SubXError::Config { .. } => 2,
            SubXError::Api { .. } => 3,
            SubXError::AiService(_) => 3,
            SubXError::SubtitleFormat { .. } => 4,
            SubXError::AudioProcessing { .. } => 5,
            SubXError::FileMatching { .. } => 6,
            _ => 1,
        }
    }

    fn user_friendly_message(&self) -> String {
        match self {
            SubXError::Io(e) => format!("File operation error: {}", e),
            SubXError::Config { message } => format!(
                "Configuration error: {}\nHint: run 'subx-cli config --help' for details",
                message
            ),
            SubXError::Api { message, source } => format!(
                "API error ({:?}): {}\nHint: check network connection and API key settings",
                source, message
            ),
            SubXError::AiService(msg) => format!(
                "AI service error: {}\nHint: check network connection and API key settings",
                msg
            ),
            SubXError::SubtitleFormat { message, .. } => format!(
                "Subtitle processing error: {}\nHint: check file format and encoding",
                message
            ),
            SubXError::AudioProcessing { message } => format!(
                "Audio processing error: {}\nHint: ensure media file integrity and support",
                message
            ),
            SubXError::FileMatching { message } => format!(
                "File matching error: {}\nHint: verify file paths and patterns",
                message
            ),
            SubXError::FileAlreadyExists(path) => format!("File already exists: {}", path),
            SubXError::FileNotFound(path) => format!("File not found: {}", path),
            SubXError::InvalidFileName(name) => format!("Invalid file name: {}", name),
            SubXError::FileOperationFailed(msg) => format!("File operation failed: {}", msg),
            SubXError::CommandExecution(msg) => msg.clone(),
            SubXError::OutputModeUnsupported { command } => format!(
                "The '{}' command does not support --output json; its stdout is a shell-completion script.\nHint: rerun without --output json (and ensure SUBX_OUTPUT is unset)",
                command
            ),
            SubXError::Other(err) => {
                format!("Unknown error: {}\nHint: please report this issue", err)
            }
            _ => format!("Error: {}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ApiErrorSource;
    use std::io;
    use std::path::PathBuf;

    // ── exit_code mapping ─────────────────────────────────────────────────────
    //
    // These tests moved here with the two presentation methods (change
    // relocate-misplaced-core-modules, Decision 4): `exit_code()` and
    // `user_friendly_message()` are trait methods now, so their coverage
    // lives with the trait.

    #[test]
    fn test_exit_codes() {
        assert_eq!(SubXError::config("test").exit_code(), 2);
        assert_eq!(SubXError::subtitle_format("SRT", "test").exit_code(), 4);
        assert_eq!(SubXError::audio_processing("test").exit_code(), 5);
        assert_eq!(SubXError::file_matching("test").exit_code(), 6);
    }

    #[test]
    fn test_user_friendly_messages() {
        let config_error = SubXError::config("missing key");
        let message = config_error.user_friendly_message();
        assert!(message.contains("Configuration error:"));
        assert!(message.contains("subx-cli config --help"));

        let ai_error = SubXError::ai_service("network failure".to_string());
        let message = ai_error.user_friendly_message();
        assert!(message.contains("AI service error:"));
        assert!(message.contains("check network connection"));
    }

    /// Audit: enumerates every `SubXError` variant and asserts that a
    /// representative instance — built from non-sensitive dummy data —
    /// never surfaces an OpenAI-style API key prefix (`sk-`) through
    /// `Display`, `Debug`, or `user_friendly_message()`. If you add a
    /// new variant, extend this list so the audit remains exhaustive.
    ///
    /// Separately, this test also exercises the sanitizing construction
    /// paths (`From<reqwest::Error>`-style flows via the AI client's
    /// `error_sanitizer` helpers) to confirm that when input *does*
    /// contain an `sk-*` secret, it is stripped before wrapping it in
    /// `SubXError::AiService`.
    #[test]
    fn test_no_api_key_leaks_in_any_variant() {
        use crate::services::ai::error_sanitizer::{
            DEFAULT_ERROR_BODY_MAX_LEN, sanitize_url_in_error, truncate_error_body,
        };
        use std::path::PathBuf;

        // 1. Canonical variant audit: benign dummy data must never yield
        //    an `sk-` substring.
        let variants: Vec<SubXError> = vec![
            SubXError::Io(io::Error::other("disk error")),
            SubXError::Config {
                message: "missing key".to_string(),
            },
            SubXError::SubtitleFormat {
                format: "SRT".to_string(),
                message: "bad timestamp".to_string(),
            },
            SubXError::AiService("upstream service failed".to_string()),
            SubXError::Api {
                message: "auth failed".to_string(),
                source: ApiErrorSource::OpenAI,
            },
            SubXError::AudioProcessing {
                message: "codec failure".to_string(),
            },
            SubXError::FileMatching {
                message: "pattern mismatch".to_string(),
            },
            SubXError::FileAlreadyExists("/tmp/example".to_string()),
            SubXError::FileNotFound("/tmp/example".to_string()),
            SubXError::InvalidFileName("bad?name".to_string()),
            SubXError::FileOperationFailed("rename failed".to_string()),
            SubXError::CommandExecution("exit 1".to_string()),
            SubXError::NoInputSpecified,
            SubXError::InvalidPath(PathBuf::from("/tmp/example")),
            SubXError::PathNotFound(PathBuf::from("/tmp/example")),
            SubXError::DirectoryReadError {
                path: PathBuf::from("/tmp/example"),
                source: io::Error::other("denied"),
            },
            SubXError::InvalidSyncConfiguration,
            SubXError::UnsupportedFileType("xyz".to_string()),
            SubXError::OutputModeUnsupported {
                command: "generate-completion".to_string(),
            },
            SubXError::Other(anyhow::anyhow!("wrapped")),
        ];

        for err in &variants {
            let display = format!("{}", err);
            let debug = format!("{:?}", err);
            let friendly = err.user_friendly_message();
            for (label, text) in [
                ("Display", &display),
                ("Debug", &debug),
                ("friendly", &friendly),
            ] {
                assert!(
                    !text.contains("sk-"),
                    "{} surface for variant {:?} contains `sk-` prefix: {}",
                    label,
                    err,
                    text
                );
            }
        }

        // 2. Sanitizing construction paths: API keys injected via the
        //    upstream response body or URL query string must be stripped
        //    before being embedded into `SubXError::AiService`.
        const SECRET: &str = "sk-test-key-12345";
        let upstream_body = format!(
            "{{\"error\": \"invalid\", \"echoed\": \"Bearer {}\"}}",
            SECRET
        );
        let truncated = truncate_error_body(&upstream_body, DEFAULT_ERROR_BODY_MAX_LEN);
        // Helper does not itself mask secrets shorter than the limit; this
        // documents that short bodies pass through unchanged so upstream
        // callers must continue to keep secrets out of request bodies.
        assert!(truncated.contains(SECRET));

        let url_leak = format!(
            "request error: https://api.example.com/v1/chat?api-key={}",
            SECRET
        );
        let cleaned = sanitize_url_in_error(&url_leak);
        assert!(!cleaned.contains("sk-test-key"));
        let wrapped = SubXError::AiService(cleaned);
        assert!(!format!("{}", wrapped).contains("sk-test-key"));
        assert!(!format!("{:?}", wrapped).contains("sk-test-key"));
    }

    // ── exit_code – remaining variants ───────────────────────────────────────

    #[test]
    fn test_exit_code_io() {
        let err = SubXError::Io(io::Error::new(io::ErrorKind::NotFound, "x"));
        assert_eq!(err.exit_code(), 1);
    }

    #[test]
    fn test_exit_code_api() {
        let err = SubXError::Api {
            message: "x".to_string(),
            source: ApiErrorSource::OpenAI,
        };
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_exit_code_ai_service() {
        let err = SubXError::AiService("x".to_string());
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn test_exit_code_catchall_variants() {
        assert_eq!(SubXError::FileAlreadyExists("f".to_string()).exit_code(), 1);
        assert_eq!(SubXError::FileNotFound("f".to_string()).exit_code(), 1);
        assert_eq!(SubXError::InvalidFileName("f".to_string()).exit_code(), 1);
        assert_eq!(
            SubXError::FileOperationFailed("f".to_string()).exit_code(),
            1
        );
        assert_eq!(SubXError::CommandExecution("f".to_string()).exit_code(), 1);
        assert_eq!(SubXError::NoInputSpecified.exit_code(), 1);
        assert_eq!(SubXError::InvalidPath(PathBuf::from("/x")).exit_code(), 1);
        assert_eq!(SubXError::PathNotFound(PathBuf::from("/x")).exit_code(), 1);
        assert_eq!(SubXError::InvalidSyncConfiguration.exit_code(), 1);
        assert_eq!(
            SubXError::UnsupportedFileType("xyz".to_string()).exit_code(),
            1
        );
        assert_eq!(SubXError::Other(anyhow::anyhow!("other")).exit_code(), 1);
    }

    // ── category / machine_code / exit_code contract ────────────────────────

    /// Exhaustive contract test for the closed `SubXError` mapping locked
    /// by `specs/error-handling/spec.md`. If a new variant is added, this
    /// test (and the exhaustive matches in `category()`/`machine_code()`)
    /// SHALL be updated; the compiler-enforced exhaustive match guards
    /// the source of truth. It lives here (not in `src/error.rs`) because
    /// the `exit_code` column is a `SubXErrorExt` method.
    #[test]
    fn test_category_and_machine_code_contract() {
        let cases: Vec<(SubXError, &'static str, &'static str, i32)> = vec![
            (SubXError::Io(io::Error::other("x")), "io", "E_IO", 1),
            (
                SubXError::Config {
                    message: "x".into(),
                },
                "config",
                "E_CONFIG",
                2,
            ),
            (
                SubXError::SubtitleFormat {
                    format: "SRT".into(),
                    message: "x".into(),
                },
                "subtitle_format",
                "E_SUBTITLE_FORMAT",
                4,
            ),
            (
                SubXError::AiService("x".into()),
                "ai_service",
                "E_AI_SERVICE",
                3,
            ),
            (
                SubXError::Api {
                    message: "x".into(),
                    source: ApiErrorSource::OpenAI,
                },
                "api",
                "E_API",
                3,
            ),
            (
                SubXError::AudioProcessing {
                    message: "x".into(),
                },
                "audio_processing",
                "E_AUDIO_PROCESSING",
                5,
            ),
            (
                SubXError::FileMatching {
                    message: "x".into(),
                },
                "file_matching",
                "E_FILE_MATCHING",
                6,
            ),
            (
                SubXError::FileAlreadyExists("x".into()),
                "file_already_exists",
                "E_FILE_ALREADY_EXISTS",
                1,
            ),
            (
                SubXError::FileNotFound("x".into()),
                "file_not_found",
                "E_FILE_NOT_FOUND",
                1,
            ),
            (
                SubXError::InvalidFileName("x".into()),
                "invalid_file_name",
                "E_INVALID_FILE_NAME",
                1,
            ),
            (
                SubXError::FileOperationFailed("x".into()),
                "file_operation_failed",
                "E_FILE_OPERATION_FAILED",
                1,
            ),
            (
                SubXError::CommandExecution("x".into()),
                "command_execution",
                "E_COMMAND_EXECUTION",
                1,
            ),
            (
                SubXError::OutputModeUnsupported {
                    command: "generate-completion".into(),
                },
                "command_execution",
                "E_OUTPUT_MODE_UNSUPPORTED",
                1,
            ),
            (
                SubXError::NoInputSpecified,
                "no_input_specified",
                "E_NO_INPUT_SPECIFIED",
                1,
            ),
            (
                SubXError::InvalidPath(PathBuf::from("/x")),
                "invalid_path",
                "E_INVALID_PATH",
                1,
            ),
            (
                SubXError::PathNotFound(PathBuf::from("/x")),
                "path_not_found",
                "E_PATH_NOT_FOUND",
                1,
            ),
            (
                SubXError::DirectoryReadError {
                    path: PathBuf::from("/x"),
                    source: io::Error::other("denied"),
                },
                "directory_read_error",
                "E_DIRECTORY_READ_ERROR",
                1,
            ),
            (
                SubXError::InvalidSyncConfiguration,
                "invalid_sync_configuration",
                "E_INVALID_SYNC_CONFIGURATION",
                1,
            ),
            (
                SubXError::UnsupportedFileType("xyz".into()),
                "unsupported_file_type",
                "E_UNSUPPORTED_FILE_TYPE",
                1,
            ),
            (
                SubXError::Other(anyhow::anyhow!("x")),
                "other",
                "E_OTHER",
                1,
            ),
        ];

        for (err, cat, code, exit) in &cases {
            assert_eq!(err.category(), *cat, "category mismatch for {:?}", err);
            assert_eq!(
                err.machine_code(),
                *code,
                "machine_code mismatch for {:?}",
                err
            );
            assert_eq!(err.exit_code(), *exit, "exit_code mismatch for {:?}", err);
            assert!(!err.category().is_empty());
            assert!(err.machine_code().starts_with("E_"));
        }
    }

    // ── user_friendly_message – all variants ─────────────────────────────────

    #[test]
    fn test_user_friendly_message_io() {
        let err = SubXError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        let msg = err.user_friendly_message();
        assert!(msg.contains("File operation error:"));
        assert!(msg.contains("denied"));
    }

    #[test]
    fn test_user_friendly_message_api() {
        let err = SubXError::Api {
            message: "forbidden".to_string(),
            source: ApiErrorSource::OpenAI,
        };
        let msg = err.user_friendly_message();
        assert!(msg.contains("API error"));
        assert!(msg.contains("forbidden"));
        assert!(msg.contains("check network connection"));
    }

    #[test]
    fn test_user_friendly_message_subtitle_format() {
        let err = SubXError::subtitle_format("ASS", "bad encoding");
        let msg = err.user_friendly_message();
        assert!(msg.contains("Subtitle processing error:"));
        assert!(msg.contains("bad encoding"));
        assert!(msg.contains("check file format"));
    }

    #[test]
    fn test_user_friendly_message_audio_processing() {
        let err = SubXError::audio_processing("corrupt frame");
        let msg = err.user_friendly_message();
        assert!(msg.contains("Audio processing error:"));
        assert!(msg.contains("corrupt frame"));
        assert!(msg.contains("media file integrity"));
    }

    #[test]
    fn test_user_friendly_message_file_matching() {
        let err = SubXError::file_matching("pattern mismatch");
        let msg = err.user_friendly_message();
        assert!(msg.contains("File matching error:"));
        assert!(msg.contains("pattern mismatch"));
        assert!(msg.contains("verify file paths"));
    }

    #[test]
    fn test_user_friendly_message_file_already_exists() {
        let err = SubXError::FileAlreadyExists("output.srt".to_string());
        assert_eq!(
            err.user_friendly_message(),
            "File already exists: output.srt"
        );
    }

    #[test]
    fn test_user_friendly_message_file_not_found() {
        let err = SubXError::FileNotFound("input.srt".to_string());
        assert_eq!(err.user_friendly_message(), "File not found: input.srt");
    }

    #[test]
    fn test_user_friendly_message_invalid_file_name() {
        let err = SubXError::InvalidFileName("bad?name".to_string());
        assert_eq!(err.user_friendly_message(), "Invalid file name: bad?name");
    }

    #[test]
    fn test_user_friendly_message_file_operation_failed() {
        let err = SubXError::FileOperationFailed("rename failed".to_string());
        assert_eq!(
            err.user_friendly_message(),
            "File operation failed: rename failed"
        );
    }

    #[test]
    fn test_user_friendly_message_command_execution() {
        let err = SubXError::CommandExecution("process died".to_string());
        assert_eq!(err.user_friendly_message(), "process died");
    }

    #[test]
    fn test_user_friendly_message_other() {
        let err = SubXError::Other(anyhow::anyhow!("mystery"));
        let msg = err.user_friendly_message();
        assert!(msg.contains("Unknown error:"));
        assert!(msg.contains("mystery"));
        assert!(msg.contains("please report this issue"));
    }

    #[test]
    fn test_user_friendly_message_catchall_variants() {
        // Variants that fall through to the `_ => format!("Error: {}", self)` arm.
        let cases: Vec<SubXError> = vec![
            SubXError::NoInputSpecified,
            SubXError::InvalidPath(PathBuf::from("/bad")),
            SubXError::PathNotFound(PathBuf::from("/missing")),
            SubXError::DirectoryReadError {
                path: PathBuf::from("/locked"),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
            },
            SubXError::InvalidSyncConfiguration,
            SubXError::UnsupportedFileType("xyz".to_string()),
        ];
        for err in &cases {
            let msg = err.user_friendly_message();
            assert!(
                msg.starts_with("Error:"),
                "Expected 'Error:' prefix for {:?}, got: {}",
                err,
                msg
            );
        }
    }

    // ── Display == user_friendly_message lock (task 5.4) ────────────────────

    /// Locks the invariant that `core::matcher::engine::operation_error_from`
    /// relies on when it renders `OperationError::message` through `Display`
    /// instead of calling the (binary-side) `user_friendly_message()`: the
    /// only variant that reaches that function — `FileOperationFailed` —
    /// renders identically through both paths, and carries no `Hint:` line
    /// that `Display` could lose. Widening the set of variants reaching
    /// `operation_error_from` without re-checking this equality silently
    /// changes the JSON per-item `error.message` contract.
    #[test]
    fn file_operation_failed_display_equals_user_friendly_message() {
        let err = SubXError::FileOperationFailed("could not rename".into());
        assert_eq!(err.to_string(), err.user_friendly_message());
        assert!(err.hint().is_none());
    }
}
