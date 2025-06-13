//! Test configuration service for isolated testing.
//!
//! This module provides a configuration service implementation specifically
//! designed for testing environments, offering complete isolation and
//! predictable configuration states.

use crate::config::service::ConfigService;
use crate::error::SubXError;
use crate::{Result, config::Config};
use std::path::{Path, PathBuf};

/// Test configuration service implementation.
///
/// This service provides a fixed configuration for testing purposes,
/// ensuring complete isolation between tests and predictable behavior.
/// It does not load from external sources or cache.
pub struct TestConfigService {
    fixed_config: Config,
}

impl TestConfigService {
    /// Create a new test configuration service with the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The fixed configuration to use
    pub fn new(config: Config) -> Self {
        Self {
            fixed_config: config,
        }
    }

    /// Create a test configuration service with default settings.
    ///
    /// This is useful for tests that don't need specific configuration values.
    pub fn with_defaults() -> Self {
        Self::new(Config::default())
    }

    /// Create a test configuration service with specific AI settings.
    ///
    /// # Arguments
    ///
    /// * `provider` - AI provider name
    /// * `model` - AI model name
    pub fn with_ai_settings(provider: &str, model: &str) -> Self {
        let mut config = Config::default();
        config.ai.provider = provider.to_string();
        config.ai.model = model.to_string();
        Self::new(config)
    }

    /// Create a test configuration service with specific AI settings including API key.
    ///
    /// # Arguments
    ///
    /// * `provider` - AI provider name
    /// * `model` - AI model name
    /// * `api_key` - API key for the provider
    pub fn with_ai_settings_and_key(provider: &str, model: &str, api_key: &str) -> Self {
        let mut config = Config::default();
        config.ai.provider = provider.to_string();
        config.ai.model = model.to_string();
        config.ai.api_key = Some(api_key.to_string());
        Self::new(config)
    }

    /// Create a test configuration service with specific sync settings.
    ///
    /// # Arguments
    ///
    /// * `correlation_threshold` - Correlation threshold for synchronization
    /// * `max_offset` - Maximum time offset in seconds
    pub fn with_sync_settings(correlation_threshold: f32, max_offset: f32) -> Self {
        let mut config = Config::default();
        config.sync.correlation_threshold = correlation_threshold;
        config.sync.max_offset_seconds = max_offset;
        Self::new(config)
    }

    /// Create a test configuration service with specific parallel processing settings.
    ///
    /// # Arguments
    ///
    /// * `max_workers` - Maximum number of parallel workers
    /// * `queue_size` - Task queue size
    pub fn with_parallel_settings(max_workers: usize, queue_size: usize) -> Self {
        let mut config = Config::default();
        config.general.max_concurrent_jobs = max_workers;
        config.parallel.task_queue_size = queue_size;
        Self::new(config)
    }

    /// Get the underlying configuration.
    ///
    /// This is useful for tests that need direct access to the configuration object.
    pub fn config(&self) -> &Config {
        &self.fixed_config
    }

    /// Get a mutable reference to the underlying configuration.
    ///
    /// This allows tests to modify the configuration after creation.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.fixed_config
    }
}

impl ConfigService for TestConfigService {
    fn get_config(&self) -> Result<Config> {
        Ok(self.fixed_config.clone())
    }

    fn reload(&self) -> Result<()> {
        // Test configuration doesn't need reloading since it's fixed
        Ok(())
    }

    fn save_config(&self) -> Result<()> {
        // Test environment does not perform actual file I/O
        Ok(())
    }

    fn save_config_to_file(&self, _path: &Path) -> Result<()> {
        // Test environment does not perform actual file I/O
        Ok(())
    }

    fn get_config_file_path(&self) -> Result<PathBuf> {
        // Return a dummy path to avoid conflicts in test environment
        Ok(PathBuf::from("/tmp/subx_test_config.toml"))
    }

    fn get_config_value(&self, key: &str) -> Result<String> {
        // Delegate to fixed configuration
        // Note: unwrap_or_default to handle Option fields
        let config = &self.fixed_config;
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(config.ai.provider.clone()),
            ["ai", "model"] => Ok(config.ai.model.clone()),
            ["ai", "api_key"] => Ok(config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url.clone()),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            ["formats", "default_output"] => Ok(config.formats.default_output.clone()),
            ["formats", "default_encoding"] => Ok(config.formats.default_encoding.clone()),
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "correlation_threshold"] => Ok(config.sync.correlation_threshold.to_string()),
            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => {
                Ok(config.general.max_concurrent_jobs.to_string())
            }
            ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
            _ => Err(SubXError::config(format!(
                "Unknown configuration key: {}",
                key
            ))),
        }
    }

    fn reset_to_defaults(&self) -> Result<()> {
        // Reset the fixed configuration to default values
        // Note: This requires interior mutability if used concurrently
        Ok(())
    }
}

impl Default for TestConfigService {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_service_with_defaults() {
        let service = TestConfigService::with_defaults();
        let config = service.get_config().unwrap();

        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1-mini");
    }

    #[test]
    fn test_config_service_with_ai_settings() {
        let service = TestConfigService::with_ai_settings("anthropic", "claude-3");
        let config = service.get_config().unwrap();

        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.model, "claude-3");
    }

    #[test]
    fn test_config_service_with_ai_settings_and_key() {
        let service =
            TestConfigService::with_ai_settings_and_key("openai", "gpt-4.1", "test-api-key");
        let config = service.get_config().unwrap();

        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1");
        assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
    }

    #[test]
    fn test_config_service_with_sync_settings() {
        let service = TestConfigService::with_sync_settings(0.8, 45.0);
        let config = service.get_config().unwrap();

        assert_eq!(config.sync.correlation_threshold, 0.8);
        assert_eq!(config.sync.max_offset_seconds, 45.0);
    }

    #[test]
    fn test_config_service_with_parallel_settings() {
        let service = TestConfigService::with_parallel_settings(8, 200);
        let config = service.get_config().unwrap();

        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    }

    #[test]
    fn test_config_service_reload() {
        let service = TestConfigService::with_defaults();

        // Reload should always succeed for test service
        assert!(service.reload().is_ok());
    }

    #[test]
    fn test_config_service_direct_access() {
        let mut service = TestConfigService::with_defaults();

        // Test direct read access
        assert_eq!(service.config().ai.provider, "openai");

        // Test mutable access
        service.config_mut().ai.provider = "modified".to_string();
        assert_eq!(service.config().ai.provider, "modified");

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "modified");
    }
}
