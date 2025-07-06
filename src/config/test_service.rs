//! Test configuration service for isolated testing.
//!
//! This module provides a configuration service implementation specifically
//! designed for testing environments, offering complete isolation and
//! predictable configuration states.

use crate::config::service::ConfigService;
use crate::error::SubXError;
use crate::{Result, config::Config};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Test configuration service implementation.
///
/// This service provides a fixed configuration for testing purposes,
/// ensuring complete isolation between tests and predictable behavior.
/// It does not load from external sources or cache.
pub struct TestConfigService {
    config: Mutex<Config>,
}

impl TestConfigService {
    /// Set AI provider, model, and API key for testing.
    pub fn set_ai_settings_and_key(&self, provider: &str, model: &str, api_key: &str) {
        let mut cfg = self.config.lock().unwrap();
        cfg.ai.provider = provider.to_string();
        cfg.ai.model = model.to_string();
        cfg.ai.api_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key.to_string())
        };
    }

    /// Set AI provider, model, API key, and custom base URL for testing.
    pub fn set_ai_settings_with_base_url(
        &self,
        provider: &str,
        model: &str,
        api_key: &str,
        base_url: &str,
    ) {
        self.set_ai_settings_and_key(provider, model, api_key);
        let mut cfg = self.config.lock().unwrap();
        cfg.ai.base_url = base_url.to_string();
    }
    /// Create a new test configuration service with the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The fixed configuration to use
    pub fn new(config: Config) -> Self {
        Self {
            config: Mutex::new(config),
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
    pub fn config(&self) -> std::sync::MutexGuard<'_, Config> {
        self.config.lock().unwrap()
    }

    /// Get a mutable reference to the underlying configuration.
    ///
    /// This allows tests to modify the configuration after creation.
    pub fn config_mut(&self) -> std::sync::MutexGuard<'_, Config> {
        self.config.lock().unwrap()
    }
}

impl ConfigService for TestConfigService {
    fn get_config(&self) -> Result<Config> {
        Ok(self.config.lock().unwrap().clone())
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
        // Delegate to current configuration
        // Note: unwrap_or_default to handle Option fields
        let config = self.config.lock().unwrap();
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(config.ai.provider.clone()),
            ["ai", "model"] => Ok(config.ai.model.clone()),
            ["ai", "api_key"] => Ok(config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url.clone()),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            ["ai", "max_sample_length"] => Ok(config.ai.max_sample_length.to_string()),
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
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "default_method"] => Ok(config.sync.default_method.clone()),
            ["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
            ["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
            ["sync", "vad", "padding_chunks"] => Ok(config.sync.vad.padding_chunks.to_string()),
            ["sync", "vad", "min_speech_duration_ms"] => {
                Ok(config.sync.vad.min_speech_duration_ms.to_string())
            }
            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "task_timeout_seconds"] => {
                Ok(config.general.task_timeout_seconds.to_string())
            }
            ["general", "enable_progress_bar"] => {
                Ok(config.general.enable_progress_bar.to_string())
            }
            ["general", "worker_idle_timeout_seconds"] => {
                Ok(config.general.worker_idle_timeout_seconds.to_string())
            }
            ["general", "max_concurrent_jobs"] => {
                Ok(config.general.max_concurrent_jobs.to_string())
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
                "Unknown configuration key: {key}"
            ))),
        }
    }

    fn reset_to_defaults(&self) -> Result<()> {
        // Reset the configuration to default values
        *self.config.lock().unwrap() = Config::default();
        Ok(())
    }

    fn set_config_value(&self, key: &str, value: &str) -> Result<()> {
        // Load current configuration
        let mut cfg = self.get_config()?;
        // Validate and set the value using the same logic as ProductionConfigService
        self.validate_and_set_value(&mut cfg, key, value)?;
        // Validate the entire configuration
        crate::config::validator::validate_config(&cfg)?;
        // Update the internal configuration
        *self.config.lock().unwrap() = cfg;
        Ok(())
    }
}

impl TestConfigService {
    /// Validate and set a configuration value (same logic as ProductionConfigService).
    fn validate_and_set_value(&self, config: &mut Config, key: &str, value: &str) -> Result<()> {
        use crate::config::OverflowStrategy;
        use crate::config::validation::*;
        use crate::error::SubXError;

        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => {
                validate_enum(value, &["openai", "anthropic", "local"])?;
                config.ai.provider = value.to_string();
            }
            ["ai", "api_key"] => {
                if !value.is_empty() {
                    validate_api_key(value)?;
                    config.ai.api_key = Some(value.to_string());
                } else {
                    config.ai.api_key = None;
                }
            }
            ["ai", "model"] => {
                config.ai.model = value.to_string();
            }
            ["ai", "base_url"] => {
                validate_url(value)?;
                config.ai.base_url = value.to_string();
            }
            ["ai", "max_sample_length"] => {
                let v = validate_usize_range(value, 100, 10000)?;
                config.ai.max_sample_length = v;
            }
            ["ai", "temperature"] => {
                let v = validate_float_range(value, 0.0, 1.0)?;
                config.ai.temperature = v;
            }
            ["ai", "max_tokens"] => {
                let v = validate_uint_range(value, 1, 100_000)?;
                config.ai.max_tokens = v;
            }
            ["ai", "retry_attempts"] => {
                let v = validate_uint_range(value, 1, 10)?;
                config.ai.retry_attempts = v;
            }
            ["ai", "retry_delay_ms"] => {
                let v = validate_u64_range(value, 100, 30000)?;
                config.ai.retry_delay_ms = v;
            }
            ["ai", "request_timeout_seconds"] => {
                let v = validate_u64_range(value, 10, 600)?;
                config.ai.request_timeout_seconds = v;
            }
            ["formats", "default_output"] => {
                validate_enum(value, &["srt", "ass", "vtt", "webvtt"])?;
                config.formats.default_output = value.to_string();
            }
            ["formats", "preserve_styling"] => {
                let v = parse_bool(value)?;
                config.formats.preserve_styling = v;
            }
            ["formats", "default_encoding"] => {
                validate_enum(value, &["utf-8", "gbk", "big5", "shift_jis"])?;
                config.formats.default_encoding = value.to_string();
            }
            ["formats", "encoding_detection_confidence"] => {
                let v = validate_float_range(value, 0.0, 1.0)?;
                config.formats.encoding_detection_confidence = v;
            }
            ["sync", "max_offset_seconds"] => {
                let v = validate_float_range(value, 0.0, 300.0)?;
                config.sync.max_offset_seconds = v;
            }
            ["sync", "default_method"] => {
                validate_enum(value, &["auto", "vad"])?;
                config.sync.default_method = value.to_string();
            }
            ["sync", "vad", "enabled"] => {
                let v = parse_bool(value)?;
                config.sync.vad.enabled = v;
            }
            ["sync", "vad", "sensitivity"] => {
                let v = validate_float_range(value, 0.0, 1.0)?;
                config.sync.vad.sensitivity = v;
            }
            ["sync", "vad", "padding_chunks"] => {
                let v = validate_uint_range(value, 0, u32::MAX)?;
                config.sync.vad.padding_chunks = v;
            }
            ["sync", "vad", "min_speech_duration_ms"] => {
                let v = validate_uint_range(value, 0, u32::MAX)?;
                config.sync.vad.min_speech_duration_ms = v;
            }
            ["sync", "correlation_threshold"] => {
                let v = validate_float_range(value, 0.0, 1.0)?;
                config.sync.correlation_threshold = v;
            }
            ["sync", "dialogue_detection_threshold"] => {
                let v = validate_float_range(value, 0.0, 1.0)?;
                config.sync.dialogue_detection_threshold = v;
            }
            ["sync", "min_dialogue_duration_ms"] => {
                let v = validate_uint_range(value, 100, 5000)?;
                config.sync.min_dialogue_duration_ms = v;
            }
            ["sync", "dialogue_merge_gap_ms"] => {
                let v = validate_uint_range(value, 50, 2000)?;
                config.sync.dialogue_merge_gap_ms = v;
            }
            ["sync", "enable_dialogue_detection"] => {
                let v = parse_bool(value)?;
                config.sync.enable_dialogue_detection = v;
            }
            ["sync", "audio_sample_rate"] => {
                let v = validate_uint_range(value, 8000, 192000)?;
                config.sync.audio_sample_rate = v;
            }
            ["sync", "auto_detect_sample_rate"] => {
                let v = parse_bool(value)?;
                config.sync.auto_detect_sample_rate = v;
            }
            ["general", "backup_enabled"] => {
                let v = parse_bool(value)?;
                config.general.backup_enabled = v;
            }
            ["general", "max_concurrent_jobs"] => {
                let v = validate_usize_range(value, 1, 64)?;
                config.general.max_concurrent_jobs = v;
            }
            ["general", "task_timeout_seconds"] => {
                let v = validate_u64_range(value, 30, 3600)?;
                config.general.task_timeout_seconds = v;
            }
            ["general", "enable_progress_bar"] => {
                let v = parse_bool(value)?;
                config.general.enable_progress_bar = v;
            }
            ["general", "worker_idle_timeout_seconds"] => {
                let v = validate_u64_range(value, 10, 3600)?;
                config.general.worker_idle_timeout_seconds = v;
            }
            ["parallel", "max_workers"] => {
                let v = validate_usize_range(value, 1, 64)?;
                config.parallel.max_workers = v;
            }
            ["parallel", "task_queue_size"] => {
                let v = validate_usize_range(value, 100, 10000)?;
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
                validate_enum(value, &["Block", "Drop", "Expand"])?;
                config.parallel.overflow_strategy = match value {
                    "Block" => OverflowStrategy::Block,
                    "Drop" => OverflowStrategy::Drop,
                    "Expand" => OverflowStrategy::Expand,
                    _ => unreachable!(),
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

        assert_eq!(config.ai.provider, "free");
        assert_eq!(config.ai.model, "deepseek/deepseek-r1-0528:free");
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
    fn test_config_service_with_ai_settings_and_key_openrouter() {
        let service = TestConfigService::with_ai_settings_and_key(
            "openrouter",
            "deepseek/deepseek-r1-0528:free",
            "test-openrouter-key",
        );
        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openrouter");
        assert_eq!(config.ai.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(config.ai.api_key, Some("test-openrouter-key".to_string()));
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
        let service = TestConfigService::with_defaults();

        // Test direct read access
        assert_eq!(service.config().ai.provider, "free");

        // Test mutable access
        service.config_mut().ai.provider = "modified".to_string();
        assert_eq!(service.config().ai.provider, "modified");

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "modified");
    }
}
