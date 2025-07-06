//! Key-value validation for configuration service.
//!
//! This module handles the validation logic that was previously embedded
//! in `ProductionConfigService::validate_and_set_value`. It provides
//! field-specific validation for configuration keys and values.
//!
//! # Architecture
//!
//! - [`crate::config::validation`] - Low-level validation functions for individual values
//! - [`crate::config::validator`] - High-level configuration section validators  
//! - [`crate::config::field_validator`] (this module) - Key-value validation for configuration service

use super::validation::*;
use crate::{Result, error::SubXError};

/// Validate and parse a configuration field based on its key.
///
/// This function handles the validation logic that was previously
/// embedded in ProductionConfigService::validate_and_set_value.
///
/// # Arguments
/// * `key` - The configuration key (e.g., "ai.temperature")
/// * `value` - The string value to validate and parse
///
/// # Returns
/// Returns Ok(()) if validation passes, or an error describing the validation failure.
pub fn validate_field(key: &str, value: &str) -> Result<()> {
    match key {
        // AI configuration fields
        "ai.provider" => {
            validate_non_empty_string(value, "AI provider")?;
            validate_enum(value, &["openai", "anthropic", "local", "openrouter"])?;
        }
        "ai.model" => validate_ai_model(value)?,
        "ai.api_key" => {
            if !value.is_empty() {
                validate_api_key(value)?;
            }
        }
        "ai.base_url" => validate_url_format(value)?,
        "ai.temperature" => {
            let temp: f32 = value
                .parse()
                .map_err(|_| SubXError::config("Temperature must be a number"))?;
            validate_temperature(temp)?;
        }
        "ai.max_tokens" => {
            let tokens: u32 = value
                .parse()
                .map_err(|_| SubXError::config("Max tokens must be a positive integer"))?;
            validate_positive_number(tokens as f64)?;
        }
        "ai.max_sample_length" => {
            let length: usize = value
                .parse()
                .map_err(|_| SubXError::config("Max sample length must be a positive integer"))?;
            validate_range(length, 100, 10000)?;
        }
        "ai.retry_attempts" => {
            let attempts: u32 = value
                .parse()
                .map_err(|_| SubXError::config("Retry attempts must be a positive integer"))?;
            validate_range(attempts, 1, 10)?;
        }
        "ai.retry_delay_ms" => {
            let delay: u64 = value
                .parse()
                .map_err(|_| SubXError::config("Retry delay must be a positive integer"))?;
            validate_range(delay, 100, 30000)?;
        }
        "ai.request_timeout_seconds" => {
            let timeout: u64 = value
                .parse()
                .map_err(|_| SubXError::config("Request timeout must be a positive integer"))?;
            validate_range(timeout, 10, 600)?;
        }

        // Sync configuration fields
        "sync.default_method" => {
            validate_enum(value, &["auto", "vad", "manual"])?;
        }
        "sync.max_offset_seconds" => {
            let offset: f32 = value
                .parse()
                .map_err(|_| SubXError::config("Max offset must be a number"))?;
            validate_range(offset, 0.1, 3600.0)?;
        }
        "sync.vad.enabled" => {
            parse_bool(value)?;
        }
        "sync.vad.sensitivity" => {
            let sensitivity: f32 = value
                .parse()
                .map_err(|_| SubXError::config("VAD sensitivity must be a number"))?;
            validate_range(sensitivity, 0.0, 1.0)?;
        }
        "sync.vad.padding_chunks" => {
            let chunks: u32 = value
                .parse()
                .map_err(|_| SubXError::config("Padding chunks must be a non-negative integer"))?;
            validate_range(chunks, 0, 10)?;
        }
        "sync.vad.min_speech_duration_ms" => {
            let _duration: u32 = value.parse().map_err(|_| {
                SubXError::config("Min speech duration must be a non-negative integer")
            })?;
            // Non-negative validation is implicit in u32 parsing
        }

        // Formats configuration fields
        "formats.default_output" => {
            validate_enum(value, &["srt", "ass", "vtt", "webvtt"])?;
        }
        "formats.preserve_styling" => {
            parse_bool(value)?;
        }
        "formats.default_encoding" => {
            validate_enum(value, &["utf-8", "gbk", "big5", "shift_jis"])?;
        }
        "formats.encoding_detection_confidence" => {
            let confidence: f32 = value
                .parse()
                .map_err(|_| SubXError::config("Encoding detection confidence must be a number"))?;
            validate_range(confidence, 0.0, 1.0)?;
        }

        // General configuration fields
        "general.backup_enabled" => {
            parse_bool(value)?;
        }
        "general.max_concurrent_jobs" => {
            let jobs: usize = value
                .parse()
                .map_err(|_| SubXError::config("Max concurrent jobs must be a positive integer"))?;
            validate_range(jobs, 1, 64)?;
        }
        "general.task_timeout_seconds" => {
            let timeout: u64 = value
                .parse()
                .map_err(|_| SubXError::config("Task timeout must be a positive integer"))?;
            validate_range(timeout, 30, 3600)?;
        }
        "general.enable_progress_bar" => {
            parse_bool(value)?;
        }
        "general.worker_idle_timeout_seconds" => {
            let timeout: u64 = value
                .parse()
                .map_err(|_| SubXError::config("Worker idle timeout must be a positive integer"))?;
            validate_range(timeout, 10, 3600)?;
        }

        // Parallel configuration fields
        "parallel.max_workers" => {
            let workers: usize = value
                .parse()
                .map_err(|_| SubXError::config("Max workers must be a positive integer"))?;
            validate_range(workers, 1, 64)?;
        }
        "parallel.task_queue_size" => {
            let size: usize = value
                .parse()
                .map_err(|_| SubXError::config("Task queue size must be a positive integer"))?;
            validate_range(size, 100, 10000)?;
        }
        "parallel.enable_task_priorities" => {
            parse_bool(value)?;
        }
        "parallel.auto_balance_workers" => {
            parse_bool(value)?;
        }
        "parallel.overflow_strategy" => {
            validate_enum(value, &["Block", "Drop", "Expand"])?;
        }

        _ => {
            return Err(SubXError::config(format!(
                "Unknown configuration key: {key}"
            )));
        }
    }

    Ok(())
}

/// Get a user-friendly description for a configuration field.
pub fn get_field_description(key: &str) -> &'static str {
    match key {
        "ai.provider" => "AI service provider (e.g., 'openai')",
        "ai.model" => "AI model name (e.g., 'gpt-4.1-mini')",
        "ai.api_key" => "API key for the AI service",
        "ai.base_url" => "Custom API endpoint URL (optional)",
        "ai.temperature" => "AI response randomness (0.0-2.0)",
        "ai.max_tokens" => "Maximum tokens in AI response",
        "ai.max_sample_length" => "Maximum sample length for AI processing",
        "ai.retry_attempts" => "Number of retry attempts for AI requests",
        "ai.retry_delay_ms" => "Delay between retry attempts in milliseconds",
        "ai.request_timeout_seconds" => "Request timeout in seconds",

        "sync.default_method" => "Synchronization method ('auto', 'vad', or 'manual')",
        "sync.max_offset_seconds" => "Maximum allowed time offset in seconds",
        "sync.vad.enabled" => "Enable voice activity detection",
        "sync.vad.sensitivity" => "Voice activity detection threshold (0.0-1.0)",
        "sync.vad.chunk_size" => "VAD processing chunk size (must be power of 2)",
        "sync.vad.sample_rate" => "Audio sample rate for VAD processing",
        "sync.vad.padding_chunks" => "Number of padding chunks for VAD",
        "sync.vad.min_speech_duration_ms" => "Minimum speech duration in milliseconds",

        "formats.default_output" => "Default output format for subtitles",
        "formats.preserve_styling" => "Preserve subtitle styling information",
        "formats.default_encoding" => "Default character encoding",
        "formats.encoding_detection_confidence" => "Confidence threshold for encoding detection",

        "general.backup_enabled" => "Enable automatic backup creation",
        "general.max_concurrent_jobs" => "Maximum number of concurrent jobs",
        "general.task_timeout_seconds" => "Task timeout in seconds",
        "general.enable_progress_bar" => "Enable progress bar display",
        "general.worker_idle_timeout_seconds" => "Worker idle timeout in seconds",

        "parallel.max_workers" => "Maximum number of worker threads",
        "parallel.task_queue_size" => "Size of the task queue",
        "parallel.enable_task_priorities" => "Enable task priority system",
        "parallel.auto_balance_workers" => "Enable automatic worker load balancing",
        "parallel.overflow_strategy" => "Strategy for handling queue overflow",

        _ => "Configuration field",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ai_fields() {
        // Valid cases
        assert!(validate_field("ai.provider", "openai").is_ok());
        assert!(validate_field("ai.provider", "openrouter").is_ok());
        assert!(validate_field("ai.temperature", "0.8").is_ok());
        assert!(validate_field("ai.max_tokens", "4000").is_ok());

        // Invalid cases
        assert!(validate_field("ai.provider", "invalid").is_err());
        assert!(validate_field("ai.temperature", "3.0").is_err());
        assert!(validate_field("ai.max_tokens", "0").is_err());
    }

    #[test]
    fn test_validate_sync_fields() {
        // Valid cases
        assert!(validate_field("sync.default_method", "vad").is_ok());
        assert!(validate_field("sync.vad.sensitivity", "0.5").is_ok());
        assert!(validate_field("sync.vad.padding_chunks", "3").is_ok());

        // Invalid cases
        assert!(validate_field("sync.default_method", "invalid").is_err());
        assert!(validate_field("sync.vad.sensitivity", "1.5").is_err());
        assert!(validate_field("sync.vad.padding_chunks", "11").is_err()); // Exceeds max allowed padding_chunks
    }

    #[test]
    fn test_validate_formats_fields() {
        // Valid cases
        assert!(validate_field("formats.default_output", "srt").is_ok());
        assert!(validate_field("formats.preserve_styling", "true").is_ok());

        // Invalid cases
        assert!(validate_field("formats.default_output", "invalid").is_err());
        assert!(validate_field("formats.preserve_styling", "maybe").is_err());
    }

    #[test]
    fn test_validate_unknown_field() {
        assert!(validate_field("unknown.field", "value").is_err());
    }

    #[test]
    fn test_get_field_description() {
        assert!(!get_field_description("ai.provider").is_empty());
        assert!(!get_field_description("sync.vad.sensitivity").is_empty());
        assert_eq!(
            get_field_description("unknown.field"),
            "Configuration field"
        );
    }
}
