//! Configuration validation module providing validation rules and constraints.
//!
//! This module provides comprehensive validation functionality for configuration
//! values, ensuring that all settings meet business requirements and system
//! constraints before being used by the application.

use crate::Result;
use crate::config::Config;
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
            if let Some(ref api_key) = config.ai.api_key {
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
        // Check max offset seconds
        if config.sync.max_offset_seconds < 0.0 || config.sync.max_offset_seconds > 300.0 {
            return Err(SubXError::config(
                "Maximum offset seconds must be between 0.0 and 300.0",
            ));
        }

        // Check correlation threshold
        if config.sync.correlation_threshold < 0.0 || config.sync.correlation_threshold > 1.0 {
            return Err(SubXError::config(
                "Correlation threshold must be between 0.0 and 1.0",
            ));
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
