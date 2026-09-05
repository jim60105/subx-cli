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
/// Each error variant maps to a stable process exit code (1–6). That
/// mapping is a property of running the `subx-cli` binary and lives in
/// `crate::cli::error_ext::SubXErrorExt::exit_code` (rendered terminal
/// prose likewise); this enum itself carries only the machine-readable
/// contract — [`Self::category`], [`Self::machine_code`] and [`Self::hint`]
/// — which library consumers can call without importing anything.
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

    /// The active output mode (e.g. `--output json`) is incompatible
    /// with the requested subcommand.
    ///
    /// Currently emitted by `generate-completion`, whose stdout is by
    /// design a shell-completion script and cannot be wrapped in the
    /// JSON envelope contract.
    ///
    /// Only the `subx-cli` binary ever constructs this variant — no code
    /// under `src/core/` or `src/services/` produces it. It nevertheless
    /// stays in the core enum so `category()` and `machine_code()` keep
    /// their wildcard-free exhaustive matches (a new variant must be
    /// mapped, not absorbed by a catch-all). The deliberate asymmetry
    /// below — `category()` is the generic `"command_execution"` while
    /// `machine_code()` is the more specific
    /// `"E_OUTPUT_MODE_UNSUPPORTED"` — is spec-locked.
    #[error(
        "The '{command}' command does not support --output json; its stdout is a shell-completion script"
    )]
    OutputModeUnsupported {
        /// The subcommand that rejected the output mode (e.g. `"generate-completion"`).
        command: String,
    },

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
    /// Stable snake_case machine-readable category for the JSON error
    /// envelope. The mapping is closed and exhaustive (no wildcard arm)
    /// so the compiler enforces updates whenever a new variant is added.
    ///
    /// This mapping is locked by the `error-handling` capability spec.
    pub fn category(&self) -> &'static str {
        match self {
            // Mapped 1:1 from the closed set defined in the spec.
            SubXError::Io(_) => "io",
            SubXError::Config { .. } => "config",
            SubXError::SubtitleFormat { .. } => "subtitle_format",
            SubXError::AiService(_) => "ai_service",
            SubXError::Api { .. } => "api",
            SubXError::AudioProcessing { .. } => "audio_processing",
            SubXError::FileMatching { .. } => "file_matching",
            SubXError::FileAlreadyExists(_) => "file_already_exists",
            SubXError::FileNotFound(_) => "file_not_found",
            SubXError::InvalidFileName(_) => "invalid_file_name",
            SubXError::FileOperationFailed(_) => "file_operation_failed",
            SubXError::CommandExecution(_) => "command_execution",
            // Spec locks `category == "command_execution"` for this variant
            // even though the machine_code is the more specific
            // `E_OUTPUT_MODE_UNSUPPORTED`.
            SubXError::OutputModeUnsupported { .. } => "command_execution",
            SubXError::NoInputSpecified => "no_input_specified",
            SubXError::InvalidPath(_) => "invalid_path",
            SubXError::PathNotFound(_) => "path_not_found",
            SubXError::DirectoryReadError { .. } => "directory_read_error",
            SubXError::InvalidSyncConfiguration => "invalid_sync_configuration",
            SubXError::UnsupportedFileType(_) => "unsupported_file_type",
            SubXError::Other(_) => "other",
        }
    }

    /// Stable upper-snake-case machine code prefixed with `E_`.
    /// Mirrors [`Self::category`] one-to-one and is similarly closed
    /// against the addition of new variants.
    pub fn machine_code(&self) -> &'static str {
        match self {
            SubXError::Io(_) => "E_IO",
            SubXError::Config { .. } => "E_CONFIG",
            SubXError::SubtitleFormat { .. } => "E_SUBTITLE_FORMAT",
            SubXError::AiService(_) => "E_AI_SERVICE",
            SubXError::Api { .. } => "E_API",
            SubXError::AudioProcessing { .. } => "E_AUDIO_PROCESSING",
            SubXError::FileMatching { .. } => "E_FILE_MATCHING",
            SubXError::FileAlreadyExists(_) => "E_FILE_ALREADY_EXISTS",
            SubXError::FileNotFound(_) => "E_FILE_NOT_FOUND",
            SubXError::InvalidFileName(_) => "E_INVALID_FILE_NAME",
            SubXError::FileOperationFailed(_) => "E_FILE_OPERATION_FAILED",
            SubXError::CommandExecution(_) => "E_COMMAND_EXECUTION",
            SubXError::OutputModeUnsupported { .. } => "E_OUTPUT_MODE_UNSUPPORTED",
            SubXError::NoInputSpecified => "E_NO_INPUT_SPECIFIED",
            SubXError::InvalidPath(_) => "E_INVALID_PATH",
            SubXError::PathNotFound(_) => "E_PATH_NOT_FOUND",
            SubXError::DirectoryReadError { .. } => "E_DIRECTORY_READ_ERROR",
            SubXError::InvalidSyncConfiguration => "E_INVALID_SYNC_CONFIGURATION",
            SubXError::UnsupportedFileType(_) => "E_UNSUPPORTED_FILE_TYPE",
            SubXError::Other(_) => "E_OTHER",
        }
    }

    /// Short user-facing remediation hint, or `None` when none applies.
    ///
    /// This is a separate, structured surface from the prose hints
    /// already baked into the binary's `SubXErrorExt::user_friendly_message`
    /// (`src/cli/error_ext.rs`); JSON callers receive it under
    /// `error.hint`.
    ///
    /// The returned text is written for the `subx-cli` terminal and names
    /// its binary and flags. Library consumers SHALL treat the return
    /// value as an *availability* signal — branch on `Some`/`None` and
    /// render their own localized copy — rather than as display copy.
    /// Rewriting the prose is not a breaking change; changing *which*
    /// variants return `Some` is: that set is the stable part of the
    /// contract.
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            SubXError::Config { .. } => {
                Some("Run 'subx-cli config --help' for configuration details.")
            }
            SubXError::Api { .. } | SubXError::AiService(_) => {
                Some("Check network connectivity and the configured API key.")
            }
            SubXError::SubtitleFormat { .. } => {
                Some("Check the subtitle file's format and encoding.")
            }
            SubXError::AudioProcessing { .. } => {
                Some("Verify the media file's integrity and supported codecs.")
            }
            SubXError::FileMatching { .. } => Some("Verify file paths and patterns."),
            SubXError::NoInputSpecified => Some("Pass an input path or use the -i/--input flag."),
            SubXError::InvalidSyncConfiguration => {
                Some("Specify both video and subtitle files, or use -i for batch processing.")
            }
            SubXError::PathNotFound(_) | SubXError::FileNotFound(_) => {
                Some("Verify the path exists and is accessible.")
            }
            SubXError::OutputModeUnsupported { .. } => Some(
                "Run the command without `--output json` (and without SUBX_OUTPUT=json) to receive the shell-completion script.",
            ),
            _ => None,
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
