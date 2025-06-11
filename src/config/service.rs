//! Configuration service system for dependency injection and test isolation.
//!
//! This module provides a clean abstraction for configuration management
//! that enables dependency injection and complete test isolation without
//! requiring unsafe code or global state resets.

use crate::{Result, config::Config, error::SubXError};
use config::{Config as ConfigCrate, ConfigBuilder, Environment, File, builder::DefaultState};
use log::debug;
use std::path::PathBuf;
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
    fn get_config(&self) -> Result<Config>;

    /// Reload configuration from sources.
    ///
    /// Forces a reload of configuration from all configured sources.
    /// This is useful for dynamic configuration updates.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration reloading fails.
    fn reload(&self) -> Result<()>;
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
}

impl ProductionConfigService {
    /// Create a new production configuration service.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration builder cannot be initialized.
    pub fn new() -> Result<Self> {
        let config_builder = ConfigCrate::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::from(Self::user_config_path()).required(false))
            .add_source(Environment::with_prefix("SUBX").separator("_"));

        Ok(Self {
            config_builder,
            cached_config: Arc::new(RwLock::new(None)),
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
            if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
                debug!("ProductionConfigService: Found OPENAI_API_KEY environment variable");
                app_config.ai.api_key = Some(api_key);
            }
        }

        // Special handling for OPENAI_BASE_URL environment variable
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
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
    use serial_test::serial;
    use std::env;

    /// Test helper to set environment variables safely for testing.
    struct EnvGuard {
        vars: Vec<(String, Option<String>)>, // (key, original_value)
    }

    impl EnvGuard {
        fn new() -> Self {
            Self { vars: Vec::new() }
        }

        fn set(&mut self, key: &str, value: &str) {
            // Store original value before setting
            let original = env::var(key).ok();
            self.vars.push((key.to_string(), original));

            unsafe {
                env::set_var(key, value);
            }
        }

        fn remove(&mut self, key: &str) {
            // Store original value before removing
            let original = env::var(key).ok();
            self.vars.push((key.to_string(), original));

            unsafe {
                env::remove_var(key);
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // Restore original values in reverse order
            for (key, original_value) in self.vars.iter().rev() {
                unsafe {
                    match original_value {
                        Some(value) => env::set_var(key, value),
                        None => env::remove_var(key),
                    }
                }
            }
        }
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
    #[serial]
    fn test_openai_api_key_environment_variable() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("OPENAI_API_KEY");

        // Set OPENAI_API_KEY environment variable with valid format
        env_guard.set("OPENAI_API_KEY", "sk-test-openai-key-123");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify the API key is loaded from environment
        assert_eq!(
            config.ai.api_key,
            Some("sk-test-openai-key-123".to_string())
        );

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_openai_base_url_environment_variable() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("SUBX_AI_BASE_URL");
        env_guard.remove("OPENAI_API_KEY");
        env_guard.remove("OPENAI_BASE_URL");

        // Set OPENAI_BASE_URL environment variable
        env_guard.set("OPENAI_BASE_URL", "https://custom.openai.endpoint");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify the base URL is loaded from environment
        assert_eq!(config.ai.base_url, "https://custom.openai.endpoint");

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_both_openai_environment_variables() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("SUBX_AI_BASE_URL");
        env_guard.remove("OPENAI_API_KEY");
        env_guard.remove("OPENAI_BASE_URL");

        // Set both OPENAI environment variables with valid API key format
        env_guard.set("OPENAI_API_KEY", "sk-test-api-key-combined");
        env_guard.set("OPENAI_BASE_URL", "https://api.custom-openai.com");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify both values are loaded from environment
        assert_eq!(
            config.ai.api_key,
            Some("sk-test-api-key-combined".to_string())
        );
        assert_eq!(config.ai.base_url, "https://api.custom-openai.com");

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_subx_prefix_takes_precedence_over_openai_api_key() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("OPENAI_API_KEY");

        // Set both SUBX_AI_APIKEY (preferred) and OPENAI_API_KEY (fallback) with valid format
        env_guard.set("SUBX_AI_APIKEY", "sk-subx-preferred-key");
        env_guard.set("OPENAI_API_KEY", "sk-openai-fallback-key");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify SUBX prefixed variable takes precedence
        assert_eq!(config.ai.api_key, Some("sk-subx-preferred-key".to_string()));

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_openai_api_key_fallback_when_subx_not_set() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("OPENAI_API_KEY");

        // Set only OPENAI_API_KEY with valid format
        env_guard.set("OPENAI_API_KEY", "sk-openai-fallback-only");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify OPENAI_API_KEY is used as fallback
        assert_eq!(
            config.ai.api_key,
            Some("sk-openai-fallback-only".to_string())
        );

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_openai_base_url_overrides_default() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("SUBX_AI_BASE_URL");
        env_guard.remove("OPENAI_API_KEY");
        env_guard.remove("OPENAI_BASE_URL");

        // Set OPENAI_BASE_URL to override default
        env_guard.set("OPENAI_BASE_URL", "https://my-proxy.openai.com/v1");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify the custom base URL overrides default
        assert_eq!(config.ai.base_url, "https://my-proxy.openai.com/v1");

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_config_reload_updates_environment_variables() {
        let mut env_guard = EnvGuard::new();

        // Clean up any existing environment variables first
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("OPENAI_API_KEY");

        // Start with one API key with valid format
        env_guard.set("OPENAI_API_KEY", "sk-initial-key");

        let service = ProductionConfigService::new().unwrap();
        let config1 = service.get_config().unwrap();
        assert_eq!(config1.ai.api_key, Some("sk-initial-key".to_string()));

        // Change the environment variable
        env_guard.set("OPENAI_API_KEY", "sk-updated-key");

        // Reload configuration
        service.reload().unwrap();
        let config2 = service.get_config().unwrap();

        // Verify the updated key is loaded
        assert_eq!(config2.ai.api_key, Some("sk-updated-key".to_string()));

        // Clean up is handled by EnvGuard's Drop trait
    }
    #[test]
    #[serial]
    fn test_no_openai_environment_variables_uses_defaults() {
        let mut env_guard = EnvGuard::new();

        // Remove all relevant environment variables
        env_guard.remove("OPENAI_API_KEY");
        env_guard.remove("OPENAI_BASE_URL");
        env_guard.remove("SUBX_AI_APIKEY");
        env_guard.remove("SUBX_AI_BASE_URL");

        // Create service and load configuration
        let service = ProductionConfigService::new().unwrap();
        let config = service.get_config().unwrap();

        // Verify default values are used
        assert_eq!(config.ai.api_key, None);
        assert_eq!(config.ai.base_url, "https://api.openai.com/v1"); // Default from Config::default()

        // Clean up is handled by EnvGuard's Drop trait
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
    fn test_test_config_service_for_comparison() {
        // Test our TestConfigService for comparison with valid API key format
        let test_service =
            TestConfigService::with_ai_settings_and_key("openai", "gpt-4", "sk-test-key");

        let config = test_service.get_config().unwrap();
        assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4");
    }
}
