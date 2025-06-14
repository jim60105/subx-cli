//! Configuration validation module providing validation rules and constraints.
//!
//! This module provides comprehensive validation functionality for configuration
//! values, ensuring that all settings meet business requirements and system
//! constraints before being used by the application.

use crate::Result;
use crate::config::Config;
use crate::config::{SyncConfig, VadConfig, WhisperConfig};
use crate::error::SubXError;

/// Trait defining the validation interface for configuration sections.
pub trait ConfigValidator {
    /// Validate the configuration and return any errors found.
    fn validate(&self, config: &Config) -> Result<()>;
}

/// AI configuration validator.
pub struct AIValidator;

impl ConfigValidator for AIValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Check AI provider
        match config.ai.provider.as_str() {
            "openai" | "anthropic" => {}
            _ => {
                return Err(SubXError::config(format!(
                    "Unsupported AI provider: {}",
                    config.ai.provider
                )));
            }
        }

        // Check API key format for OpenAI
        if config.ai.provider == "openai" {
            if let Some(api_key) = config.ai.api_key.as_deref() {
                if !api_key.starts_with("sk-") && !api_key.is_empty() {
                    return Err(SubXError::config("OpenAI API key must start with 'sk-'"));
                }
            }
        }

        // Check temperature range
        if config.ai.temperature < 0.0 || config.ai.temperature > 2.0 {
            return Err(SubXError::config(
                "Temperature value must be between 0.0 and 2.0",
            ));
        }

        // Check retry attempts
        if config.ai.retry_attempts > 10 {
            return Err(SubXError::config("Retry count cannot exceed 10 times"));
        }

        Ok(())
    }
}

/// Sync configuration validator.
pub struct SyncValidator;

impl ConfigValidator for SyncValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Delegate to SyncConfig's own validation logic
        config.sync.validate()
    }
}

impl SyncConfig {
    /// 驗證同步配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 驗證 default_method
        match self.default_method.as_str() {
            "whisper" | "vad" => {}
            _ => {
                return Err(SubXError::config(
                    "sync.default_method must be one of: whisper, vad",
                ));
            }
        }

        // 驗證 analysis_window_seconds
        if self.analysis_window_seconds == 0 || self.analysis_window_seconds > 300 {
            return Err(SubXError::config(
                "sync.analysis_window_seconds must be between 1 and 300",
            ));
        }

        // 驗證 max_offset_seconds
        if self.max_offset_seconds <= 0.0 || self.max_offset_seconds > 3600.0 {
            return Err(SubXError::config(
                "sync.max_offset_seconds must be between 0.1 and 3600.0",
            ));
        }

        // 驗證子配置
        self.whisper.validate()?;
        self.vad.validate()?;

        Ok(())
    }
}

impl WhisperConfig {
    /// 驗證 Whisper API 配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 驗證 temperature
        if self.temperature < 0.0 || self.temperature > 1.0 {
            return Err(SubXError::config(
                "sync.whisper.temperature must be between 0.0 and 1.0",
            ));
        }

        // 驗證 timeout_seconds
        if self.timeout_seconds == 0 || self.timeout_seconds > 300 {
            return Err(SubXError::config(
                "sync.whisper.timeout_seconds must be between 1 and 300",
            ));
        }

        // 驗證 max_retries
        if self.max_retries > 10 {
            return Err(SubXError::config(
                "sync.whisper.max_retries must be 10 or less",
            ));
        }

        // 驗證 min_confidence_threshold
        if self.min_confidence_threshold < 0.0 || self.min_confidence_threshold > 1.0 {
            return Err(SubXError::config(
                "sync.whisper.min_confidence_threshold must be between 0.0 and 1.0",
            ));
        }

        Ok(())
    }
}

impl VadConfig {
    /// 驗證本地 VAD 配置的有效性
    pub fn validate(&self) -> Result<()> {
        // 驗證 sensitivity
        if self.sensitivity < 0.0 || self.sensitivity > 1.0 {
            return Err(SubXError::config(
                "sync.vad.sensitivity must be between 0.0 and 1.0",
            ));
        }

        // 驗證 chunk_size (must be power of 2 and reasonable size)
        if self.chunk_size < 256 || self.chunk_size > 2048 || !self.chunk_size.is_power_of_two() {
            return Err(SubXError::config(
                "sync.vad.chunk_size must be a power of 2 between 256 and 2048",
            ));
        }

        // 驗證 sample_rate
        match self.sample_rate {
            8000 | 16000 | 32000 | 44100 | 48000 => {}
            _ => {
                return Err(SubXError::config(
                    "sync.vad.sample_rate must be one of: 8000, 16000, 32000, 44100, 48000",
                ));
            }
        }

        Ok(())
    }
}

/// Formats configuration validator.
pub struct FormatsValidator;

impl ConfigValidator for FormatsValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Check default output format
        if config.formats.default_output.is_empty() {
            return Err(SubXError::config("Default output format cannot be empty"));
        }

        // Check default encoding
        if config.formats.default_encoding.is_empty() {
            return Err(SubXError::config("Default encoding cannot be empty"));
        }

        // Check encoding detection confidence
        if config.formats.encoding_detection_confidence < 0.0
            || config.formats.encoding_detection_confidence > 1.0
        {
            return Err(SubXError::config(
                "Encoding detection confidence must be between 0.0 and 1.0",
            ));
        }

        Ok(())
    }
}

/// Parallel configuration validator.
pub struct ParallelValidator;

impl ConfigValidator for ParallelValidator {
    fn validate(&self, config: &Config) -> Result<()> {
        // Check max workers
        if config.parallel.max_workers == 0 {
            return Err(SubXError::config(
                "Maximum concurrent workers must be greater than 0",
            ));
        }

        Ok(())
    }
}

/// Validate the complete configuration.
///
/// This function runs all configuration validators and returns the first
/// error encountered, or Ok(()) if all validation passes.
pub fn validate_config(config: &Config) -> Result<()> {
    AIValidator.validate(config)?;
    SyncValidator.validate(config)?;
    FormatsValidator.validate(config)?;
    ParallelValidator.validate(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_validate_default_config() {
        let config = Config::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_ai_provider() {
        let mut config = Config::default();
        config.ai.provider = "invalid".to_string();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_invalid_temperature() {
        let mut config = Config::default();
        config.ai.temperature = 3.0; // Too high
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_invalid_correlation_threshold() {
        let mut config = Config::default();
        config.sync.correlation_threshold = 1.5; // Too high
        assert!(validate_config(&config).is_err());
    }
}
