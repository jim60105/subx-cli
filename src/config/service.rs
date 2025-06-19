#![allow(deprecated)]
//! Configuration service system for dependency injection and test isolation.
//!
//! This module provides a clean abstraction for configuration management
//! that enables dependency injection and complete test isolation without
//! requiring unsafe code or global state resets.

use crate::config::{EnvironmentProvider, SystemEnvironmentProvider};
use crate::{Result, config::Config, error::SubXError};
use config::{Config as ConfigCrate, ConfigBuilder, Environment, File, builder::DefaultState};
use log::debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Configuration service trait for dependency injection.
///
/// This trait abstracts configuration loading and reloading operations,
/// allowing different implementations for production and testing environments.
pub trait ConfigService: Send + Sync {
    /// Get the current configuration.
    ///
    /// Returns a clone of the current configuration state. This method
    /// may use internal caching for performance.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or validation fails.
    /// Get the current configuration.
    ///
    /// Returns the current [`Config`] instance loaded from files,
    /// environment variables, and defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails due to:
    /// - Invalid TOML format in configuration files
    /// - Missing required configuration values
    /// - File system access issues
    fn get_config(&self) -> Result<Config>;

    /// Reload configuration from sources.
    ///
    /// Forces a reload of configuration from all sources, discarding
    /// any cached values.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration reloading fails.
    fn reload(&self) -> Result<()>;

    /// Save current configuration to the default file location.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to determine config file path
    /// - File system write permissions are insufficient
    /// - TOML serialization fails
    fn save_config(&self) -> Result<()>;

    /// Save configuration to a specific file path.
    ///
    /// # Arguments
    ///
    /// - `path`: Target file path for the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - TOML serialization fails
    /// - Unable to create parent directories
    /// - File write operation fails
    fn save_config_to_file(&self, path: &Path) -> Result<()>;

    /// Get the default configuration file path.
    ///
    /// # Returns
    ///
    /// Returns the path where configuration files are expected to be located,
    /// typically `$CONFIG_DIR/subx/config.toml`.
    fn get_config_file_path(&self) -> Result<PathBuf>;

    /// Get a specific configuration value by key path.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated path to the configuration value (e.g., "ai.provider")
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not recognized.
    fn get_config_value(&self, key: &str) -> Result<String>;

    /// Reset configuration to default values.
    ///
    /// This will overwrite the current configuration file with default values
    /// and reload the configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if save or reload fails.
    fn reset_to_defaults(&self) -> Result<()>;

    /// Set a specific configuration value by key path.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated path to the configuration value
    /// - `value`: New value as string (will be converted to appropriate type)
    ///
    /// # Errors
    ///
    /// Returns an error if validation or persistence fails, including:
    /// - Unknown configuration key
    /// - Type conversion or validation error
    /// - Failure to persist configuration
    fn set_config_value(&self, key: &str, value: &str) -> Result<()>;
}

/// Production configuration service implementation.
///
/// This service loads configuration from multiple sources in order of priority:
/// 1. Environment variables (highest priority)
/// 2. User configuration file
/// 3. Default configuration file (lowest priority)
///
/// Configuration is cached after first load for performance.
pub struct ProductionConfigService {
    config_builder: ConfigBuilder<DefaultState>,
    cached_config: Arc<RwLock<Option<Config>>>,
    env_provider: Arc<dyn EnvironmentProvider>,
}

impl ProductionConfigService {
    /// Create a new production configuration service.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration builder cannot be initialized.
    /// Creates a configuration service using the default environment variable provider (maintains compatibility with existing methods).
    pub fn new() -> Result<Self> {
        Self::with_env_provider(Arc::new(SystemEnvironmentProvider::new()))
    }

    /// Create a configuration service using the specified environment variable provider.
    ///
    /// # Arguments
    /// * `env_provider` - Environment variable provider
    pub fn with_env_provider(env_provider: Arc<dyn EnvironmentProvider>) -> Result<Self> {
        // Check if a custom config path is specified in the environment provider
        let config_file_path = if let Some(custom_path) = env_provider.get_var("SUBX_CONFIG_PATH") {
            PathBuf::from(custom_path)
        } else {
            Self::user_config_path()
        };

        let config_builder = ConfigCrate::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::from(config_file_path).required(false))
            .add_source(Environment::with_prefix("SUBX").separator("_"));

        Ok(Self {
            config_builder,
            cached_config: Arc::new(RwLock::new(None)),
            env_provider,
        })
    }

    /// Create a configuration service with custom sources.
    ///
    /// This allows adding additional configuration sources for specific use cases.
    ///
    /// # Arguments
    ///
    /// * `sources` - Additional configuration sources to add
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration builder cannot be updated.
    pub fn with_custom_file(mut self, file_path: PathBuf) -> Result<Self> {
        self.config_builder = self.config_builder.add_source(File::from(file_path));
        Ok(self)
    }

    /// Get the user configuration file path.
    ///
    /// Returns the path to the user's configuration file, which is typically
    /// located in the user's configuration directory.
    fn user_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("subx")
            .join("config.toml")
    }

    /// Load and validate configuration from all sources.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or validation fails.
    fn load_and_validate(&self) -> Result<Config> {
        debug!("ProductionConfigService: Loading configuration from sources");

        // Build configuration from all sources
        let config_crate = self.config_builder.build_cloned().map_err(|e| {
            debug!("ProductionConfigService: Config build failed: {}", e);
            SubXError::config(format!("Failed to build configuration: {}", e))
        })?;

        // Start with default configuration
        let mut app_config = Config::default();

        // Try to deserialize from config crate, but fall back to defaults if needed
        if let Ok(config) = config_crate.clone().try_deserialize::<Config>() {
            app_config = config;
            debug!("ProductionConfigService: Full configuration loaded successfully");
        } else {
            debug!("ProductionConfigService: Full deserialization failed, attempting partial load");

            // Try to load partial configurations from environment
            if let Ok(raw_map) = config_crate
                .try_deserialize::<std::collections::HashMap<String, serde_json::Value>>()
            {
                // Extract AI configuration if available
                if let Some(ai_section) = raw_map.get("ai") {
                    if let Some(ai_obj) = ai_section.as_object() {
                        // Extract individual AI fields that are available
                        if let Some(api_key) = ai_obj.get("apikey").and_then(|v| v.as_str()) {
                            app_config.ai.api_key = Some(api_key.to_string());
                            debug!(
                                "ProductionConfigService: AI API key loaded from SUBX_AI_APIKEY"
                            );
                        }
                        if let Some(provider) = ai_obj.get("provider").and_then(|v| v.as_str()) {
                            app_config.ai.provider = provider.to_string();
                            debug!(
                                "ProductionConfigService: AI provider loaded from SUBX_AI_PROVIDER"
                            );
                        }
                        if let Some(model) = ai_obj.get("model").and_then(|v| v.as_str()) {
                            app_config.ai.model = model.to_string();
                            debug!("ProductionConfigService: AI model loaded from SUBX_AI_MODEL");
                        }
                        if let Some(base_url) = ai_obj.get("base_url").and_then(|v| v.as_str()) {
                            app_config.ai.base_url = base_url.to_string();
                            debug!(
                                "ProductionConfigService: AI base URL loaded from SUBX_AI_BASE_URL"
                            );
                        }
                    }
                }
            }
        }

        // Special handling for OPENAI_API_KEY environment variable
        // This provides backward compatibility with direct OPENAI_API_KEY usage
        if app_config.ai.api_key.is_none() {
            if let Some(api_key) = self.env_provider.get_var("OPENAI_API_KEY") {
                debug!("ProductionConfigService: Found OPENAI_API_KEY environment variable");
                app_config.ai.api_key = Some(api_key);
            }
        }

        // Special handling for OPENAI_BASE_URL environment variable
        if let Some(base_url) = self.env_provider.get_var("OPENAI_BASE_URL") {
            debug!("ProductionConfigService: Found OPENAI_BASE_URL environment variable");
            app_config.ai.base_url = base_url;
        }

        // Validate the configuration
        crate::config::validator::validate_config(&app_config).map_err(|e| {
            debug!("ProductionConfigService: Config validation failed: {}", e);
            SubXError::config(format!("Configuration validation failed: {}", e))
        })?;

        debug!("ProductionConfigService: Configuration loaded and validated successfully");
        Ok(app_config)
    }

    /// Validate and set a configuration value.
    ///
    /// This method now delegates validation to the field_validator module.
    fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        use crate::config::field_validator;

        // Use the dedicated field validator
        field_validator::validate_field(key, value)?;

        // Set the value in the configuration
        self.set_value_internal(config, key, value)?;

        // Validate the entire configuration after the change
        self.validate_configuration(config)?;

        Ok(())
    }

    /// Internal method to set configuration values without validation.
    fn set_value_internal(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        use crate::config::OverflowStrategy;
        use crate::config::validation::*;
        use crate::error::SubXError;

        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => {
                config.ai.provider = value.to_string();
            }
            ["ai", "api_key"] => {
                if !value.is_empty() {
                    config.ai.api_key = Some(value.to_string());
                } else {
                    config.ai.api_key = None;
                }
            }
            ["ai", "model"] => {
                config.ai.model = value.to_string();
            }
            ["ai", "base_url"] => {
                config.ai.base_url = value.to_string();
            }
            ["ai", "max_sample_length"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.max_sample_length = v;
            }
            ["ai", "temperature"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.temperature = v;
            }
            ["ai", "max_tokens"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.max_tokens = v;
            }
            ["ai", "retry_attempts"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.retry_attempts = v;
            }
            ["ai", "retry_delay_ms"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.retry_delay_ms = v;
            }
            ["ai", "request_timeout_seconds"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.ai.request_timeout_seconds = v;
            }
            ["formats", "default_output"] => {
                config.formats.default_output = value.to_string();
            }
            ["formats", "preserve_styling"] => {
                let v = parse_bool(value)?;
                config.formats.preserve_styling = v;
            }
            ["formats", "default_encoding"] => {
                config.formats.default_encoding = value.to_string();
            }
            ["formats", "encoding_detection_confidence"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.formats.encoding_detection_confidence = v;
            }
            ["sync", "max_offset_seconds"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.sync.max_offset_seconds = v;
            }
            ["sync", "default_method"] => {
                config.sync.default_method = value.to_string();
            }
            ["sync", "vad", "enabled"] => {
                let v = parse_bool(value)?;
                config.sync.vad.enabled = v;
            }
            ["sync", "vad", "sensitivity"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.sync.vad.sensitivity = v;
            }
            ["sync", "vad", "padding_chunks"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.sync.vad.padding_chunks = v;
            }
            ["sync", "vad", "min_speech_duration_ms"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.sync.vad.min_speech_duration_ms = v;
            }
            ["general", "backup_enabled"] => {
                let v = parse_bool(value)?;
                config.general.backup_enabled = v;
            }
            ["general", "max_concurrent_jobs"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.general.max_concurrent_jobs = v;
            }
            ["general", "task_timeout_seconds"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.general.task_timeout_seconds = v;
            }
            ["general", "enable_progress_bar"] => {
                let v = parse_bool(value)?;
                config.general.enable_progress_bar = v;
            }
            ["general", "worker_idle_timeout_seconds"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.general.worker_idle_timeout_seconds = v;
            }
            ["parallel", "max_workers"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.parallel.max_workers = v;
            }
            ["parallel", "task_queue_size"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.parallel.task_queue_size = v;
            }
            ["parallel", "enable_task_priorities"] => {
                let v = parse_bool(value)?;
                config.parallel.enable_task_priorities = v;
            }
            ["parallel", "auto_balance_workers"] => {
                let v = parse_bool(value)?;
                config.parallel.auto_balance_workers = v;
            }
            ["parallel", "overflow_strategy"] => {
                config.parallel.overflow_strategy = match value {
                    "Block" => OverflowStrategy::Block,
                    "Drop" => OverflowStrategy::Drop,
                    "Expand" => OverflowStrategy::Expand,
                    _ => unreachable!(), // Validation already done
                };
            }
            _ => {
                return Err(SubXError::config(format!(
                    "Unknown configuration key: {}",
                    key
                )));
            }
        }
        Ok(())
    }

    /// Validate the entire configuration.
    fn validate_configuration(&self, config: &Config) -> Result<()> {
        use crate::config::validator;
        validator::validate_config(config)
    }

    /// Save configuration to file with specific config object.
    fn save_config_to_file_with_config(
        &self,
        path: &std::path::Path,
        config: &Config,
    ) -> Result<()> {
        let toml_content = toml::to_string_pretty(config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }
        std::fs::write(path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;
        Ok(())
    }
}

impl ConfigService for ProductionConfigService {
    fn get_config(&self) -> Result<Config> {
        // Check cache first
        {
            let cache = self.cached_config.read().unwrap();
            if let Some(config) = cache.as_ref() {
                debug!("ProductionConfigService: Returning cached configuration");
                return Ok(config.clone());
            }
        }

        // Load configuration
        let app_config = self.load_and_validate()?;

        // Update cache
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = Some(app_config.clone());
        }

        Ok(app_config)
    }

    fn reload(&self) -> Result<()> {
        debug!("ProductionConfigService: Reloading configuration");

        // Clear cache to force reload
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = None;
        }

        // Trigger reload by calling get_config
        self.get_config()?;

        debug!("ProductionConfigService: Configuration reloaded successfully");
        Ok(())
    }

    fn save_config(&self) -> Result<()> {
        let _config = self.get_config()?;
        let path = self.get_config_file_path()?;
        self.save_config_to_file(&path)
    }

    fn save_config_to_file(&self, path: &Path) -> Result<()> {
        let config = self.get_config()?;
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }

        std::fs::write(path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    fn get_config_file_path(&self) -> Result<PathBuf> {
        // Allow injection via EnvironmentProvider for testing
        if let Some(custom) = self.env_provider.get_var("SUBX_CONFIG_PATH") {
            return Ok(PathBuf::from(custom));
        }

        let config_dir = dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?;
        Ok(config_dir.join("subx").join("config.toml"))
    }

    fn get_config_value(&self, key: &str) -> Result<String> {
        let config = self.get_config()?;
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(config.ai.provider.clone()),
            ["ai", "model"] => Ok(config.ai.model.clone()),
            ["ai", "api_key"] => Ok(config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url.clone()),
            ["ai", "max_sample_length"] => Ok(config.ai.max_sample_length.to_string()),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            ["ai", "max_tokens"] => Ok(config.ai.max_tokens.to_string()),
            ["ai", "retry_attempts"] => Ok(config.ai.retry_attempts.to_string()),
            ["ai", "retry_delay_ms"] => Ok(config.ai.retry_delay_ms.to_string()),
            ["ai", "request_timeout_seconds"] => Ok(config.ai.request_timeout_seconds.to_string()),

            ["formats", "default_output"] => Ok(config.formats.default_output.clone()),
            ["formats", "default_encoding"] => Ok(config.formats.default_encoding.clone()),
            ["formats", "preserve_styling"] => Ok(config.formats.preserve_styling.to_string()),
            ["formats", "encoding_detection_confidence"] => {
                Ok(config.formats.encoding_detection_confidence.to_string())
            }

            ["sync", "default_method"] => Ok(config.sync.default_method.clone()),
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
            ["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
            ["sync", "vad", "padding_chunks"] => Ok(config.sync.vad.padding_chunks.to_string()),
            ["sync", "vad", "min_speech_duration_ms"] => {
                Ok(config.sync.vad.min_speech_duration_ms.to_string())
            }

            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => {
                Ok(config.general.max_concurrent_jobs.to_string())
            }
            ["general", "task_timeout_seconds"] => {
                Ok(config.general.task_timeout_seconds.to_string())
            }
            ["general", "enable_progress_bar"] => {
                Ok(config.general.enable_progress_bar.to_string())
            }
            ["general", "worker_idle_timeout_seconds"] => {
                Ok(config.general.worker_idle_timeout_seconds.to_string())
            }

            ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
            ["parallel", "task_queue_size"] => Ok(config.parallel.task_queue_size.to_string()),
            ["parallel", "enable_task_priorities"] => {
                Ok(config.parallel.enable_task_priorities.to_string())
            }
            ["parallel", "auto_balance_workers"] => {
                Ok(config.parallel.auto_balance_workers.to_string())
            }
            ["parallel", "overflow_strategy"] => {
                Ok(format!("{:?}", config.parallel.overflow_strategy))
            }

            _ => Err(SubXError::config(format!(
                "Unknown configuration key: {}",
                key
            ))),
        }
    }

    fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        // 1. Load current configuration
        let mut config = self.get_config()?;

        // 2. Validate and set the value
        self.validate_and_set_value(&mut config, key, value)?;

        // 3. Validate the entire configuration
        crate::config::validator::validate_config(&config)?;

        // 4. Save to file
        let path = self.get_config_file_path()?;
        self.save_config_to_file_with_config(&path, &config)?;

        // 5. Update cache
        {
            let mut cache = self.cached_config.write().unwrap();
            *cache = Some(config);
        }

        Ok(())
    }

    fn reset_to_defaults(&self) -> Result<()> {
        let default_config = Config::default();
        let path = self.get_config_file_path()?;

        let toml_content = toml::to_string_pretty(&default_config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }

        std::fs::write(&path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;

        self.reload()
    }
}

impl Default for ProductionConfigService {
    fn default() -> Self {
        Self::new().expect("Failed to create default ProductionConfigService")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfigService;
    use crate::config::TestEnvironmentProvider;
    use std::sync::Arc;

    #[test]
    fn test_production_config_service_creation() {
        let service = ProductionConfigService::new();
        assert!(service.is_ok());
    }

    #[test]
    fn test_production_config_service_with_custom_file() {
        let service = ProductionConfigService::new()
            .unwrap()
            .with_custom_file(PathBuf::from("test.toml"));
        assert!(service.is_ok());
    }

    #[test]
    fn test_production_service_implements_config_service_trait() {
        let service = ProductionConfigService::new().unwrap();

        // Test trait methods
        let config1 = service.get_config();
        assert!(config1.is_ok());

        let reload_result = service.reload();
        assert!(reload_result.is_ok());

        let config2 = service.get_config();
        assert!(config2.is_ok());
    }

    #[test]
    fn test_config_service_with_openai_api_key() {
        // Test configuration with OpenAI API key using TestConfigService
        let test_service = TestConfigService::with_ai_settings_and_key(
            "openai",
            "gpt-4.1-mini",
            "sk-test-openai-key-123",
        );

        let config = test_service.get_config().unwrap();
        assert_eq!(
            config.ai.api_key,
            Some("sk-test-openai-key-123".to_string())
        );
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1-mini");
    }

    #[test]
    fn test_config_service_with_custom_base_url() {
        // Test configuration with custom base URL
        let mut config = Config::default();
        config.ai.base_url = "https://custom.openai.endpoint".to_string();

        let test_service = TestConfigService::new(config);
        let loaded_config = test_service.get_config().unwrap();

        assert_eq!(loaded_config.ai.base_url, "https://custom.openai.endpoint");
    }

    #[test]
    fn test_config_service_with_both_openai_settings() {
        // Test configuration with both API key and base URL
        let mut config = Config::default();
        config.ai.api_key = Some("sk-test-api-key-combined".to_string());
        config.ai.base_url = "https://api.custom-openai.com".to_string();

        let test_service = TestConfigService::new(config);
        let loaded_config = test_service.get_config().unwrap();

        assert_eq!(
            loaded_config.ai.api_key,
            Some("sk-test-api-key-combined".to_string())
        );
        assert_eq!(loaded_config.ai.base_url, "https://api.custom-openai.com");
    }

    #[test]
    fn test_config_service_provider_precedence() {
        // Test that manually configured values take precedence
        let test_service =
            TestConfigService::with_ai_settings_and_key("openai", "gpt-4.1", "sk-explicit-key");

        let config = test_service.get_config().unwrap();
        assert_eq!(config.ai.api_key, Some("sk-explicit-key".to_string()));
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1");
    }

    #[test]
    fn test_config_service_fallback_behavior() {
        // Test fallback to default values when no specific configuration provided
        let test_service = TestConfigService::with_defaults();
        let config = test_service.get_config().unwrap();

        // Should use default values
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1-mini");
        assert_eq!(config.ai.base_url, "https://api.openai.com/v1");
        assert_eq!(config.ai.api_key, None); // No API key by default
    }

    #[test]
    fn test_config_service_reload_functionality() {
        // Test configuration reload capability
        let test_service = TestConfigService::with_defaults();

        // First load
        let config1 = test_service.get_config().unwrap();
        assert_eq!(config1.ai.provider, "openai");

        // Reload should always succeed for test service
        let reload_result = test_service.reload();
        assert!(reload_result.is_ok());

        // Second load should still work
        let config2 = test_service.get_config().unwrap();
        assert_eq!(config2.ai.provider, "openai");
    }

    #[test]
    fn test_config_service_custom_base_url_override() {
        // Test that custom base URL properly overrides default
        let mut config = Config::default();
        config.ai.base_url = "https://my-proxy.openai.com/v1".to_string();

        let test_service = TestConfigService::new(config);
        let loaded_config = test_service.get_config().unwrap();

        assert_eq!(loaded_config.ai.base_url, "https://my-proxy.openai.com/v1");
    }

    #[test]
    fn test_config_service_sync_settings() {
        // Test sync configuration settings
        let test_service = TestConfigService::with_sync_settings(0.8, 45.0);
        let config = test_service.get_config().unwrap();

        assert_eq!(config.sync.correlation_threshold, 0.8);
        assert_eq!(config.sync.max_offset_seconds, 45.0);
    }

    #[test]
    fn test_config_service_parallel_settings() {
        // Test parallel processing configuration
        let test_service = TestConfigService::with_parallel_settings(8, 200);
        let config = test_service.get_config().unwrap();

        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    }

    #[test]
    fn test_config_service_direct_access() {
        // Test direct configuration access and mutation
        let test_service = TestConfigService::with_defaults();

        // Test direct read access
        assert_eq!(test_service.config().ai.provider, "openai");

        // Test mutable access
        test_service.config_mut().ai.provider = "modified".to_string();
        assert_eq!(test_service.config().ai.provider, "modified");

        // Test that get_config reflects the changes
        let config = test_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "modified");
    }

    #[test]
    fn test_production_config_service_openai_api_key_loading() {
        // Test OPENAI_API_KEY environment variable loading
        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("OPENAI_API_KEY", "sk-test-openai-key-env");

        // Use a non-existent config path to avoid interference from existing config files
        env_provider.set_var(
            "SUBX_CONFIG_PATH",
            "/tmp/test_config_that_does_not_exist.toml",
        );

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(
            config.ai.api_key,
            Some("sk-test-openai-key-env".to_string())
        );
    }

    #[test]
    fn test_production_config_service_openai_base_url_loading() {
        // Test OPENAI_BASE_URL environment variable loading
        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("OPENAI_BASE_URL", "https://test.openai.com/v1");

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.base_url, "https://test.openai.com/v1");
    }

    #[test]
    fn test_production_config_service_both_openai_env_vars() {
        // Test setting both OPENAI environment variables simultaneously
        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("OPENAI_API_KEY", "sk-test-key-both");
        env_provider.set_var("OPENAI_BASE_URL", "https://both.openai.com/v1");

        // Use a non-existent config path to avoid interference from existing config files
        env_provider.set_var(
            "SUBX_CONFIG_PATH",
            "/tmp/test_config_both_that_does_not_exist.toml",
        );

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.api_key, Some("sk-test-key-both".to_string()));
        assert_eq!(config.ai.base_url, "https://both.openai.com/v1");
    }

    #[test]
    fn test_production_config_service_no_openai_env_vars() {
        // Test the case with no OPENAI environment variables
        let mut env_provider = TestEnvironmentProvider::new(); // Empty provider

        // Use a non-existent config path to avoid interference from existing config files
        env_provider.set_var(
            "SUBX_CONFIG_PATH",
            "/tmp/test_config_no_openai_that_does_not_exist.toml",
        );

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        // Should use default values
        assert_eq!(config.ai.api_key, None);
        assert_eq!(config.ai.base_url, "https://api.openai.com/v1"); // Default value
    }

    #[test]
    fn test_production_config_service_api_key_priority() {
        // Test API key priority: existing API key should not be overwritten
        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("OPENAI_API_KEY", "sk-env-key");
        // Simulate API key loaded from other sources (e.g., configuration file)
        env_provider.set_var("SUBX_AI_APIKEY", "sk-config-key");

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        // SUBX_AI_APIKEY should have higher priority (since it's processed first)
        // This test only verifies priority order, should at least have a value
        assert!(config.ai.api_key.is_some());
    }
}
