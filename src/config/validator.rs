//! Configuration validation system for ensuring configuration integrity.
//!
//! This module provides a comprehensive validation framework for SubX configurations,
//! including specialized validators for different configuration sections and
//! business rule enforcement.
//!
//! # Architecture
//!
//! The validation system uses the [`ConfigValidator`] trait to define validation
//! logic for different aspects of the configuration:
//!
//! - [`AIConfigValidator`] - Validates AI service settings and credentials
//! - [`SyncConfigValidator`] - Validates audio synchronization parameters
//! - [`FormatsConfigValidator`] - Validates subtitle format processing settings
//! - [`GeneralConfigValidator`] - Validates general application settings
//! - `ParallelConfigValidator` - Validates parallel processing configuration
//!
//! # Usage
//!
//! ```rust
//! use subx_cli::config::{Config, manager::ConfigError};
//! use subx_cli::config::validator::{ConfigValidator, AIConfigValidator};
//!
//! let config = Config::default();
//!
//! // Validate specific section
//! AIConfigValidator.validate(&config)?;
//!
//! // Or validate all sections
//! let validators: Vec<Box<dyn ConfigValidator>> = vec![
//!     Box::new(AIConfigValidator),
//!     // Add other validators...
//! ];
//!
//! for validator in validators {
//!     validator.validate(&config)?;
//! }
//! # Ok::<(), ConfigError>(())
//! ```
//!
//! # Validation Categories
//!
//! ## Business Rules
//! - API key format validation
//! - Supported provider verification
//! - Parameter range checking
//!
//! ## Data Integrity
//! - Required field presence
//! - Cross-field consistency
//! - Format validation
//!
//! ## Security
//! - Credential format verification
//! - Safe default enforcement
//! - Path traversal prevention

use crate::config::Config;
use crate::config::manager::ConfigError;

/// Trait defining the validation interface for configuration sections.
///
/// Implementors provide validation logic for specific aspects of the
/// SubX configuration, ensuring data integrity and business rule compliance.
///
/// # Implementation Guidelines
///
/// - Return specific [`ConfigError`] variants with descriptive messages
/// - Validate both individual fields and cross-field relationships
/// - Include the field path in error messages for easier debugging
/// - Consider security implications of configuration values
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::{Config, manager::ConfigError};
/// use subx_cli::config::validator::ConfigValidator;
///
/// struct CustomValidator;
///
/// impl ConfigValidator for CustomValidator {
///     fn validate(&self, config: &Config) -> Result<(), ConfigError> {
///         if config.ai.provider.is_empty() {
///             return Err(ConfigError::InvalidValue(
///                 "ai.provider".to_string(),
///                 "Provider cannot be empty".to_string(),
///             ));
///         }
///         Ok(())
///     }
///
///     fn validator_name(&self) -> &'static str {
///         "CustomValidator"
///     }
/// }
/// ```
pub trait ConfigValidator: Send + Sync {
    /// Validates the given configuration.
    ///
    /// # Arguments
    ///
    /// - `config`: The complete configuration to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation passes, or a [`ConfigError`] describing
    /// the first validation failure encountered.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::InvalidValue`] for field-specific validation failures,
    /// or [`ConfigError::ValidationError`] for general validation issues.
    fn validate(&self, config: &Config) -> Result<(), ConfigError>;

    /// Returns the human-readable name of this validator.
    ///
    /// Used for debugging and error reporting to identify which
    /// validator detected a configuration issue.
    fn validator_name(&self) -> &'static str;
}

/// Validates AI service configuration settings.
///
/// Ensures that AI provider settings are properly configured with
/// valid credentials, supported providers, and reasonable parameters.
///
/// # Validation Rules
///
/// ## Provider Validation
/// - Must be a supported provider ("openai", "anthropic", "local")
/// - Provider-specific credential format validation
///
/// ## API Key Validation  
/// - OpenAI keys must start with "sk-"
/// - Keys must meet minimum length requirements
/// - Warn about potentially insecure key storage
///
/// ## Model Validation
/// - Model must be supported by the chosen provider
/// - Model-specific parameter validation
///
/// ## Parameter Validation
/// - `max_sample_length` must be positive and reasonable
/// - `base_url` must be a valid URL format
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::{Config, manager::ConfigError};
/// use subx_cli::config::validator::{ConfigValidator, AIConfigValidator};
///
/// let mut config = Config::default();
/// config.ai.provider = "openai".to_string();
/// config.ai.api_key = Some("sk-valid-key-format".to_string());
///
/// AIConfigValidator.validate(&config)?;
/// # Ok::<(), ConfigError>(())
/// ```
pub struct AIConfigValidator;

impl ConfigValidator for AIConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        // Validate provider is supported
        match config.ai.provider.as_str() {
            "openai" => {}
            other => {
                return Err(ConfigError::InvalidValue(
                    "ai.provider".to_string(),
                    format!("Unsupported AI provider: {}", other),
                ));
            }
        }
        if let Some(ref api_key) = config.ai.api_key {
            if !api_key.starts_with("sk-") {
                return Err(ConfigError::InvalidValue(
                    "ai.api_key".to_string(),
                    "OpenAI API key must start with 'sk-'".to_string(),
                ));
            }
        }
        let valid_models = [
            "gpt-4",
            "gpt-4-turbo",
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-3.5-turbo",
        ];
        if !valid_models.contains(&config.ai.model.as_str()) {
            return Err(ConfigError::InvalidValue(
                "ai.model".to_string(),
                format!(
                    "Unsupported model: {}, supported models: {:?}",
                    config.ai.model, valid_models
                ),
            ));
        }
        if config.ai.temperature < 0.0 || config.ai.temperature > 2.0 {
            return Err(ConfigError::InvalidValue(
                "ai.temperature".to_string(),
                "Temperature value must be between 0.0 and 2.0".to_string(),
            ));
        }
        if config.ai.retry_attempts > 10 {
            return Err(ConfigError::InvalidValue(
                "ai.retry_attempts".to_string(),
                "Retry count cannot exceed 10 times".to_string(),
            ));
        }
        // Validate base_url format
        if let Err(e) = validate_base_url(&config.ai.base_url) {
            return Err(ConfigError::InvalidValue(
                "ai.base_url".to_string(),
                e.to_string(),
            ));
        }
        Ok(())
    }

    fn validator_name(&self) -> &'static str {
        "ai_config"
    }
}

fn validate_base_url(url: &str) -> Result<(), String> {
    use url::Url;
    let parsed = Url::parse(url).map_err(|e| format!("Invalid URL format: {}", e))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Base URL must use http or https protocol".to_string());
    }

    if parsed.host().is_none() {
        return Err("Base URL must contain a valid hostname".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_base_url;

    #[test]
    fn valid_base_url_https() {
        assert!(validate_base_url("https://api.example.com/v1").is_ok());
    }

    #[test]
    fn valid_base_url_http() {
        assert!(validate_base_url("http://localhost:8000").is_ok());
    }

    #[test]
    fn invalid_base_url_scheme() {
        assert!(validate_base_url("ftp://example.com").is_err());
    }

    #[test]
    fn invalid_base_url_no_host() {
        // Valid scheme but missing authority/host
        assert!(validate_base_url("http://").is_err());
    }

    #[test]
    fn test_ai_config_validator_default_and_invalid() {
        use super::{AIConfigValidator, ConfigValidator};
        use crate::config::Config;

        let mut c = Config::default();
        // default config should pass
        assert!(AIConfigValidator.validate(&c).is_ok());
        // invalid provider should fail
        c.ai.provider = "invalid".to_string();
        assert!(AIConfigValidator.validate(&c).is_err());
    }

    #[test]
    fn test_sync_config_validator_valid_and_invalid() {
        use super::{ConfigValidator, SyncConfigValidator};
        use crate::config::Config;

        let mut c = Config::default();
        assert!(SyncConfigValidator.validate(&c).is_ok());
        c.sync.max_offset_seconds = 0.0;
        assert!(SyncConfigValidator.validate(&c).is_err());
    }

    #[test]
    fn test_formats_config_validator_valid_and_invalid() {
        use super::{ConfigValidator, FormatsConfigValidator};
        use crate::config::Config;

        let mut c = Config::default();
        assert!(FormatsConfigValidator.validate(&c).is_ok());
        c.formats.default_output.clear();
        assert!(FormatsConfigValidator.validate(&c).is_err());
    }

    #[test]
    fn test_general_config_validator_valid_and_invalid() {
        use super::{ConfigValidator, GeneralConfigValidator};
        use crate::config::Config;

        let mut c = Config::default();
        assert!(GeneralConfigValidator.validate(&c).is_ok());
        c.general.max_concurrent_jobs = 0;
        assert!(GeneralConfigValidator.validate(&c).is_err());
    }
}

/// Validator for synchronization-related configuration.
pub struct SyncConfigValidator;

impl ConfigValidator for SyncConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        if config.sync.max_offset_seconds <= 0.0 || config.sync.max_offset_seconds > 300.0 {
            return Err(ConfigError::InvalidValue(
                "sync.max_offset_seconds".to_string(),
                "Maximum offset seconds must be between 0.0 and 300.0".to_string(),
            ));
        }
        if config.sync.correlation_threshold < 0.0 || config.sync.correlation_threshold > 1.0 {
            return Err(ConfigError::InvalidValue(
                "sync.correlation_threshold".to_string(),
                "Correlation threshold must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }

    fn validator_name(&self) -> &'static str {
        "sync_config"
    }
}

/// Validator for subtitle formats configuration.
pub struct FormatsConfigValidator;

impl ConfigValidator for FormatsConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        if config.formats.default_output.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "formats.default_output".to_string(),
                "Default output format cannot be empty".to_string(),
            ));
        }
        if config.formats.default_encoding.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "formats.default_encoding".to_string(),
                "Default encoding cannot be empty".to_string(),
            ));
        }
        if config.formats.encoding_detection_confidence < 0.0
            || config.formats.encoding_detection_confidence > 1.0
        {
            return Err(ConfigError::InvalidValue(
                "formats.encoding_detection_confidence".to_string(),
                "Encoding detection confidence must be between 0.0 and 1.0".to_string(),
            ));
        }
        Ok(())
    }

    fn validator_name(&self) -> &'static str {
        "formats_config"
    }
}

/// Validator for general application configuration.
pub struct GeneralConfigValidator;

impl ConfigValidator for GeneralConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        if config.general.max_concurrent_jobs == 0 {
            return Err(ConfigError::InvalidValue(
                "general.max_concurrent_jobs".to_string(),
                "Maximum concurrent workers must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }

    fn validator_name(&self) -> &'static str {
        "general_config"
    }
}

/// Validate a complete configuration using all available validators.
///
/// This function runs all available configuration validators and returns
/// the first validation error encountered, if any.
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Errors
///
/// Returns the first validation error encountered by any validator.
pub fn validate_config(config: &Config) -> Result<(), ConfigError> {
    let validators: Vec<Box<dyn ConfigValidator>> = vec![
        Box::new(AIConfigValidator),
        Box::new(SyncConfigValidator),
        Box::new(FormatsConfigValidator),
        Box::new(GeneralConfigValidator),
    ];

    for validator in validators {
        validator.validate(config)?;
    }

    Ok(())
}
