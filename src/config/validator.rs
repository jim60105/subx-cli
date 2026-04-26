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
    AIConfig, FormatsConfig, GeneralConfig, ParallelConfig, SyncConfig, TranslationConfig,
    VadConfig,
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
    validate_translation_config(&config.translation)?;

    // Cross-section validation
    validate_config_consistency(config)?;

    Ok(())
}

/// Validate AI configuration section.
pub fn validate_ai_config(ai_config: &AIConfig) -> Result<()> {
    validate_non_empty_string(&ai_config.provider, "AI provider")?;

    // Validation arms key off the canonical provider value so callers may
    // pass either `local` or the alias `ollama` without divergence.
    let canonical = crate::config::field_validator::normalize_ai_provider(&ai_config.provider);

    // Validate provider-specific settings
    match canonical.as_str() {
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
                require_https_for_hosted_provider(&ai_config.base_url, "openai")?;
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
                require_https_for_hosted_provider(&ai_config.base_url, "openrouter")?;
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
        "azure-openai" => {
            if let Some(api_key) = &ai_config.api_key {
                if !api_key.is_empty() {
                    validate_api_key(api_key)?;
                }
            }
            validate_ai_model(&ai_config.model)?;
            validate_temperature(ai_config.temperature)?;
            validate_positive_number(ai_config.max_tokens as f64)?;
            if let Some(ver) = &ai_config.api_version {
                if ver.trim().is_empty() {
                    return Err(SubXError::config(
                        "Azure OpenAI api_version must not be empty",
                    ));
                }
            }
            if !ai_config.base_url.is_empty() {
                validate_url_format(&ai_config.base_url)?;
                require_https_for_hosted_provider(&ai_config.base_url, "azure-openai")?;
            }
        }
        "local" => {
            // `api_key` is optional for the local provider. When supplied,
            // it is run through the same permissive `validate_api_key`
            // helper used by other providers (no prefix requirement).
            if let Some(api_key) = &ai_config.api_key {
                if !api_key.is_empty() {
                    validate_api_key(api_key)?;
                }
            }
            // `base_url` is required for the local provider — there is no
            // safe default because every runtime listens on a different
            // port. Both http:// and https:// are accepted; the local
            // provider is endpoint-agnostic and may target loopback, LAN,
            // VPN, or public hosts.
            if ai_config.base_url.trim().is_empty() {
                return Err(SubXError::config(
                    "ai.base_url is required when ai.provider is `local` \
                     (e.g. http://localhost:11434/v1 for Ollama, \
                     http://localhost:1234/v1 for LM Studio, \
                     http://localhost:8080/v1 for llama.cpp's llama-server)",
                ));
            }
            validate_url_format(&ai_config.base_url)?;
            validate_ai_model(&ai_config.model)?;
            validate_temperature(ai_config.temperature)?;
            validate_positive_number(ai_config.max_tokens as f64)?;
        }
        _ => {
            return Err(SubXError::config(format!(
                "Unsupported AI provider: {}. Supported providers: openai, openrouter, anthropic, azure-openai, local",
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

/// Reject any user-set `ai.base_url` whose scheme is not `https://` for
/// hosted providers (`openai`, `openrouter`, `azure-openai`).
///
/// The error message names the field, the unsupported scheme, states that
/// HTTPS is required for hosted providers, and appends the canonical
/// `local_provider_hint()` so users who actually wanted to call an
/// OpenAI-compatible local or LAN endpoint get a one-line fix.
fn require_https_for_hosted_provider(base_url: &str, provider: &str) -> Result<()> {
    let parsed = url::Url::parse(base_url)
        .map_err(|_| SubXError::config(format!("Invalid URL format: {base_url}")))?;
    let scheme = parsed.scheme();
    if scheme != "https" {
        return Err(SubXError::config(format!(
            "ai.base_url uses unsupported scheme `{scheme}://` for hosted provider `{provider}`; \
             hosted providers require HTTPS. {}",
            crate::services::ai::local_provider_hint(),
        )));
    }
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

/// Validate translation configuration section.
///
/// Ensures `batch_size` is a positive integer and that
/// `default_target_language`, when set, is a non-empty trimmed string.
pub fn validate_translation_config(translation_config: &TranslationConfig) -> Result<()> {
    if translation_config.batch_size == 0 {
        return Err(SubXError::config(
            "translation.batch_size must be greater than zero",
        ));
    }
    if translation_config.batch_size > 1000 {
        return Err(SubXError::config(
            "translation.batch_size should not exceed 1000",
        ));
    }
    if let Some(lang) = &translation_config.default_target_language {
        if lang.trim().is_empty() {
            return Err(SubXError::config(
                "translation.default_target_language must not be empty",
            ));
        }
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
        let ai_config = AIConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test123456789".to_string()),
            temperature: 0.8,
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());

        // openrouter test
        let ai_config = AIConfig {
            provider: "openrouter".to_string(),
            api_key: Some("test-openrouter-key".to_string()),
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());

        // azure-openai test
        let ai_config = AIConfig {
            provider: "azure-openai".to_string(),
            api_key: Some("azure-key-123".to_string()),
            model: "dep123".to_string(),
            api_version: Some("2025-04-01-preview".to_string()),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_invalid_provider() {
        let ai_config = AIConfig {
            provider: "invalid".to_string(),
            ..Default::default()
        };
        let err = validate_ai_config(&ai_config).unwrap_err();
        assert!(err.to_string().contains(
            "Unsupported AI provider: invalid. Supported providers: openai, openrouter, anthropic, azure-openai, local"
        ));
    }

    // ── local provider arm ───────────────────────────────────────────────────

    #[test]
    fn test_validate_ai_config_local_without_api_key() {
        let ai_config = AIConfig {
            provider: "local".to_string(),
            api_key: None,
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3.1:8b-instruct".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_local_with_empty_api_key() {
        let ai_config = AIConfig {
            provider: "local".to_string(),
            api_key: Some("".to_string()),
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3.1:8b-instruct".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_local_rejects_empty_base_url() {
        let ai_config = AIConfig {
            provider: "local".to_string(),
            api_key: None,
            base_url: "".to_string(),
            model: "llama3.1".to_string(),
            ..Default::default()
        };
        let err = validate_ai_config(&ai_config).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ai.base_url"),
            "error must name ai.base_url field, got: {msg}"
        );
        assert!(
            msg.contains("local"),
            "error must mention `local` provider, got: {msg}"
        );
    }

    #[test]
    fn test_validate_ai_config_local_accepts_lan_http_base_url() {
        let ai_config = AIConfig {
            provider: "local".to_string(),
            api_key: None,
            base_url: "http://192.168.50.50:11434/v1".to_string(),
            model: "llama3.1".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_local_accepts_https_tailnet_base_url() {
        let ai_config = AIConfig {
            provider: "local".to_string(),
            api_key: None,
            base_url: "https://ollama.tailnet.ts.net/v1".to_string(),
            model: "qwen2.5:7b".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_local_accepts_ollama_alias_via_normalize() {
        // The validation arm keys off the canonical value, so an `ollama`
        // alias (as it would arrive from `SUBX_AI_PROVIDER=ollama` before
        // service.rs canonicalizes it) is treated identically to `local`.
        let ai_config = AIConfig {
            provider: "ollama".to_string(),
            api_key: None,
            base_url: "http://localhost:11434/v1".to_string(),
            model: "llama3.1".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    // ── §1.9: hosted providers require HTTPS for user-set base_url ───────────

    #[test]
    fn test_validate_ai_config_openai_rejects_http_base_url() {
        let ai_config = AIConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test1234567890".to_string()),
            base_url: "http://localhost:11434/v1".to_string(),
            ..Default::default()
        };
        let err = validate_ai_config(&ai_config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ai.base_url"), "msg={msg}");
        assert!(msg.contains("http"), "msg={msg}");
        assert!(msg.contains("HTTPS"), "msg={msg}");
        // Hint mentions both `local` and `ollama`.
        assert!(msg.contains("local"), "msg={msg}");
        assert!(msg.contains("ollama"), "msg={msg}");
    }

    #[test]
    fn test_validate_ai_config_openrouter_rejects_http_base_url() {
        let ai_config = AIConfig {
            provider: "openrouter".to_string(),
            api_key: Some("test-openrouter-key".to_string()),
            base_url: "http://x.example.com/v1".to_string(),
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            ..Default::default()
        };
        let err = validate_ai_config(&ai_config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ai.base_url"), "msg={msg}");
        assert!(msg.contains("HTTPS"), "msg={msg}");
        assert!(msg.contains("local") && msg.contains("ollama"), "msg={msg}");
    }

    #[test]
    fn test_validate_ai_config_azure_openai_rejects_http_base_url() {
        let ai_config = AIConfig {
            provider: "azure-openai".to_string(),
            api_key: Some("azure-key-123".to_string()),
            model: "dep123".to_string(),
            api_version: Some("2025-04-01-preview".to_string()),
            base_url: "http://example.openai.azure.com".to_string(),
            ..Default::default()
        };
        let err = validate_ai_config(&ai_config).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ai.base_url"), "msg={msg}");
        assert!(msg.contains("HTTPS"), "msg={msg}");
        assert!(msg.contains("local") && msg.contains("ollama"), "msg={msg}");
    }

    #[test]
    fn test_validate_ai_config_hosted_https_base_url_accepted() {
        for provider in &["openai", "openrouter", "azure-openai"] {
            let ai_config = AIConfig {
                provider: provider.to_string(),
                api_key: Some(if *provider == "openai" {
                    "sk-test-key-12345".to_string()
                } else {
                    "any-key-12345".to_string()
                }),
                base_url: "https://example.com/v1".to_string(),
                api_version: if *provider == "azure-openai" {
                    Some("2025-04-01-preview".to_string())
                } else {
                    None
                },
                ..Default::default()
            };
            assert!(
                validate_ai_config(&ai_config).is_ok(),
                "provider={provider} should accept https base_url"
            );
        }
    }

    #[test]
    fn test_validate_ai_config_hosted_default_base_url_unaffected() {
        // The default `https://api.openai.com/v1` SHALL still pass for
        // hosted providers without the user explicitly setting base_url.
        let ai_config = AIConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test-key".to_string()),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_ok());
    }

    #[test]
    fn test_validate_ai_config_local_unaffected_by_https_rule() {
        // Regression: the HTTPS-required rule must not bleed into local.
        let http = AIConfig {
            provider: "local".to_string(),
            base_url: "http://10.0.0.5:11434/v1".to_string(),
            model: "llama3.1".to_string(),
            ..Default::default()
        };
        let https = AIConfig {
            provider: "local".to_string(),
            base_url: "https://internal.example.com/v1".to_string(),
            model: "llama3.1".to_string(),
            ..Default::default()
        };
        assert!(validate_ai_config(&http).is_ok());
        assert!(validate_ai_config(&https).is_ok());
    }

    #[test]
    fn test_validate_ai_config_invalid_temperature() {
        let ai_config = AIConfig {
            provider: "openai".to_string(),
            temperature: 3.0, // Too high
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_err());
    }

    #[test]
    fn test_validate_ai_config_invalid_openai_key() {
        let ai_config = AIConfig {
            provider: "openai".to_string(),
            api_key: Some("invalid-key".to_string()),
            ..Default::default()
        };
        assert!(validate_ai_config(&ai_config).is_err());
    }

    #[test]
    fn test_validate_sync_config_valid() {
        let sync_config = SyncConfig::default();
        assert!(validate_sync_config(&sync_config).is_ok());
    }

    #[test]
    fn test_validate_vad_config_invalid_sensitivity() {
        let vad_config = VadConfig {
            sensitivity: 1.5, // Too high (should be 0.0-1.0)
            ..Default::default()
        };
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
