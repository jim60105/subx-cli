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

/// Normalize an `ai.provider` value to its canonical form.
///
/// This is the **single** alias-resolution point in the codebase. It
/// lowercases and trims its input and maps the alias `"ollama"` to the
/// canonical identifier `"local"`. All other recognized providers
/// (`openai`, `openrouter`, `azure-openai`, `local`) pass through after
/// trimming and lowercasing. Unknown values pass through unchanged so
/// downstream allow-list validation still rejects them.
///
/// Every component that reads or writes `ai.provider` SHALL invoke this
/// helper before using the value:
///
/// 1. `subx config set ai.provider <value>` — the persisted on-disk value
///    is the canonical form.
/// 2. `subx config get ai.provider` — returns the canonical form.
/// 3. `ProductionConfigService` env-var loading — `SUBX_AI_PROVIDER=ollama`
///    is normalized before any precedence or scoping decision (including
///    the hosted-provider env-var carve-out for `local`).
/// 4. `validate_ai_config` — validation arms key off the canonical value.
/// 5. `ComponentFactory::create_ai_provider` — the dispatch match arm
///    uses the canonical value.
///
/// # Examples
///
/// ```
/// use subx_cli::config::field_validator::normalize_ai_provider;
///
/// assert_eq!(normalize_ai_provider("ollama"), "local");
/// assert_eq!(normalize_ai_provider("OLLAMA"), "local");
/// assert_eq!(normalize_ai_provider(" ollama "), "local");
/// assert_eq!(normalize_ai_provider("openai"), "openai");
/// assert_eq!(normalize_ai_provider("Azure-OpenAI"), "azure-openai");
/// // Unknown values are returned trimmed+lowercased so the allow-list
/// // can still reject them with the existing error.
/// assert_eq!(normalize_ai_provider("grok"), "grok");
/// ```
pub fn normalize_ai_provider(value: &str) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed == "ollama" {
        "local".to_string()
    } else {
        trimmed
    }
}

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
            validate_enum(
                value,
                &[
                    "openai",
                    "anthropic",
                    "local",
                    "ollama",
                    "openrouter",
                    "azure-openai",
                ],
            )?;
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

        // Azure OpenAI specific fields
        "ai.api_version" => {
            validate_non_empty_string(value, "Azure OpenAI API version")?;
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
        "general.max_subtitle_bytes" => {
            let bytes: u64 = value
                .parse()
                .map_err(|_| SubXError::config("max_subtitle_bytes must be a positive integer"))?;
            validate_range(bytes, 1024_u64, 1_073_741_824_u64)?;
        }
        "general.max_audio_bytes" => {
            let bytes: u64 = value
                .parse()
                .map_err(|_| SubXError::config("max_audio_bytes must be a positive integer"))?;
            validate_range(bytes, 1024_u64, 10_737_418_240_u64)?;
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

        // Translation configuration fields
        "translation.batch_size" => {
            let size: usize = value.parse().map_err(|_| {
                SubXError::config("Translation batch size must be a positive integer")
            })?;
            validate_range(size, 1, 1000)?;
        }
        "translation.default_target_language" => {
            if !value.is_empty() {
                validate_non_empty_string(value, "translation.default_target_language")?;
            }
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
        "ai.provider" => {
            "AI service provider ('openai', 'openrouter', 'azure-openai', or 'local'; 'ollama' is accepted as an alias for 'local')"
        }
        "ai.model" => "AI model name (e.g., 'gpt-4.1-mini')",
        "ai.api_key" => "API key for the AI service",
        "ai.base_url" => "Custom API endpoint URL (optional)",
        "ai.temperature" => "AI response randomness (0.0-2.0)",
        "ai.max_tokens" => "Maximum tokens in AI response",
        "ai.max_sample_length" => "Maximum sample length for AI processing",
        "ai.retry_attempts" => "Number of retry attempts for AI requests",
        "ai.retry_delay_ms" => "Delay between retry attempts in milliseconds",
        "ai.request_timeout_seconds" => "Request timeout in seconds",
        "ai.api_version" => "Azure OpenAI API version (optional, defaults to latest)",

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
        "general.max_subtitle_bytes" => "Maximum subtitle file size in bytes",
        "general.max_audio_bytes" => "Maximum audio file size in bytes",

        "parallel.max_workers" => "Maximum number of worker threads",
        "parallel.task_queue_size" => "Size of the task queue",
        "parallel.enable_task_priorities" => "Enable task priority system",
        "parallel.auto_balance_workers" => "Enable automatic worker load balancing",
        "parallel.overflow_strategy" => "Strategy for handling queue overflow",

        "translation.batch_size" => "Maximum subtitle cues per AI translation request (1-1000)",
        "translation.default_target_language" => {
            "Default target language used when --target-language is omitted"
        }

        _ => "Configuration field",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ai.provider ──────────────────────────────────────────────────────────

    #[test]
    fn test_ai_provider_valid_all_enum_values() {
        for v in &[
            "openai",
            "anthropic",
            "local",
            "ollama",
            "openrouter",
            "azure-openai",
        ] {
            assert!(validate_field("ai.provider", v).is_ok(), "provider={v}");
        }
    }

    // ── normalize_ai_provider ────────────────────────────────────────────────

    #[test]
    fn test_normalize_ai_provider_ollama_alias() {
        assert_eq!(normalize_ai_provider("ollama"), "local");
        assert_eq!(normalize_ai_provider("OLLAMA"), "local");
        assert_eq!(normalize_ai_provider(" ollama "), "local");
        assert_eq!(normalize_ai_provider("\tOllama\n"), "local");
    }

    #[test]
    fn test_normalize_ai_provider_canonical_pass_through() {
        for v in &["openai", "openrouter", "azure-openai", "local"] {
            assert_eq!(normalize_ai_provider(v), *v, "canonical input {v}");
        }
    }

    #[test]
    fn test_normalize_ai_provider_unknown_pass_through_lowercased() {
        // Unknown values are returned (trimmed + lowercased) so the
        // downstream allow-list still rejects them with the existing error.
        assert_eq!(normalize_ai_provider("grok"), "grok");
        assert_eq!(normalize_ai_provider("GROK"), "grok");
        assert_eq!(
            normalize_ai_provider("  unknown-provider  "),
            "unknown-provider"
        );
    }

    #[test]
    fn test_ai_provider_invalid_value() {
        assert!(validate_field("ai.provider", "grok").is_err());
        assert!(validate_field("ai.provider", "OPENAI").is_err()); // case-sensitive
    }

    #[test]
    fn test_ai_provider_empty_value() {
        assert!(validate_field("ai.provider", "").is_err());
    }

    // ── ai.model ─────────────────────────────────────────────────────────────

    #[test]
    fn test_ai_model_valid() {
        assert!(validate_field("ai.model", "gpt-4.1-mini").is_ok());
        assert!(validate_field("ai.model", &"a".repeat(100)).is_ok()); // max length
    }

    #[test]
    fn test_ai_model_empty() {
        assert!(validate_field("ai.model", "").is_err());
    }

    #[test]
    fn test_ai_model_too_long() {
        assert!(validate_field("ai.model", &"a".repeat(101)).is_err());
    }

    // ── ai.api_key ────────────────────────────────────────────────────────────

    #[test]
    fn test_ai_api_key_empty_is_ok() {
        // Empty api_key is explicitly allowed (optional)
        assert!(validate_field("ai.api_key", "").is_ok());
    }

    #[test]
    fn test_ai_api_key_valid() {
        assert!(validate_field("ai.api_key", "sk-abcdefghij1234").is_ok());
    }

    #[test]
    fn test_ai_api_key_too_short() {
        assert!(validate_field("ai.api_key", "short").is_err()); // < 10 chars
    }

    // ── ai.base_url ───────────────────────────────────────────────────────────

    #[test]
    fn test_ai_base_url_valid() {
        assert!(validate_field("ai.base_url", "https://api.openai.com/v1").is_ok());
        assert!(validate_field("ai.base_url", "http://localhost:8080").is_ok());
    }

    #[test]
    fn test_ai_base_url_empty_is_ok() {
        assert!(validate_field("ai.base_url", "").is_ok());
        assert!(validate_field("ai.base_url", "   ").is_ok());
    }

    #[test]
    fn test_ai_base_url_invalid() {
        assert!(validate_field("ai.base_url", "not-a-url").is_err());
        assert!(validate_field("ai.base_url", "://missing-scheme").is_err());
    }

    // ── ai.temperature ────────────────────────────────────────────────────────

    #[test]
    fn test_ai_temperature_valid_boundaries() {
        assert!(validate_field("ai.temperature", "0.0").is_ok());
        assert!(validate_field("ai.temperature", "2.0").is_ok());
        assert!(validate_field("ai.temperature", "1.0").is_ok());
    }

    #[test]
    fn test_ai_temperature_out_of_range() {
        assert!(validate_field("ai.temperature", "-0.1").is_err());
        assert!(validate_field("ai.temperature", "2.1").is_err());
    }

    #[test]
    fn test_ai_temperature_parse_error() {
        assert!(validate_field("ai.temperature", "hot").is_err());
        assert!(validate_field("ai.temperature", "").is_err());
    }

    // ── ai.max_tokens ─────────────────────────────────────────────────────────

    #[test]
    fn test_ai_max_tokens_valid() {
        assert!(validate_field("ai.max_tokens", "1").is_ok());
        assert!(validate_field("ai.max_tokens", "4096").is_ok());
    }

    #[test]
    fn test_ai_max_tokens_zero_is_err() {
        assert!(validate_field("ai.max_tokens", "0").is_err());
    }

    #[test]
    fn test_ai_max_tokens_parse_error() {
        assert!(validate_field("ai.max_tokens", "lots").is_err());
        assert!(validate_field("ai.max_tokens", "-1").is_err());
    }

    // ── ai.max_sample_length ─────────────────────────────────────────────────

    #[test]
    fn test_ai_max_sample_length_valid_boundaries() {
        assert!(validate_field("ai.max_sample_length", "100").is_ok());
        assert!(validate_field("ai.max_sample_length", "10000").is_ok());
        assert!(validate_field("ai.max_sample_length", "5000").is_ok());
    }

    #[test]
    fn test_ai_max_sample_length_out_of_range() {
        assert!(validate_field("ai.max_sample_length", "99").is_err());
        assert!(validate_field("ai.max_sample_length", "10001").is_err());
    }

    #[test]
    fn test_ai_max_sample_length_parse_error() {
        assert!(validate_field("ai.max_sample_length", "big").is_err());
    }

    // ── ai.retry_attempts ─────────────────────────────────────────────────────

    #[test]
    fn test_ai_retry_attempts_valid_boundaries() {
        assert!(validate_field("ai.retry_attempts", "1").is_ok());
        assert!(validate_field("ai.retry_attempts", "10").is_ok());
        assert!(validate_field("ai.retry_attempts", "3").is_ok());
    }

    #[test]
    fn test_ai_retry_attempts_out_of_range() {
        assert!(validate_field("ai.retry_attempts", "0").is_err());
        assert!(validate_field("ai.retry_attempts", "11").is_err());
    }

    #[test]
    fn test_ai_retry_attempts_parse_error() {
        assert!(validate_field("ai.retry_attempts", "many").is_err());
    }

    // ── ai.retry_delay_ms ─────────────────────────────────────────────────────

    #[test]
    fn test_ai_retry_delay_ms_valid_boundaries() {
        assert!(validate_field("ai.retry_delay_ms", "100").is_ok());
        assert!(validate_field("ai.retry_delay_ms", "30000").is_ok());
        assert!(validate_field("ai.retry_delay_ms", "1000").is_ok());
    }

    #[test]
    fn test_ai_retry_delay_ms_out_of_range() {
        assert!(validate_field("ai.retry_delay_ms", "99").is_err());
        assert!(validate_field("ai.retry_delay_ms", "30001").is_err());
    }

    #[test]
    fn test_ai_retry_delay_ms_parse_error() {
        assert!(validate_field("ai.retry_delay_ms", "fast").is_err());
    }

    // ── ai.request_timeout_seconds ────────────────────────────────────────────

    #[test]
    fn test_ai_request_timeout_seconds_valid_boundaries() {
        assert!(validate_field("ai.request_timeout_seconds", "10").is_ok());
        assert!(validate_field("ai.request_timeout_seconds", "600").is_ok());
        assert!(validate_field("ai.request_timeout_seconds", "60").is_ok());
    }

    #[test]
    fn test_ai_request_timeout_seconds_out_of_range() {
        assert!(validate_field("ai.request_timeout_seconds", "9").is_err());
        assert!(validate_field("ai.request_timeout_seconds", "601").is_err());
    }

    #[test]
    fn test_ai_request_timeout_seconds_parse_error() {
        assert!(validate_field("ai.request_timeout_seconds", "now").is_err());
    }

    // ── ai.api_version ────────────────────────────────────────────────────────

    #[test]
    fn test_ai_api_version_valid() {
        assert!(validate_field("ai.api_version", "2024-02-15-preview").is_ok());
        assert!(validate_field("ai.api_version", "v1").is_ok());
    }

    #[test]
    fn test_ai_api_version_empty_is_err() {
        assert!(validate_field("ai.api_version", "").is_err());
        assert!(validate_field("ai.api_version", "   ").is_err());
    }

    // ── sync.default_method ───────────────────────────────────────────────────

    #[test]
    fn test_sync_default_method_valid_all_values() {
        assert!(validate_field("sync.default_method", "auto").is_ok());
        assert!(validate_field("sync.default_method", "vad").is_ok());
        assert!(validate_field("sync.default_method", "manual").is_ok());
    }

    #[test]
    fn test_sync_default_method_invalid() {
        assert!(validate_field("sync.default_method", "force").is_err());
        assert!(validate_field("sync.default_method", "").is_err());
    }

    // ── sync.max_offset_seconds ───────────────────────────────────────────────

    #[test]
    fn test_sync_max_offset_seconds_valid_boundaries() {
        assert!(validate_field("sync.max_offset_seconds", "0.1").is_ok());
        assert!(validate_field("sync.max_offset_seconds", "3600.0").is_ok());
        assert!(validate_field("sync.max_offset_seconds", "10.0").is_ok());
    }

    #[test]
    fn test_sync_max_offset_seconds_out_of_range() {
        assert!(validate_field("sync.max_offset_seconds", "0.0").is_err());
        assert!(validate_field("sync.max_offset_seconds", "3601.0").is_err());
    }

    #[test]
    fn test_sync_max_offset_seconds_parse_error() {
        assert!(validate_field("sync.max_offset_seconds", "abit").is_err());
    }

    // ── sync.vad.enabled ──────────────────────────────────────────────────────

    #[test]
    fn test_sync_vad_enabled_valid() {
        assert!(validate_field("sync.vad.enabled", "true").is_ok());
        assert!(validate_field("sync.vad.enabled", "false").is_ok());
        assert!(validate_field("sync.vad.enabled", "1").is_ok());
        assert!(validate_field("sync.vad.enabled", "0").is_ok());
    }

    #[test]
    fn test_sync_vad_enabled_invalid() {
        assert!(validate_field("sync.vad.enabled", "maybe").is_err());
    }

    // ── sync.vad.sensitivity ──────────────────────────────────────────────────

    #[test]
    fn test_sync_vad_sensitivity_valid_boundaries() {
        assert!(validate_field("sync.vad.sensitivity", "0.0").is_ok());
        assert!(validate_field("sync.vad.sensitivity", "1.0").is_ok());
        assert!(validate_field("sync.vad.sensitivity", "0.5").is_ok());
    }

    #[test]
    fn test_sync_vad_sensitivity_out_of_range() {
        assert!(validate_field("sync.vad.sensitivity", "-0.1").is_err());
        assert!(validate_field("sync.vad.sensitivity", "1.1").is_err());
    }

    #[test]
    fn test_sync_vad_sensitivity_parse_error() {
        assert!(validate_field("sync.vad.sensitivity", "high").is_err());
    }

    // ── sync.vad.padding_chunks ───────────────────────────────────────────────

    #[test]
    fn test_sync_vad_padding_chunks_valid_boundaries() {
        assert!(validate_field("sync.vad.padding_chunks", "0").is_ok());
        assert!(validate_field("sync.vad.padding_chunks", "10").is_ok());
        assert!(validate_field("sync.vad.padding_chunks", "5").is_ok());
    }

    #[test]
    fn test_sync_vad_padding_chunks_out_of_range() {
        assert!(validate_field("sync.vad.padding_chunks", "11").is_err());
    }

    #[test]
    fn test_sync_vad_padding_chunks_parse_error() {
        assert!(validate_field("sync.vad.padding_chunks", "some").is_err());
        assert!(validate_field("sync.vad.padding_chunks", "-1").is_err());
    }

    // ── sync.vad.min_speech_duration_ms ───────────────────────────────────────

    #[test]
    fn test_sync_vad_min_speech_duration_ms_valid() {
        assert!(validate_field("sync.vad.min_speech_duration_ms", "0").is_ok());
        assert!(validate_field("sync.vad.min_speech_duration_ms", "250").is_ok());
        assert!(validate_field("sync.vad.min_speech_duration_ms", "4294967295").is_ok()); // u32::MAX
    }

    #[test]
    fn test_sync_vad_min_speech_duration_ms_parse_error() {
        assert!(validate_field("sync.vad.min_speech_duration_ms", "long").is_err());
        assert!(validate_field("sync.vad.min_speech_duration_ms", "-1").is_err());
    }

    // ── formats.default_output ────────────────────────────────────────────────

    #[test]
    fn test_formats_default_output_valid_all_values() {
        for v in &["srt", "ass", "vtt", "webvtt"] {
            assert!(
                validate_field("formats.default_output", v).is_ok(),
                "format={v}"
            );
        }
    }

    #[test]
    fn test_formats_default_output_invalid() {
        assert!(validate_field("formats.default_output", "txt").is_err());
        assert!(validate_field("formats.default_output", "SRT").is_err());
    }

    // ── formats.preserve_styling ──────────────────────────────────────────────

    #[test]
    fn test_formats_preserve_styling_valid() {
        assert!(validate_field("formats.preserve_styling", "true").is_ok());
        assert!(validate_field("formats.preserve_styling", "false").is_ok());
        assert!(validate_field("formats.preserve_styling", "yes").is_ok());
        assert!(validate_field("formats.preserve_styling", "no").is_ok());
    }

    #[test]
    fn test_formats_preserve_styling_invalid() {
        assert!(validate_field("formats.preserve_styling", "maybe").is_err());
    }

    // ── formats.default_encoding ──────────────────────────────────────────────

    #[test]
    fn test_formats_default_encoding_valid_all_values() {
        for v in &["utf-8", "gbk", "big5", "shift_jis"] {
            assert!(
                validate_field("formats.default_encoding", v).is_ok(),
                "enc={v}"
            );
        }
    }

    #[test]
    fn test_formats_default_encoding_invalid() {
        assert!(validate_field("formats.default_encoding", "latin1").is_err());
        assert!(validate_field("formats.default_encoding", "UTF-8").is_err()); // case-sensitive
    }

    // ── formats.encoding_detection_confidence ────────────────────────────────

    #[test]
    fn test_formats_encoding_detection_confidence_valid_boundaries() {
        assert!(validate_field("formats.encoding_detection_confidence", "0.0").is_ok());
        assert!(validate_field("formats.encoding_detection_confidence", "1.0").is_ok());
        assert!(validate_field("formats.encoding_detection_confidence", "0.75").is_ok());
    }

    #[test]
    fn test_formats_encoding_detection_confidence_out_of_range() {
        assert!(validate_field("formats.encoding_detection_confidence", "-0.1").is_err());
        assert!(validate_field("formats.encoding_detection_confidence", "1.1").is_err());
    }

    #[test]
    fn test_formats_encoding_detection_confidence_parse_error() {
        assert!(validate_field("formats.encoding_detection_confidence", "high").is_err());
    }

    // ── general.backup_enabled ────────────────────────────────────────────────

    #[test]
    fn test_general_backup_enabled_valid() {
        assert!(validate_field("general.backup_enabled", "true").is_ok());
        assert!(validate_field("general.backup_enabled", "false").is_ok());
        assert!(validate_field("general.backup_enabled", "on").is_ok());
        assert!(validate_field("general.backup_enabled", "off").is_ok());
    }

    #[test]
    fn test_general_backup_enabled_invalid() {
        assert!(validate_field("general.backup_enabled", "yeah").is_err());
    }

    // ── general.max_concurrent_jobs ───────────────────────────────────────────

    #[test]
    fn test_general_max_concurrent_jobs_valid_boundaries() {
        assert!(validate_field("general.max_concurrent_jobs", "1").is_ok());
        assert!(validate_field("general.max_concurrent_jobs", "64").is_ok());
        assert!(validate_field("general.max_concurrent_jobs", "8").is_ok());
    }

    #[test]
    fn test_general_max_concurrent_jobs_out_of_range() {
        assert!(validate_field("general.max_concurrent_jobs", "0").is_err());
        assert!(validate_field("general.max_concurrent_jobs", "65").is_err());
    }

    #[test]
    fn test_general_max_concurrent_jobs_parse_error() {
        assert!(validate_field("general.max_concurrent_jobs", "many").is_err());
    }

    // ── general.task_timeout_seconds ──────────────────────────────────────────

    #[test]
    fn test_general_task_timeout_seconds_valid_boundaries() {
        assert!(validate_field("general.task_timeout_seconds", "30").is_ok());
        assert!(validate_field("general.task_timeout_seconds", "3600").is_ok());
        assert!(validate_field("general.task_timeout_seconds", "300").is_ok());
    }

    #[test]
    fn test_general_task_timeout_seconds_out_of_range() {
        assert!(validate_field("general.task_timeout_seconds", "29").is_err());
        assert!(validate_field("general.task_timeout_seconds", "3601").is_err());
    }

    #[test]
    fn test_general_task_timeout_seconds_parse_error() {
        assert!(validate_field("general.task_timeout_seconds", "forever").is_err());
    }

    // ── general.enable_progress_bar ───────────────────────────────────────────

    #[test]
    fn test_general_enable_progress_bar_valid() {
        assert!(validate_field("general.enable_progress_bar", "true").is_ok());
        assert!(validate_field("general.enable_progress_bar", "false").is_ok());
    }

    #[test]
    fn test_general_enable_progress_bar_invalid() {
        assert!(validate_field("general.enable_progress_bar", "show").is_err());
    }

    // ── general.worker_idle_timeout_seconds ───────────────────────────────────

    #[test]
    fn test_general_worker_idle_timeout_seconds_valid_boundaries() {
        assert!(validate_field("general.worker_idle_timeout_seconds", "10").is_ok());
        assert!(validate_field("general.worker_idle_timeout_seconds", "3600").is_ok());
        assert!(validate_field("general.worker_idle_timeout_seconds", "60").is_ok());
    }

    #[test]
    fn test_general_worker_idle_timeout_seconds_out_of_range() {
        assert!(validate_field("general.worker_idle_timeout_seconds", "9").is_err());
        assert!(validate_field("general.worker_idle_timeout_seconds", "3601").is_err());
    }

    #[test]
    fn test_general_worker_idle_timeout_seconds_parse_error() {
        assert!(validate_field("general.worker_idle_timeout_seconds", "soon").is_err());
    }

    // ── general.max_subtitle_bytes ────────────────────────────────────────────

    #[test]
    fn test_general_max_subtitle_bytes_valid_boundaries() {
        assert!(validate_field("general.max_subtitle_bytes", "1024").is_ok());
        assert!(validate_field("general.max_subtitle_bytes", "1073741824").is_ok());
        assert!(validate_field("general.max_subtitle_bytes", "10485760").is_ok());
    }

    #[test]
    fn test_general_max_subtitle_bytes_out_of_range() {
        assert!(validate_field("general.max_subtitle_bytes", "1023").is_err());
        assert!(validate_field("general.max_subtitle_bytes", "1073741825").is_err());
    }

    #[test]
    fn test_general_max_subtitle_bytes_parse_error() {
        assert!(validate_field("general.max_subtitle_bytes", "huge").is_err());
    }

    // ── general.max_audio_bytes ───────────────────────────────────────────────

    #[test]
    fn test_general_max_audio_bytes_valid_boundaries() {
        assert!(validate_field("general.max_audio_bytes", "1024").is_ok());
        assert!(validate_field("general.max_audio_bytes", "10737418240").is_ok());
        assert!(validate_field("general.max_audio_bytes", "104857600").is_ok());
    }

    #[test]
    fn test_general_max_audio_bytes_out_of_range() {
        assert!(validate_field("general.max_audio_bytes", "1023").is_err());
        assert!(validate_field("general.max_audio_bytes", "10737418241").is_err());
    }

    #[test]
    fn test_general_max_audio_bytes_parse_error() {
        assert!(validate_field("general.max_audio_bytes", "big").is_err());
    }

    // ── parallel.max_workers ──────────────────────────────────────────────────

    #[test]
    fn test_parallel_max_workers_valid_boundaries() {
        assert!(validate_field("parallel.max_workers", "1").is_ok());
        assert!(validate_field("parallel.max_workers", "64").is_ok());
        assert!(validate_field("parallel.max_workers", "4").is_ok());
    }

    #[test]
    fn test_parallel_max_workers_out_of_range() {
        assert!(validate_field("parallel.max_workers", "0").is_err());
        assert!(validate_field("parallel.max_workers", "65").is_err());
    }

    #[test]
    fn test_parallel_max_workers_parse_error() {
        assert!(validate_field("parallel.max_workers", "auto").is_err());
    }

    // ── parallel.task_queue_size ──────────────────────────────────────────────

    #[test]
    fn test_parallel_task_queue_size_valid_boundaries() {
        assert!(validate_field("parallel.task_queue_size", "100").is_ok());
        assert!(validate_field("parallel.task_queue_size", "10000").is_ok());
        assert!(validate_field("parallel.task_queue_size", "1000").is_ok());
    }

    #[test]
    fn test_parallel_task_queue_size_out_of_range() {
        assert!(validate_field("parallel.task_queue_size", "99").is_err());
        assert!(validate_field("parallel.task_queue_size", "10001").is_err());
    }

    #[test]
    fn test_parallel_task_queue_size_parse_error() {
        assert!(validate_field("parallel.task_queue_size", "large").is_err());
    }

    // ── parallel.enable_task_priorities ───────────────────────────────────────

    #[test]
    fn test_parallel_enable_task_priorities_valid() {
        assert!(validate_field("parallel.enable_task_priorities", "true").is_ok());
        assert!(validate_field("parallel.enable_task_priorities", "false").is_ok());
    }

    #[test]
    fn test_parallel_enable_task_priorities_invalid() {
        assert!(validate_field("parallel.enable_task_priorities", "yes_please").is_err());
    }

    // ── parallel.auto_balance_workers ─────────────────────────────────────────

    #[test]
    fn test_parallel_auto_balance_workers_valid() {
        assert!(validate_field("parallel.auto_balance_workers", "true").is_ok());
        assert!(validate_field("parallel.auto_balance_workers", "false").is_ok());
    }

    #[test]
    fn test_parallel_auto_balance_workers_invalid() {
        assert!(validate_field("parallel.auto_balance_workers", "auto").is_err());
    }

    // ── parallel.overflow_strategy ────────────────────────────────────────────

    #[test]
    fn test_parallel_overflow_strategy_valid_all_values() {
        for v in &["Block", "Drop", "Expand"] {
            assert!(
                validate_field("parallel.overflow_strategy", v).is_ok(),
                "strategy={v}"
            );
        }
    }

    #[test]
    fn test_parallel_overflow_strategy_invalid() {
        assert!(validate_field("parallel.overflow_strategy", "block").is_err()); // case-sensitive
        assert!(validate_field("parallel.overflow_strategy", "Ignore").is_err());
    }

    // ── unknown key ───────────────────────────────────────────────────────────

    #[test]
    fn test_validate_unknown_field_returns_error() {
        assert!(validate_field("unknown.field", "value").is_err());
        assert!(validate_field("ai", "openai").is_err());
        assert!(validate_field("", "value").is_err());
    }

    // ── get_field_description ────────────────────────────────────────────────

    #[test]
    fn test_get_field_description_all_known_keys() {
        let known_keys = [
            "ai.provider",
            "ai.model",
            "ai.api_key",
            "ai.base_url",
            "ai.temperature",
            "ai.max_tokens",
            "ai.max_sample_length",
            "ai.retry_attempts",
            "ai.retry_delay_ms",
            "ai.request_timeout_seconds",
            "ai.api_version",
            "sync.default_method",
            "sync.max_offset_seconds",
            "sync.vad.enabled",
            "sync.vad.sensitivity",
            "sync.vad.chunk_size",
            "sync.vad.sample_rate",
            "sync.vad.padding_chunks",
            "sync.vad.min_speech_duration_ms",
            "formats.default_output",
            "formats.preserve_styling",
            "formats.default_encoding",
            "formats.encoding_detection_confidence",
            "general.backup_enabled",
            "general.max_concurrent_jobs",
            "general.task_timeout_seconds",
            "general.enable_progress_bar",
            "general.worker_idle_timeout_seconds",
            "general.max_subtitle_bytes",
            "general.max_audio_bytes",
            "parallel.max_workers",
            "parallel.task_queue_size",
            "parallel.enable_task_priorities",
            "parallel.auto_balance_workers",
            "parallel.overflow_strategy",
        ];
        for key in &known_keys {
            let desc = get_field_description(key);
            assert!(!desc.is_empty(), "empty description for {key}");
            assert_ne!(desc, "Configuration field", "generic fallback for {key}");
        }
    }

    #[test]
    fn test_get_field_description_unknown_key_returns_fallback() {
        assert_eq!(
            get_field_description("unknown.field"),
            "Configuration field"
        );
        assert_eq!(get_field_description(""), "Configuration field");
    }
}
