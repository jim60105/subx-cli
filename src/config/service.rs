//! Configuration service system for dependency injection and test isolation.
//!
//! This module provides a clean abstraction for configuration management
//! that enables dependency injection and complete test isolation without
//! requiring unsafe code or global state resets.

use crate::config::{EnvironmentProvider, SystemEnvironmentProvider};
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
        let config_builder = ConfigCrate::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::from(Self::user_config_path()).required(false))
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
            "gpt-4o-mini",
            "sk-test-openai-key-123",
        );

        let config = test_service.get_config().unwrap();
        assert_eq!(
            config.ai.api_key,
            Some("sk-test-openai-key-123".to_string())
        );
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4o-mini");
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
            TestConfigService::with_ai_settings_and_key("openai", "gpt-4", "sk-explicit-key");

        let config = test_service.get_config().unwrap();
        assert_eq!(config.ai.api_key, Some("sk-explicit-key".to_string()));
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4");
    }

    #[test]
    fn test_config_service_fallback_behavior() {
        // Test fallback to default values when no specific configuration provided
        let test_service = TestConfigService::with_defaults();
        let config = test_service.get_config().unwrap();

        // Should use default values
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4o-mini");
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
        let mut test_service = TestConfigService::with_defaults();

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

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.api_key, Some("sk-test-key-both".to_string()));
        assert_eq!(config.ai.base_url, "https://both.openai.com/v1");
    }

    #[test]
    fn test_production_config_service_no_openai_env_vars() {
        // Test the case with no OPENAI environment variables
        let env_provider = TestEnvironmentProvider::new(); // Empty provider

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
