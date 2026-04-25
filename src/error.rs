//! Comprehensive error types for the SubX CLI application operations.
//!
//! This module defines the `SubXError` enum covering all error conditions
//! that can occur during subtitle processing, AI service integration,
//! audio analysis, file matching, and general command execution.
//!
//! It also provides helper methods to construct errors and generate
//! user-friendly messages.
use thiserror::Error;

/// Represents all possible errors in the SubX application.
///
/// Each variant provides specific context to facilitate debugging and
/// user-friendly reporting.
///
/// # Examples
///
/// ```rust
/// use subx_cli::error::{SubXError, SubXResult};
///
/// fn example() -> SubXResult<()> {
///     Err(SubXError::SubtitleFormat {
///         format: "SRT".to_string(),
///         message: "Invalid timestamp format".to_string(),
///     })
/// }
/// ```
///
/// # Exit Codes
///
/// Each error variant maps to an exit code via `SubXError::exit_code`.
#[derive(Error, Debug)]
pub enum SubXError {
    /// I/O operation failed during file system access.
    ///
    /// This variant wraps `std::io::Error` and provides context about
    /// file operations that failed.
    ///
    /// # Common Causes
    /// - Permission issues
    /// - Insufficient disk space
    /// - Network filesystem errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error due to invalid or missing settings.
    ///
    /// Contains a human-readable message describing the issue.
    #[error("Configuration error: {message}")]
    Config {
        /// Description of the configuration error
        message: String,
    },

    /// Subtitle format error indicating invalid timestamps or structure.
    ///
    /// Provides the subtitle format and detailed message.
    #[error("Subtitle format error [{format}]: {message}")]
    SubtitleFormat {
        /// The subtitle format that caused the error (e.g., "SRT", "ASS")
        format: String,
        /// Detailed error message describing the issue
        message: String,
    },

    /// AI service encountered an error.
    ///
    /// Captures the raw error message from the AI provider.
    #[error("AI service error: {0}")]
    AiService(String),

    /// API request error with specified source.
    ///
    /// Represents errors that occur during API requests, providing both
    /// the error message and the source of the API error.
    #[error("API error [{source:?}]: {message}")]
    Api {
        /// Error message from the API
        message: String,
        /// Source of the API error
        source: ApiErrorSource,
    },

    /// Audio processing error during analysis or format conversion.
    ///
    /// Provides a message describing the audio processing failure.
    #[error("Audio processing error: {message}")]
    AudioProcessing {
        /// Description of the audio processing error
        message: String,
    },

    /// Error during file matching or discovery.
    ///
    /// Contains details about path resolution or pattern matching failures.
    #[error("File matching error: {message}")]
    FileMatching {
        /// Description of the file matching error
        message: String,
    },
    /// Indicates that a file operation failed because the target exists.
    #[error("File already exists: {0}")]
    FileAlreadyExists(String),
    /// Indicates that the specified file was not found.
    #[error("File not found: {0}")]
    FileNotFound(String),
    /// Invalid file name encountered.
    #[error("Invalid file name: {0}")]
    InvalidFileName(String),
    /// Generic file operation failure with message.
    #[error("File operation failed: {0}")]
    FileOperationFailed(String),
    /// Generic command execution error.
    #[error("{0}")]
    CommandExecution(String),

    /// No input path was specified for the operation.
    #[error("No input path specified")]
    NoInputSpecified,

    /// The provided path is invalid or malformed.
    #[error("Invalid path: {0}")]
    InvalidPath(std::path::PathBuf),

    /// The specified path does not exist on the filesystem.
    #[error("Path not found: {0}")]
    PathNotFound(std::path::PathBuf),

    /// Unable to read the specified directory.
    #[error("Unable to read directory: {path}")]
    DirectoryReadError {
        /// The directory path that could not be read
        path: std::path::PathBuf,
        /// The underlying I/O error
        #[source]
        source: std::io::Error,
    },

    /// Invalid synchronization configuration: please specify video and subtitle files, or use -i parameter for batch processing.
    #[error(
        "Invalid sync configuration: please specify video and subtitle files, or use -i parameter for batch processing"
    )]
    InvalidSyncConfiguration,

    /// Unsupported file type encountered.
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    /// Catch-all error variant wrapping any other failure.
    #[error("Unknown error: {0}")]
    Other(#[from] anyhow::Error),
}

// Unit test: SubXError error types and helper methods
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    // ── Display messages ──────────────────────────────────────────────────────

    #[test]
    fn test_io_error_display() {
        let err = SubXError::Io(io::Error::new(io::ErrorKind::PermissionDenied, "denied"));
        assert!(err.to_string().starts_with("I/O error:"));
        assert!(err.to_string().contains("denied"));
    }

    #[test]
    fn test_ai_service_display() {
        let err = SubXError::AiService("timeout".to_string());
        assert_eq!(err.to_string(), "AI service error: timeout");
    }

    #[test]
    fn test_api_display() {
        let err = SubXError::Api {
            message: "bad request".to_string(),
            source: ApiErrorSource::OpenAI,
        };
        let s = err.to_string();
        assert!(s.contains("API error"));
        assert!(s.contains("bad request"));
        assert!(s.contains("OpenAI"));
    }

    #[test]
    fn test_file_already_exists_display() {
        let err = SubXError::FileAlreadyExists("foo.srt".to_string());
        assert_eq!(err.to_string(), "File already exists: foo.srt");
    }

    #[test]
    fn test_file_not_found_display() {
        let err = SubXError::FileNotFound("bar.srt".to_string());
        assert_eq!(err.to_string(), "File not found: bar.srt");
    }

    #[test]
    fn test_invalid_file_name_display() {
        let err = SubXError::InvalidFileName("bad?name".to_string());
        assert_eq!(err.to_string(), "Invalid file name: bad?name");
    }

    #[test]
    fn test_file_operation_failed_display() {
        let err = SubXError::FileOperationFailed("rename failed".to_string());
        assert_eq!(err.to_string(), "File operation failed: rename failed");
    }

    #[test]
    fn test_command_execution_display() {
        let err = SubXError::CommandExecution("exit 1".to_string());
        assert_eq!(err.to_string(), "exit 1");
    }

    #[test]
    fn test_no_input_specified_display() {
        let err = SubXError::NoInputSpecified;
        assert_eq!(err.to_string(), "No input path specified");
    }

    #[test]
    fn test_invalid_path_display() {
        let err = SubXError::InvalidPath(PathBuf::from("/bad/path"));
        assert!(err.to_string().contains("Invalid path:"));
        assert!(err.to_string().contains("/bad/path"));
    }

    #[test]
    fn test_path_not_found_display() {
        let err = SubXError::PathNotFound(PathBuf::from("/missing"));
        assert!(err.to_string().contains("Path not found:"));
    }

    #[test]
    fn test_directory_read_error_display() {
        let err = SubXError::DirectoryReadError {
            path: PathBuf::from("/locked"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(err.to_string().contains("Unable to read directory:"));
        assert!(err.to_string().contains("/locked"));
    }

    #[test]
    fn test_invalid_sync_configuration_display() {
        let err = SubXError::InvalidSyncConfiguration;
        assert!(err.to_string().contains("Invalid sync configuration"));
    }

    #[test]
    fn test_unsupported_file_type_display() {
        let err = SubXError::UnsupportedFileType("xyz".to_string());
        assert_eq!(err.to_string(), "Unsupported file type: xyz");
    }

    #[test]
    fn test_other_error_display() {
        let err = SubXError::Other(anyhow::anyhow!("wrapped error"));
        assert!(err.to_string().contains("Unknown error:"));
        assert!(err.to_string().contains("wrapped error"));
    }

    // ── ApiErrorSource ────────────────────────────────────────────────────────

    #[test]
    fn test_api_error_source_display() {
        assert_eq!(ApiErrorSource::OpenAI.to_string(), "OpenAI");
        assert_eq!(ApiErrorSource::Whisper.to_string(), "Whisper");
    }

    // ── exit_code mapping ─────────────────────────────────────────────────────

    #[test]
    fn test_config_error_creation() {
        let error = SubXError::config("test config error");
        assert!(matches!(error, SubXError::Config { .. }));
        assert_eq!(error.to_string(), "Configuration error: test config error");
    }

    #[test]
    fn test_subtitle_format_error_creation() {
        let error = SubXError::subtitle_format("SRT", "invalid format");
        assert!(matches!(error, SubXError::SubtitleFormat { .. }));
        let msg = error.to_string();
        assert!(msg.contains("SRT"));
        assert!(msg.contains("invalid format"));
    }

    #[test]
    fn test_audio_processing_error_creation() {
        let error = SubXError::audio_processing("decode failed");
        assert!(matches!(error, SubXError::AudioProcessing { .. }));
        assert_eq!(error.to_string(), "Audio processing error: decode failed");
    }

    #[test]
    fn test_file_matching_error_creation() {
        let error = SubXError::file_matching("match failed");
        assert!(matches!(error, SubXError::FileMatching { .. }));
        assert_eq!(error.to_string(), "File matching error: match failed");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let subx_error: SubXError = io_error.into();
        assert!(matches!(subx_error, SubXError::Io(_)));
    }

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
                    text,
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

    // ── Helper constructor methods ────────────────────────────────────────────

    #[test]
    fn test_ai_service_helper() {
        let err = SubXError::ai_service("network failure");
        assert!(matches!(err, SubXError::AiService(_)));
        assert_eq!(err.to_string(), "AI service error: network failure");
    }

    #[test]
    fn test_parallel_processing_helper() {
        let err = SubXError::parallel_processing("channel closed".to_string());
        assert!(matches!(err, SubXError::CommandExecution(_)));
        assert!(err.to_string().contains("Parallel processing error:"));
        assert!(err.to_string().contains("channel closed"));
    }

    #[test]
    fn test_task_execution_failed_helper() {
        let err = SubXError::task_execution_failed("task-42".to_string(), "panic".to_string());
        assert!(matches!(err, SubXError::CommandExecution(_)));
        assert!(err.to_string().contains("task-42"));
        assert!(err.to_string().contains("panic"));
    }

    #[test]
    fn test_worker_pool_exhausted_helper() {
        let err = SubXError::worker_pool_exhausted();
        assert!(matches!(err, SubXError::CommandExecution(_)));
        assert_eq!(err.to_string(), "Worker pool exhausted");
    }

    #[test]
    fn test_task_timeout_helper() {
        let dur = std::time::Duration::from_secs(30);
        let err = SubXError::task_timeout("task-7".to_string(), dur);
        assert!(matches!(err, SubXError::CommandExecution(_)));
        assert!(err.to_string().contains("task-7"));
        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_dialogue_detection_failed_helper() {
        let err = SubXError::dialogue_detection_failed("no speech found");
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("Dialogue detection failed:"));
        assert!(err.to_string().contains("no speech found"));
    }

    #[test]
    fn test_invalid_audio_format_helper() {
        let err = SubXError::invalid_audio_format("flac");
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("Unsupported audio format:"));
        assert!(err.to_string().contains("flac"));
    }

    #[test]
    fn test_dialogue_segment_invalid_helper() {
        let err = SubXError::dialogue_segment_invalid("negative duration");
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("Invalid dialogue segment:"));
        assert!(err.to_string().contains("negative duration"));
    }

    #[test]
    fn test_whisper_api_helper() {
        let err = SubXError::whisper_api("rate limited");
        assert!(matches!(err, SubXError::Api { .. }));
        let s = err.to_string();
        assert!(s.contains("Whisper"));
        assert!(s.contains("rate limited"));
    }

    #[test]
    fn test_audio_extraction_helper() {
        let err = SubXError::audio_extraction("ffmpeg missing");
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("ffmpeg missing"));
    }

    // ── From conversions ─────────────────────────────────────────────────────

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("some anyhow error");
        let err: SubXError = anyhow_err.into();
        assert!(matches!(err, SubXError::Other(_)));
        assert!(err.to_string().contains("some anyhow error"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err: serde_json::Error =
            serde_json::from_str::<serde_json::Value>("not json {{{").unwrap_err();
        let err: SubXError = json_err.into();
        assert!(matches!(err, SubXError::Config { .. }));
        assert!(
            err.to_string()
                .contains("JSON serialization/deserialization error:")
        );
    }

    #[test]
    fn test_from_config_error_not_found() {
        let config_err = config::ConfigError::NotFound("settings.toml".to_string());
        let err: SubXError = config_err.into();
        assert!(matches!(err, SubXError::Config { .. }));
        assert!(err.to_string().contains("Configuration file not found:"));
        assert!(err.to_string().contains("settings.toml"));
    }

    #[test]
    fn test_from_config_error_message() {
        let config_err = config::ConfigError::Message("bad value".to_string());
        let err: SubXError = config_err.into();
        assert!(matches!(err, SubXError::Config { .. }));
        assert!(err.to_string().contains("bad value"));
    }

    #[test]
    fn test_from_config_error_other() {
        // Use a variant that falls through to the catch-all arm.
        let config_err = config::ConfigError::Foreign(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "foreign cfg error",
        )));
        let err: SubXError = config_err.into();
        assert!(matches!(err, SubXError::Config { .. }));
        assert!(err.to_string().contains("Configuration error:"));
    }

    #[test]
    fn test_from_box_dyn_error() {
        let boxed: Box<dyn std::error::Error> =
            Box::new(io::Error::new(io::ErrorKind::Other, "boxed error"));
        let err: SubXError = boxed.into();
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("Audio processing error:"));
        assert!(err.to_string().contains("boxed error"));
    }

    #[test]
    fn test_from_walkdir_error() {
        // Walk a non-existent path to generate a walkdir::Error.
        let walk_err = walkdir::WalkDir::new("/nonexistent_subx_test_path_xyz")
            .into_iter()
            .filter_map(|e| e.err())
            .next();
        if let Some(we) = walk_err {
            let err: SubXError = we.into();
            assert!(matches!(err, SubXError::FileMatching { .. }));
        }
        // If no error was produced (unlikely), the From impl was never
        // reached, but the test still passes: we cannot force the error.
    }

    #[test]
    fn test_from_symphonia_error() {
        use symphonia::core::errors::Error as SymphoniaError;
        let sym_err = SymphoniaError::DecodeError("bad frame");
        let err: SubXError = sym_err.into();
        assert!(matches!(err, SubXError::AudioProcessing { .. }));
        assert!(err.to_string().contains("Audio processing error:"));
    }

    // ── SubXResult type alias ─────────────────────────────────────────────────

    #[test]
    fn test_subx_result_ok() {
        let result: SubXResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_subx_result_err() {
        let result: SubXResult<i32> = Err(SubXError::NoInputSpecified);
        assert!(result.is_err());
    }
}

// Convert reqwest error to AI service error
impl From<reqwest::Error> for SubXError {
    fn from(err: reqwest::Error) -> Self {
        let raw = err.to_string();
        // Strip query strings from any embedded URLs, since reqwest's Display
        // implementation includes the full request URL which may carry
        // sensitive credentials (e.g. `?api-key=...`).
        let sanitized = crate::services::ai::error_sanitizer::sanitize_url_in_error(&raw);
        SubXError::AiService(sanitized)
    }
}

// Convert file exploration error to file matching error
impl From<walkdir::Error> for SubXError {
    fn from(err: walkdir::Error) -> Self {
        SubXError::FileMatching {
            message: err.to_string(),
        }
    }
}
// Convert symphonia error to audio processing error
impl From<symphonia::core::errors::Error> for SubXError {
    fn from(err: symphonia::core::errors::Error) -> Self {
        SubXError::audio_processing(err.to_string())
    }
}

// Convert config crate error to configuration error
impl From<config::ConfigError> for SubXError {
    fn from(err: config::ConfigError) -> Self {
        match err {
            config::ConfigError::NotFound(path) => SubXError::Config {
                message: format!("Configuration file not found: {}", path),
            },
            config::ConfigError::Message(msg) => SubXError::Config { message: msg },
            _ => SubXError::Config {
                message: format!("Configuration error: {}", err),
            },
        }
    }
}

impl From<serde_json::Error> for SubXError {
    fn from(err: serde_json::Error) -> Self {
        SubXError::Config {
            message: format!("JSON serialization/deserialization error: {}", err),
        }
    }
}

/// Specialized `Result` type for SubX operations.
pub type SubXResult<T> = Result<T, SubXError>;

impl SubXError {
    /// Create a configuration error with the given message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let err = SubXError::config("invalid setting");
    /// assert_eq!(err.to_string(), "Configuration error: invalid setting");
    /// ```
    pub fn config<S: Into<String>>(message: S) -> Self {
        SubXError::Config {
            message: message.into(),
        }
    }

    /// Create a subtitle format error for the given format and message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let err = SubXError::subtitle_format("SRT", "invalid timestamp");
    /// assert!(err.to_string().contains("SRT"));
    /// ```
    pub fn subtitle_format<S1, S2>(format: S1, message: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        SubXError::SubtitleFormat {
            format: format.into(),
            message: message.into(),
        }
    }

    /// Create an audio processing error with the given message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let err = SubXError::audio_processing("decode failed");
    /// assert_eq!(err.to_string(), "Audio processing error: decode failed");
    /// ```
    pub fn audio_processing<S: Into<String>>(message: S) -> Self {
        SubXError::AudioProcessing {
            message: message.into(),
        }
    }

    /// Create an AI service error with the given message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let err = SubXError::ai_service("network failure");
    /// assert_eq!(err.to_string(), "AI service error: network failure");
    /// ```
    pub fn ai_service<S: Into<String>>(message: S) -> Self {
        SubXError::AiService(message.into())
    }

    /// Create a file matching error with the given message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let err = SubXError::file_matching("not found");
    /// assert_eq!(err.to_string(), "File matching error: not found");
    /// ```
    pub fn file_matching<S: Into<String>>(message: S) -> Self {
        SubXError::FileMatching {
            message: message.into(),
        }
    }
    /// Create a parallel processing error with the given message.
    pub fn parallel_processing(msg: String) -> Self {
        SubXError::CommandExecution(format!("Parallel processing error: {}", msg))
    }
    /// Create a task execution failure error with task ID and reason.
    pub fn task_execution_failed(task_id: String, reason: String) -> Self {
        SubXError::CommandExecution(format!("Task {} execution failed: {}", task_id, reason))
    }
    /// Create a worker pool exhausted error.
    pub fn worker_pool_exhausted() -> Self {
        SubXError::CommandExecution("Worker pool exhausted".to_string())
    }
    /// Create a task timeout error with task ID and duration.
    pub fn task_timeout(task_id: String, duration: std::time::Duration) -> Self {
        SubXError::CommandExecution(format!(
            "Task {} timed out (limit: {:?})",
            task_id, duration
        ))
    }
    /// Create a dialogue detection failure error with the given message.
    pub fn dialogue_detection_failed<S: Into<String>>(msg: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("Dialogue detection failed: {}", msg.into()),
        }
    }
    /// Create an invalid audio format error for the given format.
    pub fn invalid_audio_format<S: Into<String>>(format: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("Unsupported audio format: {}", format.into()),
        }
    }
    /// Create an invalid dialogue segment error with the given reason.
    pub fn dialogue_segment_invalid<S: Into<String>>(reason: S) -> Self {
        SubXError::AudioProcessing {
            message: format!("Invalid dialogue segment: {}", reason.into()),
        }
    }
    /// Return the corresponding exit code for this error variant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// assert_eq!(SubXError::config("x").exit_code(), 2);
    /// ```
    pub fn exit_code(&self) -> i32 {
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

    /// Return a user-friendly error message with suggested remedies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::error::SubXError;
    /// let msg = SubXError::config("missing key").user_friendly_message();
    /// assert!(msg.contains("Configuration error:"));
    /// ```
    pub fn user_friendly_message(&self) -> String {
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
            SubXError::Other(err) => {
                format!("Unknown error: {}\nHint: please report this issue", err)
            }
            _ => format!("Error: {}", self),
        }
    }
}

/// Helper functions for Whisper API and audio processing related errors.
impl SubXError {
    /// Create a Whisper API error.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message describing the Whisper API failure
    ///
    /// # Returns
    ///
    /// A new `SubXError::Api` variant with Whisper as the source
    pub fn whisper_api<T: Into<String>>(message: T) -> Self {
        Self::Api {
            message: message.into(),
            source: ApiErrorSource::Whisper,
        }
    }

    /// Create an audio extraction/transcoding error.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message describing the audio processing failure
    ///
    /// # Returns
    ///
    /// A new `SubXError::AudioProcessing` variant
    pub fn audio_extraction<T: Into<String>>(message: T) -> Self {
        Self::AudioProcessing {
            message: message.into(),
        }
    }
}

/// API error source enumeration.
///
/// Specifies the source of API-related errors to help with error diagnosis
/// and handling.
#[derive(Debug, thiserror::Error)]
pub enum ApiErrorSource {
    /// OpenAI Whisper API
    #[error("OpenAI")]
    OpenAI,
    /// Whisper API
    #[error("Whisper")]
    Whisper,
}

// Support conversion from Box<dyn Error> to SubXError::AudioProcessing
impl From<Box<dyn std::error::Error>> for SubXError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        SubXError::audio_processing(err.to_string())
    }
}
