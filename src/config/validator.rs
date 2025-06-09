//! Configuration validators for unified configuration management.

use crate::config::Config;
use crate::config::manager::ConfigError;

/// Trait for configuration validation.
pub trait ConfigValidator: Send + Sync {
    /// Validate the given configuration.
    fn validate(&self, config: &Config) -> Result<(), ConfigError>;
    /// Return the name of the validator.
    fn validator_name(&self) -> &'static str;
}

/// Validator for AI-related configuration.
pub struct AIConfigValidator;

impl ConfigValidator for AIConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        // 驗證 provider 是否受支援
        match config.ai.provider.as_str() {
            "openai" => {}
            other => {
                return Err(ConfigError::InvalidValue(
                    "ai.provider".to_string(),
                    format!("不支援的 AI 提供商: {}", other),
                ));
            }
        }
        if let Some(ref api_key) = config.ai.api_key {
            if !api_key.starts_with("sk-") {
                return Err(ConfigError::InvalidValue(
                    "ai.api_key".to_string(),
                    "OpenAI API 金鑰必須以 'sk-' 開頭".to_string(),
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
                    "不支援的模型: {}，支援的模型: {:?}",
                    config.ai.model, valid_models
                ),
            ));
        }
        if config.ai.temperature < 0.0 || config.ai.temperature > 2.0 {
            return Err(ConfigError::InvalidValue(
                "ai.temperature".to_string(),
                "溫度值必須在 0.0 到 2.0 之間".to_string(),
            ));
        }
        if config.ai.retry_attempts > 10 {
            return Err(ConfigError::InvalidValue(
                "ai.retry_attempts".to_string(),
                "重試次數不能超過 10 次".to_string(),
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
    let parsed = Url::parse(url).map_err(|e| format!("無效的 URL 格式: {}", e))?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("base URL 必須使用 http 或 https 協定".to_string());
    }

    if parsed.host().is_none() {
        return Err("base URL 必須包含有效的主機名稱".to_string());
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
                "最大偏移秒數必須在 0.0 到 300.0 之間".to_string(),
            ));
        }
        if config.sync.correlation_threshold < 0.0 || config.sync.correlation_threshold > 1.0 {
            return Err(ConfigError::InvalidValue(
                "sync.correlation_threshold".to_string(),
                "相關性閾值必須在 0.0 到 1.0 之間".to_string(),
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
                "預設輸出格式不能為空".to_string(),
            ));
        }
        if config.formats.default_encoding.trim().is_empty() {
            return Err(ConfigError::InvalidValue(
                "formats.default_encoding".to_string(),
                "預設編碼不能為空".to_string(),
            ));
        }
        if config.formats.encoding_detection_confidence < 0.0
            || config.formats.encoding_detection_confidence > 1.0
        {
            return Err(ConfigError::InvalidValue(
                "formats.encoding_detection_confidence".to_string(),
                "編碼檢測信心度必須介於 0.0 和 1.0 之間".to_string(),
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
                "最大併發工作數必須大於 0".to_string(),
            ));
        }
        Ok(())
    }

    fn validator_name(&self) -> &'static str {
        "general_config"
    }
}
