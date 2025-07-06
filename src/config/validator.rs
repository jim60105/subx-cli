//! High-level configuration validation for configuration sections.
//!
//! This module provides validation for complete configuration sections and
//! the entire configuration structure. It builds upon the low-level validation
//! functions from the [`crate::config::validation`] module.
//!
//! # Architecture
//!
//! - [`crate::config::validation`] - Low-level validation functions for individual values
//! - [`crate::config::validator`] (this module) - High-level configuration section validators
//! - [`crate::config::field_validator`] - Key-value validation for configuration service

use super::validation::*;
use crate::Result;
use crate::config::Config;
use crate::config::{
    AIConfig, FormatsConfig, GeneralConfig, ParallelConfig, SyncConfig, VadConfig,
};
use crate::error::SubXError;

/// Validate the complete configuration.
///
/// This function validates all configuration sections and their
/// interdependencies.
///
/// # Arguments
/// * `config` - The configuration to validate
///
/// # Errors
/// Returns the first validation error encountered.
pub fn validate_config(config: &Config) -> Result<()> {
    validate_ai_config(&config.ai)?;
    validate_sync_config(&config.sync)?;
    validate_general_config(&config.general)?;
    validate_formats_config(&config.formats)?;
    validate_parallel_config(&config.parallel)?;

    // Cross-section validation
    validate_config_consistency(config)?;

    Ok(())
}

/// Validate AI configuration section.
pub fn validate_ai_config(ai_config: &AIConfig) -> Result<()> {
    validate_non_empty_string(&ai_config.provider, "AI provider")?;

    // Validate provider-specific settings
    match ai_config.provider.as_str() {
        "openai" => {
            if let Some(api_key) = &ai_config.api_key {
                if !api_key.is_empty() {
                    validate_api_key(api_key)?;
                    if !api_key.starts_with("sk-") {
                        return Err(SubXError::config("OpenAI API key must start with 'sk-'"));
                    }
                }
            }
            validate_ai_model(&ai_config.model)?;
            validate_temperature(ai_config.temperature)?;
            validate_positive_number(ai_config.max_tokens as f64)?;

            if !ai_config.base_url.is_empty() {
                validate_url_format(&ai_config.base_url)?;
            }
        }
        "openrouter" => {
            if let Some(api_key) = &ai_config.api_key {
                if !api_key.is_empty() {
                    validate_api_key(api_key)?;
                    // OpenRouter API keys have no specific prefix requirement
                }
            }
            validate_ai_model(&ai_config.model)?;
            validate_temperature(ai_config.temperature)?;
            validate_positive_number(ai_config.max_tokens as f64)?;

            if !ai_config.base_url.is_empty() {
                validate_url_format(&ai_config.base_url)?;
            }
        }
        "anthropic" => {
            if let Some(api_key) = &ai_config.api_key {
                if !api_key.is_empty() {
                    validate_api_key(api_key)?;
                }
            }
            validate_ai_model(&ai_config.model)?;
            validate_temperature(ai_config.temperature)?;
        }
        _ => {
            return Err(SubXError::config(format!(
                "Unsupported AI provider: {}. Supported providers: openai, openrouter, anthropic",
                ai_config.provider
            )));
        }
    }

    // Validate retry settings
    validate_positive_number(ai_config.retry_attempts as f64)?;
    if ai_config.retry_attempts > 10 {
        return Err(SubXError::config("Retry count cannot exceed 10 times"));
    }

    // Validate timeout settings
    validate_range(ai_config.request_timeout_seconds as f64, 10.0, 600.0)
        .map_err(|_| SubXError::config("Request timeout must be between 10 and 600 seconds"))?;

    Ok(())
}

/// Validate sync configuration section.
pub fn validate_sync_config(sync_config: &SyncConfig) -> Result<()> {
    // Delegate to SyncConfig's validation with enhancements
    sync_config.validate()
}

/// Validate general configuration section.
pub fn validate_general_config(general_config: &GeneralConfig) -> Result<()> {
    // Validate concurrent jobs
    validate_positive_number(general_config.max_concurrent_jobs as f64)?;
    if general_config.max_concurrent_jobs > 64 {
        return Err(SubXError::config(
            "Maximum concurrent jobs should not exceed 64",
        ));
    }

    // Validate timeout settings
    validate_range(general_config.task_timeout_seconds as f64, 30.0, 3600.0)
        .map_err(|_| SubXError::config("Task timeout must be between 30 and 3600 seconds"))?;

    validate_range(
        general_config.worker_idle_timeout_seconds as f64,
        10.0,
        3600.0,
    )
    .map_err(|_| SubXError::config("Worker idle timeout must be between 10 and 3600 seconds"))?;

    Ok(())
}

/// Validate formats configuration section.
pub fn validate_formats_config(formats_config: &FormatsConfig) -> Result<()> {
    // Check default output format
    validate_non_empty_string(&formats_config.default_output, "Default output format")?;
    validate_enum(
        &formats_config.default_output,
        &["srt", "ass", "vtt", "webvtt"],
    )?;

    // Check default encoding
    validate_non_empty_string(&formats_config.default_encoding, "Default encoding")?;
    validate_enum(
        &formats_config.default_encoding,
        &["utf-8", "gbk", "big5", "shift_jis"],
    )?;

    // Check encoding detection confidence
    validate_range(formats_config.encoding_detection_confidence, 0.0, 1.0).map_err(|_| {
        SubXError::config("Encoding detection confidence must be between 0.0 and 1.0")
    })?;

    Ok(())
}

/// Validate parallel processing configuration.
pub fn validate_parallel_config(parallel_config: &ParallelConfig) -> Result<()> {
    // Check max workers
    validate_positive_number(parallel_config.max_workers as f64)?;
    if parallel_config.max_workers > 64 {
        return Err(SubXError::config("Maximum workers should not exceed 64"));
    }

    // Check task queue size
    validate_positive_number(parallel_config.task_queue_size as f64)?;
    if parallel_config.task_queue_size < 100 {
        return Err(SubXError::config("Task queue size should be at least 100"));
    }

    Ok(())
}

/// Validate configuration consistency across sections.
fn validate_config_consistency(config: &Config) -> Result<()> {
    // Example: Ensure AI is properly configured if using AI features
    if config.ai.provider == "openai" {
        if let Some(api_key) = &config.ai.api_key {
            if api_key.is_empty() {
                return Err(SubXError::config(
                    "OpenAI provider is selected but API key is empty",
                ));
            }
        }
        // Note: We don't require API key for default config to allow basic operation
    }

    // Ensure reasonable resource allocation
    if config.parallel.max_workers > config.general.max_concurrent_jobs {
        log::warn!(
            "Parallel max_workers ({}) exceeds general max_concurrent_jobs ({})",
            config.parallel.max_workers,
            config.general.max_concurrent_jobs
        );
    }

    Ok(())
}

impl SyncConfig {
    /// Validate the sync configuration for correctness.
    ///
    /// Checks all sync-related configuration parameters to ensure they
    /// are within valid ranges and have acceptable values.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or an error describing
    /// the validation failure.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - `default_method` is not one of the supported methods
    /// - `max_offset_seconds` is outside the valid range
    /// - VAD configuration validation fails
    pub fn validate(&self) -> Result<()> {
        // Validate default_method
        validate_enum(&self.default_method, &["vad", "auto", "manual"])?;

        // Validate max_offset_seconds
        validate_positive_number(self.max_offset_seconds)?;
        if self.max_offset_seconds > 3600.0 {
            return Err(SubXError::config(
                "sync.max_offset_seconds should not exceed 3600 seconds (1 hour). If a larger value is needed, please verify the sync requirements are reasonable.",
            ));
        }

        // Provide recommendations for common use cases
        if self.max_offset_seconds < 5.0 {
            log::warn!(
                "sync.max_offset_seconds is set to {:.1}s which may be too small. Consider using 30.0-60.0 seconds.",
                self.max_offset_seconds
            );
        } else if self.max_offset_seconds > 600.0 && self.max_offset_seconds <= 3600.0 {
            log::warn!(
                "sync.max_offset_seconds is set to {:.1}s which is quite large. Please confirm this meets your requirements.",
                self.max_offset_seconds
            );
        }

        // Validate sub-configurations
        self.vad.validate()?;

        Ok(())
    }
}

impl VadConfig {
    /// Validate the local VAD configuration for correctness.
    ///
    /// Ensures that all VAD-related parameters are within acceptable
    /// ranges and have valid values for audio processing.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or an error describing
    /// the validation failure.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - `sensitivity` is outside the valid range (0.0-1.0)
    pub fn validate(&self) -> Result<()> {
        // Validate sensitivity range
        if !(0.0..=1.0).contains(&self.sensitivity) {
            return Err(SubXError::config(
                "VAD sensitivity must be between 0.0 and 1.0",
            ));
        }
        // Validate padding_chunks
        if self.padding_chunks > 10 {
            return Err(SubXError::config("VAD padding_chunks must not exceed 10"));
        }
        // Validate minimum speech duration
        if self.min_speech_duration_ms > 5000 {
            return Err(SubXError::config(
                "VAD min_speech_duration_ms must not exceed 5000ms",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AIConfig, Config, SyncConfig, VadConfig};

    #[test]
    fn test_validate_default_config() {
        let config = Config::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_valid() {
        let mut ai_config = AIConfig::default();
        ai_config.provider = "openai".to_string();
        ai_config.api_key = Some("sk-test123456789".to_string());
        ai_config.temperature = 0.8;
        assert!(validate_ai_config(&ai_config).is_ok());

        // openrouter test
        let mut ai_config = AIConfig::default();
        ai_config.provider = "openrouter".to_string();
        ai_config.api_key = Some("test-openrouter-key".to_string());
        ai_config.model = "deepseek/deepseek-r1-0528:free".to_string();
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_invalid_provider() {
        let mut ai_config = AIConfig::default();
        ai_config.provider = "invalid".to_string();
        let err = validate_ai_config(&ai_config).unwrap_err();
        assert!(err.to_string().contains(
            "Unsupported AI provider: invalid. Supported providers: openai, openrouter, anthropic"
        ));
    }

    #[test]
    fn test_validate_ai_config_invalid_temperature() {
        let mut ai_config = AIConfig::default();
        ai_config.provider = "openai".to_string();
        ai_config.temperature = 3.0; // Too high
        assert!(validate_ai_config(&ai_config).is_err());
    }

    #[test]
    fn test_validate_ai_config_invalid_openai_key() {
        let mut ai_config = AIConfig::default();
        ai_config.provider = "openai".to_string();
        ai_config.api_key = Some("invalid-key".to_string());
        assert!(validate_ai_config(&ai_config).is_err());
    }

    #[test]
    fn test_validate_sync_config_valid() {
        let sync_config = SyncConfig::default();
        assert!(validate_sync_config(&sync_config).is_ok());
    }

    #[test]
    fn test_validate_vad_config_invalid_sensitivity() {
        let mut vad_config = VadConfig::default();
        vad_config.sensitivity = 1.5; // Too high (should be 0.0-1.0)
        assert!(vad_config.validate().is_err());
    }

    #[test]
    fn test_validate_config_consistency() {
        let mut config = Config::default();
        config.ai.provider = "openai".to_string();
        config.ai.api_key = Some("".to_string()); // Empty API key should fail
        assert!(validate_config(&config).is_err());

        // Valid case with proper API key
        config.ai.api_key = Some("sk-valid123".to_string());
        assert!(validate_config(&config).is_ok());

        // Valid case with no API key (default state)
        config.ai.api_key = None;
        assert!(validate_config(&config).is_ok());
    }
}
