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
    Config { message: String },

    /// Subtitle format error indicating invalid timestamps or structure.
    ///
    /// Provides the subtitle format and detailed message.
    #[error("Subtitle format error [{format}]: {message}")]
    SubtitleFormat { format: String, message: String },

    /// AI service encountered an error.
    ///
    /// Captures the raw error message from the AI provider.
    #[error("AI service error: {0}")]
    AiService(String),

    /// Audio processing error during analysis or format conversion.
    ///
    /// Provides a message describing the audio processing failure.
    #[error("Audio processing error: {message}")]
    AudioProcessing { message: String },

    /// Error during file matching or discovery.
    ///
    /// Contains details about path resolution or pattern matching failures.
    #[error("File matching error: {message}")]
    FileMatching { message: String },
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

    /// Catch-all error variant wrapping any other failure.
    #[error("Unknown error: {0}")]
    Other(#[from] anyhow::Error),
}

// 單元測試: SubXError 錯誤類型與輔助方法
#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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
        let io_error = io::Error::new(io::ErrorKind::NotFound, "檔案不存在");
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
        assert!(message.contains("subx config --help"));

        let ai_error = SubXError::ai_service("network failure".to_string());
        let message = ai_error.user_friendly_message();
        assert!(message.contains("AI service error:"));
        assert!(message.contains("check network connection"));
    }
}

// 將 reqwest 錯誤轉換為 AI 服務錯誤
impl From<reqwest::Error> for SubXError {
    fn from(err: reqwest::Error) -> Self {
        SubXError::AiService(err.to_string())
    }
}

// 將檔案探索錯誤轉換為文件匹配錯誤
impl From<walkdir::Error> for SubXError {
    fn from(err: walkdir::Error) -> Self {
        SubXError::FileMatching {
            message: err.to_string(),
        }
    }
}
// 將 symphonia 錯誤轉換為音訊處理錯誤
impl From<symphonia::core::errors::Error> for SubXError {
    fn from(err: symphonia::core::errors::Error) -> Self {
        SubXError::audio_processing(err.to_string())
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
                "Configuration error: {}\nHint: run 'subx config --help' for details",
                message
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
        }
    }
}

// aus 錯誤轉換
impl From<aus::AudioError> for SubXError {
    fn from(error: aus::AudioError) -> Self {
        SubXError::audio_processing(format!("{:?}", error))
    }
}

impl From<aus::spectrum::SpectrumError> for SubXError {
    fn from(error: aus::spectrum::SpectrumError) -> Self {
        SubXError::audio_processing(format!("頻譜處理錯誤: {:?}", error))
    }
}
