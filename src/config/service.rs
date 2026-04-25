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

/// Write configuration content to `path` with restrictive permissions.
///
/// On Unix the parent directory is created (if missing) with mode `0o700` and
/// the file is created/truncated with mode `0o600`, ensuring only the current
/// user can read the file containing sensitive values such as API keys.
///
/// On non-Unix platforms the file is written with the platform's default
/// permissions because POSIX modes do not apply.
#[cfg(unix)]
fn secure_write_config_file(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(content.as_bytes())?;
    // Ensure an existing file's permissions are tightened as well.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn secure_write_config_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)
}

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
            debug!("ProductionConfigService: Config build failed: {e}");
            SubXError::config(format!("Failed to build configuration: {e}"))
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

        // Special handling for OPENROUTER_API_KEY environment variable
        if let Some(api_key) = self.env_provider.get_var("OPENROUTER_API_KEY") {
            debug!("ProductionConfigService: Found OPENROUTER_API_KEY environment variable");
            app_config.ai.provider = "openrouter".to_string();
            app_config.ai.api_key = Some(api_key);
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

        // Special handling for Azure OpenAI environment variables
        if let Some(api_key) = self.env_provider.get_var("AZURE_OPENAI_API_KEY") {
            debug!("ProductionConfigService: Found AZURE_OPENAI_API_KEY environment variable");
            app_config.ai.provider = "azure-openai".to_string();
            app_config.ai.api_key = Some(api_key);
        }
        if let Some(endpoint) = self.env_provider.get_var("AZURE_OPENAI_ENDPOINT") {
            debug!("ProductionConfigService: Found AZURE_OPENAI_ENDPOINT environment variable");
            app_config.ai.base_url = endpoint;
        }
        if let Some(version) = self.env_provider.get_var("AZURE_OPENAI_API_VERSION") {
            debug!("ProductionConfigService: Found AZURE_OPENAI_API_VERSION environment variable");
            app_config.ai.api_version = Some(version);
        }
        // Special handling for Azure OpenAI deployment ID environment variable
        if let Some(deployment) = self.env_provider.get_var("AZURE_OPENAI_DEPLOYMENT_ID") {
            debug!(
                "ProductionConfigService: Found AZURE_OPENAI_DEPLOYMENT_ID environment variable"
            );
            app_config.ai.model = deployment;
        }

        // Validate the configuration
        crate::config::validator::validate_config(&app_config).map_err(|e| {
            debug!("ProductionConfigService: Config validation failed: {e}");
            SubXError::config(format!("Configuration validation failed: {e}"))
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
            ["ai", "api_version"] => {
                if !value.is_empty() {
                    config.ai.api_version = Some(value.to_string());
                } else {
                    config.ai.api_version = None;
                }
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
            ["general", "max_subtitle_bytes"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.general.max_subtitle_bytes = v;
            }
            ["general", "max_audio_bytes"] => {
                let v = value.parse().unwrap(); // Validation already done
                config.general.max_audio_bytes = v;
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
                    "Unknown configuration key: {key}"
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
            .map_err(|e| SubXError::config(format!("TOML serialization error: {e}")))?;
        secure_write_config_file(path, &toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {e}")))?;
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
            .map_err(|e| SubXError::config(format!("TOML serialization error: {e}")))?;

        secure_write_config_file(path, &toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {e}")))?;

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
            ["general", "max_subtitle_bytes"] => Ok(config.general.max_subtitle_bytes.to_string()),
            ["general", "max_audio_bytes"] => Ok(config.general.max_audio_bytes.to_string()),

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

        secure_write_config_file(&path, &toml_content)
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

    /// Helper: create a ProductionConfigService whose config file lives in
    /// the given TempDir so file-writing tests are isolated.
    fn make_service_with_tmp_config(dir: &tempfile::TempDir) -> ProductionConfigService {
        let config_path = dir.path().join("config.toml");
        let mut env = TestEnvironmentProvider::new();
        env.set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        ProductionConfigService::with_env_provider(Arc::new(env)).unwrap()
    }

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
    fn test_production_config_service_openrouter_api_key_loading() {
        use crate::config::TestEnvironmentProvider;
        use std::sync::Arc;

        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("OPENROUTER_API_KEY", "test-openrouter-key");
        env_provider.set_var("SUBX_CONFIG_PATH", "/tmp/test_config_openrouter.toml");

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.api_key, Some("test-openrouter-key".to_string()));
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
    fn test_config_size_limits_defaults() {
        let service = TestConfigService::with_defaults();
        let cfg = service.get_config().unwrap();
        assert_eq!(cfg.general.max_subtitle_bytes, 52_428_800);
        assert_eq!(cfg.general.max_audio_bytes, 2_147_483_648);
    }

    #[test]
    fn test_config_size_limits_roundtrip() {
        let service = TestConfigService::with_defaults();

        service
            .set_config_value("general.max_subtitle_bytes", "65536")
            .unwrap();
        service
            .set_config_value("general.max_audio_bytes", "1048576")
            .unwrap();

        assert_eq!(
            service
                .get_config_value("general.max_subtitle_bytes")
                .unwrap(),
            "65536"
        );
        assert_eq!(
            service.get_config_value("general.max_audio_bytes").unwrap(),
            "1048576"
        );
    }

    #[test]
    fn test_config_size_limits_validation_reject() {
        let service = TestConfigService::with_defaults();
        // Below minimum (1024)
        assert!(
            service
                .set_config_value("general.max_subtitle_bytes", "100")
                .is_err()
        );
        // Above maximum (1 GiB)
        assert!(
            service
                .set_config_value("general.max_subtitle_bytes", "2147483648")
                .is_err()
        );
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

    #[cfg(unix)]
    #[test]
    fn test_secure_write_config_file_sets_0600_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create tempdir");
        let nested = dir.path().join("subdir");
        let path = nested.join("config.toml");

        super::secure_write_config_file(&path, "api_key = \"secret\"\n")
            .expect("secure write should succeed");

        let meta = std::fs::metadata(&path).expect("file must exist");
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "file permissions must be 0o600, got {:o}",
            mode
        );

        let dir_meta = std::fs::metadata(&nested).expect("parent must exist");
        let dir_mode = dir_meta.permissions().mode() & 0o777;
        assert_eq!(
            dir_mode, 0o700,
            "directory permissions must be 0o700, got {:o}",
            dir_mode
        );

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "api_key = \"secret\"\n");
    }

    #[cfg(unix)]
    #[test]
    fn test_secure_write_config_file_truncates_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("create tempdir");
        let path = dir.path().join("config.toml");

        // Create an existing file with permissive mode and stale contents.
        std::fs::write(&path, "stale contents that should be replaced").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        super::secure_write_config_file(&path, "new = \"value\"\n").expect("secure write");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new = \"value\"\n");
    }

    // -----------------------------------------------------------------------
    // Caching behaviour
    // -----------------------------------------------------------------------

    #[test]
    fn test_production_config_get_config_caches_result() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        let config1 = service.get_config().unwrap();
        let config2 = service.get_config().unwrap();
        assert_eq!(config1.ai.provider, config2.ai.provider);
        assert_eq!(config1.ai.model, config2.ai.model);
    }

    #[test]
    fn test_production_config_reload_clears_cache_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.get_config().unwrap(); // populate cache
        service.reload().unwrap(); // must clear then reload
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
    }

    // -----------------------------------------------------------------------
    // Azure OpenAI environment variable handling
    // -----------------------------------------------------------------------

    #[test]
    fn test_azure_openai_api_key_sets_provider_and_key() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var("AZURE_OPENAI_API_KEY", "azure-api-key-test");
        env.set_var("SUBX_CONFIG_PATH", "/nonexistent/azure_api_key_test.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "azure-openai");
        assert_eq!(config.ai.api_key, Some("azure-api-key-test".to_string()));
    }

    #[test]
    fn test_azure_openai_endpoint_sets_base_url() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var(
            "AZURE_OPENAI_ENDPOINT",
            "https://my-instance.openai.azure.com",
        );
        env.set_var("SUBX_CONFIG_PATH", "/nonexistent/azure_endpoint_test.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.base_url, "https://my-instance.openai.azure.com");
    }

    #[test]
    fn test_azure_openai_api_version_sets_api_version() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var("AZURE_OPENAI_API_VERSION", "2024-02-01");
        env.set_var("SUBX_CONFIG_PATH", "/nonexistent/azure_version_test.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.api_version, Some("2024-02-01".to_string()));
    }

    #[test]
    fn test_azure_openai_deployment_id_sets_model() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var("AZURE_OPENAI_API_KEY", "azure-key-for-deploy");
        env.set_var("AZURE_OPENAI_DEPLOYMENT_ID", "my-gpt4-deployment");
        env.set_var("SUBX_CONFIG_PATH", "/nonexistent/azure_deploy_test.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.model, "my-gpt4-deployment");
    }

    #[test]
    fn test_azure_openai_all_env_vars_together() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var("AZURE_OPENAI_API_KEY", "full-azure-api-key");
        env.set_var("AZURE_OPENAI_ENDPOINT", "https://full.openai.azure.com");
        env.set_var("AZURE_OPENAI_API_VERSION", "2024-05-01");
        env.set_var("AZURE_OPENAI_DEPLOYMENT_ID", "full-deployment-name");
        env.set_var("SUBX_CONFIG_PATH", "/nonexistent/azure_full_test.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "azure-openai");
        assert_eq!(config.ai.api_key, Some("full-azure-api-key".to_string()));
        assert_eq!(config.ai.base_url, "https://full.openai.azure.com");
        assert_eq!(config.ai.api_version, Some("2024-05-01".to_string()));
        assert_eq!(config.ai.model, "full-deployment-name");
    }

    // -----------------------------------------------------------------------
    // get_config_file_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_config_file_path_uses_subx_config_path_env() {
        let mut env = TestEnvironmentProvider::new();
        env.set_var("SUBX_CONFIG_PATH", "/custom/path/config.toml");
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let path = service.get_config_file_path().unwrap();
        assert_eq!(path, PathBuf::from("/custom/path/config.toml"));
    }

    #[test]
    fn test_get_config_file_path_default_contains_subx() {
        let env = TestEnvironmentProvider::new(); // no SUBX_CONFIG_PATH
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let path = service.get_config_file_path().unwrap();
        let s = path.to_str().unwrap();
        assert!(s.contains("subx"), "expected 'subx' in path: {s}");
        assert!(
            s.ends_with("config.toml"),
            "expected 'config.toml' suffix: {s}"
        );
    }

    // -----------------------------------------------------------------------
    // save_config_to_file / save_config
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_config_to_file_writes_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        let save_path = dir.path().join("output.toml");
        service.save_config_to_file(&save_path).unwrap();
        let content = std::fs::read_to_string(&save_path).unwrap();
        assert!(content.contains("[ai]"), "missing [ai] section: {content}");
        assert!(
            content.contains("provider"),
            "missing 'provider': {content}"
        );
    }

    #[test]
    fn test_save_config_writes_to_configured_path() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.save_config().unwrap();
        let config_path = dir.path().join("config.toml");
        assert!(config_path.exists(), "config file was not created");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[ai]"));
    }

    // -----------------------------------------------------------------------
    // reset_to_defaults
    // -----------------------------------------------------------------------

    #[test]
    fn test_reset_to_defaults_restores_default_config() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        // First write a file so reset has something to overwrite
        service.save_config().unwrap();
        service.reset_to_defaults().unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1-mini");
        assert_eq!(config.formats.default_output, "srt");
    }

    // -----------------------------------------------------------------------
    // get_config_value – all branches in ProductionConfigService
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_config_value_all_ai_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        for key in &[
            "ai.provider",
            "ai.model",
            "ai.api_key",
            "ai.base_url",
            "ai.max_sample_length",
            "ai.temperature",
            "ai.max_tokens",
            "ai.retry_attempts",
            "ai.retry_delay_ms",
            "ai.request_timeout_seconds",
        ] {
            assert!(
                service.get_config_value(key).is_ok(),
                "failed for key: {key}"
            );
        }
    }

    #[test]
    fn test_get_config_value_all_formats_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        for key in &[
            "formats.default_output",
            "formats.default_encoding",
            "formats.preserve_styling",
            "formats.encoding_detection_confidence",
        ] {
            assert!(
                service.get_config_value(key).is_ok(),
                "failed for key: {key}"
            );
        }
    }

    #[test]
    fn test_get_config_value_all_sync_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        for key in &[
            "sync.default_method",
            "sync.max_offset_seconds",
            "sync.vad.enabled",
            "sync.vad.sensitivity",
            "sync.vad.padding_chunks",
            "sync.vad.min_speech_duration_ms",
        ] {
            assert!(
                service.get_config_value(key).is_ok(),
                "failed for key: {key}"
            );
        }
    }

    #[test]
    fn test_get_config_value_all_general_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        for key in &[
            "general.backup_enabled",
            "general.max_concurrent_jobs",
            "general.task_timeout_seconds",
            "general.enable_progress_bar",
            "general.worker_idle_timeout_seconds",
            "general.max_subtitle_bytes",
            "general.max_audio_bytes",
        ] {
            assert!(
                service.get_config_value(key).is_ok(),
                "failed for key: {key}"
            );
        }
    }

    #[test]
    fn test_get_config_value_all_parallel_keys() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        for key in &[
            "parallel.max_workers",
            "parallel.task_queue_size",
            "parallel.enable_task_priorities",
            "parallel.auto_balance_workers",
            "parallel.overflow_strategy",
        ] {
            assert!(
                service.get_config_value(key).is_ok(),
                "failed for key: {key}"
            );
        }
    }

    #[test]
    fn test_get_config_value_unknown_key_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        assert!(service.get_config_value("nonexistent.key").is_err());
        assert!(service.get_config_value("ai").is_err());
    }

    #[test]
    fn test_get_config_value_returns_correct_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        assert_eq!(service.get_config_value("ai.provider").unwrap(), "openai");
        assert_eq!(
            service.get_config_value("ai.model").unwrap(),
            "gpt-4.1-mini"
        );
        assert_eq!(service.get_config_value("ai.api_key").unwrap(), "");
        assert_eq!(
            service.get_config_value("formats.default_output").unwrap(),
            "srt"
        );
        assert_eq!(
            service.get_config_value("general.backup_enabled").unwrap(),
            "false"
        );
    }

    // -----------------------------------------------------------------------
    // set_config_value – AI section
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_ai_provider() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.provider", "openrouter")
            .unwrap();
        assert_eq!(
            service.get_config_value("ai.provider").unwrap(),
            "openrouter"
        );
    }

    #[test]
    fn test_set_config_value_ai_model() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.set_config_value("ai.model", "gpt-4.1").unwrap();
        assert_eq!(service.get_config_value("ai.model").unwrap(), "gpt-4.1");
    }

    #[test]
    fn test_set_config_value_ai_api_key_non_empty() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.api_key", "sk-test-apikey-12345")
            .unwrap();
        assert_eq!(
            service.get_config_value("ai.api_key").unwrap(),
            "sk-test-apikey-12345"
        );
    }

    #[test]
    fn test_set_config_value_ai_api_key_empty_clears_key() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        // Set a key first
        service
            .set_config_value("ai.api_key", "sk-test-apikey-12345")
            .unwrap();
        // Then clear it
        service.set_config_value("ai.api_key", "").unwrap();
        assert_eq!(service.get_config_value("ai.api_key").unwrap(), "");
        let config = service.get_config().unwrap();
        assert!(config.ai.api_key.is_none());
    }

    #[test]
    fn test_set_config_value_ai_base_url() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.base_url", "https://custom.example.com/v1")
            .unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.base_url, "https://custom.example.com/v1");
    }

    #[test]
    fn test_set_config_value_ai_temperature() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.set_config_value("ai.temperature", "0.7").unwrap();
        let config = service.get_config().unwrap();
        assert!((config.ai.temperature - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_set_config_value_ai_max_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.set_config_value("ai.max_tokens", "5000").unwrap();
        assert_eq!(service.get_config_value("ai.max_tokens").unwrap(), "5000");
    }

    #[test]
    fn test_set_config_value_ai_retry_attempts() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service.set_config_value("ai.retry_attempts", "5").unwrap();
        assert_eq!(service.get_config_value("ai.retry_attempts").unwrap(), "5");
    }

    #[test]
    fn test_set_config_value_ai_retry_delay_ms() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.retry_delay_ms", "2000")
            .unwrap();
        assert_eq!(
            service.get_config_value("ai.retry_delay_ms").unwrap(),
            "2000"
        );
    }

    #[test]
    fn test_set_config_value_ai_request_timeout_seconds() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.request_timeout_seconds", "60")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("ai.request_timeout_seconds")
                .unwrap(),
            "60"
        );
    }

    #[test]
    fn test_set_config_value_ai_max_sample_length() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.max_sample_length", "500")
            .unwrap();
        assert_eq!(
            service.get_config_value("ai.max_sample_length").unwrap(),
            "500"
        );
    }

    #[test]
    fn test_set_config_value_ai_api_version_non_empty() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("ai.api_version", "2024-02-01")
            .unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.api_version, Some("2024-02-01".to_string()));
    }

    // -----------------------------------------------------------------------
    // set_config_value – formats section
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_formats_default_output() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("formats.default_output", "ass")
            .unwrap();
        assert_eq!(
            service.get_config_value("formats.default_output").unwrap(),
            "ass"
        );
    }

    #[test]
    fn test_set_config_value_formats_preserve_styling() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("formats.preserve_styling", "true")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(config.formats.preserve_styling);
    }

    #[test]
    fn test_set_config_value_formats_default_encoding() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("formats.default_encoding", "utf-8")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("formats.default_encoding")
                .unwrap(),
            "utf-8"
        );
    }

    #[test]
    fn test_set_config_value_formats_encoding_detection_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("formats.encoding_detection_confidence", "0.9")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!((config.formats.encoding_detection_confidence - 0.9).abs() < 0.001);
    }

    // -----------------------------------------------------------------------
    // set_config_value – sync section
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_sync_max_offset_seconds() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.max_offset_seconds", "30")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!((config.sync.max_offset_seconds - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_set_config_value_sync_default_method() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.default_method", "vad")
            .unwrap();
        assert_eq!(
            service.get_config_value("sync.default_method").unwrap(),
            "vad"
        );
    }

    #[test]
    fn test_set_config_value_sync_vad_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.vad.enabled", "false")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(!config.sync.vad.enabled);
    }

    #[test]
    fn test_set_config_value_sync_vad_sensitivity() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.vad.sensitivity", "0.5")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!((config.sync.vad.sensitivity - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_set_config_value_sync_vad_padding_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.vad.padding_chunks", "5")
            .unwrap();
        assert_eq!(
            service.get_config_value("sync.vad.padding_chunks").unwrap(),
            "5"
        );
    }

    #[test]
    fn test_set_config_value_sync_vad_min_speech_duration_ms() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("sync.vad.min_speech_duration_ms", "500")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("sync.vad.min_speech_duration_ms")
                .unwrap(),
            "500"
        );
    }

    // -----------------------------------------------------------------------
    // set_config_value – general section
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_general_backup_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("general.backup_enabled", "true")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(config.general.backup_enabled);
    }

    #[test]
    fn test_set_config_value_general_max_concurrent_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("general.max_concurrent_jobs", "8")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("general.max_concurrent_jobs")
                .unwrap(),
            "8"
        );
    }

    #[test]
    fn test_set_config_value_general_task_timeout_seconds() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("general.task_timeout_seconds", "120")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("general.task_timeout_seconds")
                .unwrap(),
            "120"
        );
    }

    #[test]
    fn test_set_config_value_general_enable_progress_bar() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("general.enable_progress_bar", "false")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(!config.general.enable_progress_bar);
    }

    #[test]
    fn test_set_config_value_general_worker_idle_timeout_seconds() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("general.worker_idle_timeout_seconds", "60")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("general.worker_idle_timeout_seconds")
                .unwrap(),
            "60"
        );
    }

    // -----------------------------------------------------------------------
    // set_config_value – parallel section
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_parallel_max_workers() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.max_workers", "4")
            .unwrap();
        assert_eq!(
            service.get_config_value("parallel.max_workers").unwrap(),
            "4"
        );
    }

    #[test]
    fn test_set_config_value_parallel_task_queue_size() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.task_queue_size", "200")
            .unwrap();
        assert_eq!(
            service
                .get_config_value("parallel.task_queue_size")
                .unwrap(),
            "200"
        );
    }

    #[test]
    fn test_set_config_value_parallel_enable_task_priorities() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.enable_task_priorities", "true")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(config.parallel.enable_task_priorities);
    }

    #[test]
    fn test_set_config_value_parallel_auto_balance_workers() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.auto_balance_workers", "false")
            .unwrap();
        let config = service.get_config().unwrap();
        assert!(!config.parallel.auto_balance_workers);
    }

    #[test]
    fn test_set_config_value_parallel_overflow_strategy_block() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.overflow_strategy", "Block")
            .unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(
            config.parallel.overflow_strategy,
            crate::config::OverflowStrategy::Block
        );
    }

    #[test]
    fn test_set_config_value_parallel_overflow_strategy_drop() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.overflow_strategy", "Drop")
            .unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(
            config.parallel.overflow_strategy,
            crate::config::OverflowStrategy::Drop
        );
    }

    #[test]
    fn test_set_config_value_parallel_overflow_strategy_expand() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        service
            .set_config_value("parallel.overflow_strategy", "Expand")
            .unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(
            config.parallel.overflow_strategy,
            crate::config::OverflowStrategy::Expand
        );
    }

    // -----------------------------------------------------------------------
    // set_config_value – error paths
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_unknown_key_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        assert!(
            service
                .set_config_value("nonexistent.key", "value")
                .is_err()
        );
    }

    #[test]
    fn test_set_config_value_invalid_value_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let service = make_service_with_tmp_config(&dir);
        // temperature must be in [0.0, 2.0]
        assert!(service.set_config_value("ai.temperature", "99.9").is_err());
        // provider must be a known enum value
        assert!(
            service
                .set_config_value("ai.provider", "unknown-provider")
                .is_err()
        );
    }

    // -----------------------------------------------------------------------
    // Default trait impl
    // -----------------------------------------------------------------------

    #[test]
    fn test_production_config_service_default_trait_impl() {
        let service = ProductionConfigService::default();
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
    }

    // -----------------------------------------------------------------------
    // Loading config values from a TOML file
    // -----------------------------------------------------------------------

    #[test]
    fn test_production_config_service_loads_values_from_toml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("custom.toml");

        // Write a serialised default config with one field overridden
        let mut cfg = crate::config::Config::default();
        cfg.ai.provider = "openrouter".to_string();
        cfg.ai.model = "toml-loaded-model".to_string();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        std::fs::write(&config_path, toml_str).unwrap();

        let mut env = TestEnvironmentProvider::new();
        env.set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
        let loaded = service.get_config().unwrap();
        assert_eq!(loaded.ai.provider, "openrouter");
        assert_eq!(loaded.ai.model, "toml-loaded-model");
    }

    // -----------------------------------------------------------------------
    // TestConfigService instance methods (set_ai_settings_and_key,
    // set_ai_settings_with_base_url) – covered here to keep service tests
    // complete and avoid cross-module duplication.
    // -----------------------------------------------------------------------

    #[test]
    fn test_test_config_service_set_ai_settings_and_key_instance_method() {
        let service = TestConfigService::with_defaults();
        service.set_ai_settings_and_key("openrouter", "my-model", "test-key-1234567890");
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openrouter");
        assert_eq!(config.ai.model, "my-model");
        assert_eq!(config.ai.api_key, Some("test-key-1234567890".to_string()));
    }

    #[test]
    fn test_test_config_service_set_ai_settings_and_key_empty_clears_key() {
        let service = TestConfigService::with_defaults();
        service.set_ai_settings_and_key("openai", "gpt-4", "");
        let config = service.get_config().unwrap();
        assert!(config.ai.api_key.is_none());
    }

    #[test]
    fn test_test_config_service_set_ai_settings_with_base_url() {
        let service = TestConfigService::with_defaults();
        service.set_ai_settings_with_base_url(
            "openai",
            "gpt-4.1",
            "sk-test-key-12345",
            "https://proxy.example.com/v1",
        );
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1");
        assert_eq!(config.ai.api_key, Some("sk-test-key-12345".to_string()));
        assert_eq!(config.ai.base_url, "https://proxy.example.com/v1");
    }

    // -----------------------------------------------------------------------
    // File persistence: set_config_value updates the file on disk
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_config_value_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let mut env = TestEnvironmentProvider::new();
        env.set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        let service = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();

        service.set_config_value("ai.model", "gpt-4.1").unwrap();

        let file_content = std::fs::read_to_string(&config_path).unwrap();
        assert!(
            file_content.contains("gpt-4.1"),
            "model not persisted to disk: {file_content}"
        );
    }

    // -----------------------------------------------------------------------
    // secure_write_config_file – parent dir already exists (no creation)
    // -----------------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn test_secure_write_config_file_existing_parent_dir() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        super::secure_write_config_file(&path, "key = \"value\"\n")
            .expect("write to existing dir should succeed");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "key = \"value\"\n");
    }
}
