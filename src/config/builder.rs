//! Configuration builder for fluent test configuration creation.
//!
//! This module provides a fluent API for building test configurations,
//! making it easy to create specific configuration scenarios for testing.

use crate::config::test_service::TestConfigService;
use crate::config::{Config, OverflowStrategy};

/// Fluent builder for creating test configurations.
///
/// This builder provides a convenient way to create configurations
/// for testing with specific settings, using method chaining for clarity.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::TestConfigBuilder;
///
/// let config = TestConfigBuilder::new()
///     .with_ai_provider("openai")
///     .with_ai_model("gpt-4")
///     .with_sync_threshold(0.8)
///     .build_config();
/// ```
pub struct TestConfigBuilder {
    config: Config,
}

impl TestConfigBuilder {
    /// Create a new configuration builder with default values.
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    // AI Configuration Methods

    /// Set the AI provider.
    ///
    /// # Arguments
    ///
    /// * `provider` - The AI provider name (e.g., "openai", "anthropic")
    pub fn with_ai_provider(mut self, provider: &str) -> Self {
        self.config.ai.provider = provider.to_string();
        self
    }

    /// Set the AI model.
    ///
    /// # Arguments
    ///
    /// * `model` - The AI model name (e.g., "gpt-4", "claude-3")
    pub fn with_ai_model(mut self, model: &str) -> Self {
        self.config.ai.model = model.to_string();
        self
    }

    /// Set the AI API key.
    ///
    /// # Arguments
    ///
    /// * `api_key` - The API key for authentication
    pub fn with_ai_api_key(mut self, api_key: &str) -> Self {
        self.config.ai.api_key = Some(api_key.to_string());
        self
    }

    /// Set the AI base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL for the AI service
    pub fn with_ai_base_url(mut self, base_url: &str) -> Self {
        self.config.ai.base_url = base_url.to_string();
        self
    }

    /// Set the maximum sample length for AI requests.
    ///
    /// # Arguments
    ///
    /// * `length` - Maximum sample length in characters
    pub fn with_max_sample_length(mut self, length: usize) -> Self {
        self.config.ai.max_sample_length = length;
        self
    }

    /// Set the AI temperature parameter.
    ///
    /// # Arguments
    ///
    /// * `temperature` - Temperature value (0.0-1.0)
    pub fn with_ai_temperature(mut self, temperature: f32) -> Self {
        self.config.ai.temperature = temperature;
        self
    }

    /// Set the AI retry parameters.
    ///
    /// # Arguments
    ///
    /// * `attempts` - Number of retry attempts
    /// * `delay_ms` - Delay between retries in milliseconds
    pub fn with_ai_retry(mut self, attempts: u32, delay_ms: u64) -> Self {
        self.config.ai.retry_attempts = attempts;
        self.config.ai.retry_delay_ms = delay_ms;
        self
    }

    // Sync Configuration Methods

    /// Set the sync correlation threshold.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Correlation threshold (0.0-1.0)
    pub fn with_sync_threshold(mut self, threshold: f32) -> Self {
        self.config.sync.correlation_threshold = threshold;
        self
    }

    /// Set the maximum offset for synchronization.
    ///
    /// # Arguments
    ///
    /// * `offset` - Maximum offset in seconds
    pub fn with_max_offset(mut self, offset: f32) -> Self {
        self.config.sync.max_offset_seconds = offset;
        self
    }

    /// Set the audio sample rate.
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz
    pub fn with_audio_sample_rate(mut self, sample_rate: u32) -> Self {
        self.config.sync.audio_sample_rate = sample_rate;
        self
    }

    /// Enable or disable dialogue detection.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable dialogue detection
    pub fn with_dialogue_detection(mut self, enabled: bool) -> Self {
        self.config.sync.enable_dialogue_detection = enabled;
        self
    }

    /// Set dialogue detection parameters.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Detection threshold (0.0-1.0)
    /// * `min_duration_ms` - Minimum dialogue duration in milliseconds
    /// * `merge_gap_ms` - Gap for merging dialogue segments in milliseconds
    pub fn with_dialogue_params(
        mut self,
        threshold: f32,
        min_duration_ms: u64,
        merge_gap_ms: u64,
    ) -> Self {
        self.config.sync.dialogue_detection_threshold = threshold;
        self.config.sync.min_dialogue_duration_ms = min_duration_ms;
        self.config.sync.dialogue_merge_gap_ms = merge_gap_ms;
        self
    }

    /// Enable or disable auto-detection of sample rate.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable auto-detection
    pub fn with_auto_detect_sample_rate(mut self, enabled: bool) -> Self {
        self.config.sync.auto_detect_sample_rate = enabled;
        self
    }

    // Formats Configuration Methods

    /// Set the default output format.
    ///
    /// # Arguments
    ///
    /// * `format` - The output format (e.g., "srt", "ass", "vtt")
    pub fn with_default_output_format(mut self, format: &str) -> Self {
        self.config.formats.default_output = format.to_string();
        self
    }

    /// Enable or disable style preservation.
    ///
    /// # Arguments
    ///
    /// * `preserve` - Whether to preserve styling
    pub fn with_preserve_styling(mut self, preserve: bool) -> Self {
        self.config.formats.preserve_styling = preserve;
        self
    }

    /// Set the default encoding.
    ///
    /// # Arguments
    ///
    /// * `encoding` - The default encoding (e.g., "utf-8", "gbk")
    pub fn with_default_encoding(mut self, encoding: &str) -> Self {
        self.config.formats.default_encoding = encoding.to_string();
        self
    }

    /// Set the encoding detection confidence threshold.
    ///
    /// # Arguments
    ///
    /// * `confidence` - Confidence threshold (0.0-1.0)
    pub fn with_encoding_detection_confidence(mut self, confidence: f32) -> Self {
        self.config.formats.encoding_detection_confidence = confidence;
        self
    }

    // General Configuration Methods

    /// Enable or disable backup.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable backup
    pub fn with_backup_enabled(mut self, enabled: bool) -> Self {
        self.config.general.backup_enabled = enabled;
        self
    }

    /// Set the maximum number of concurrent jobs.
    ///
    /// # Arguments
    ///
    /// * `jobs` - Maximum concurrent jobs
    pub fn with_max_concurrent_jobs(mut self, jobs: usize) -> Self {
        self.config.general.max_concurrent_jobs = jobs;
        self
    }

    /// Set the task timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Timeout in seconds
    pub fn with_task_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.general.task_timeout_seconds = timeout_seconds;
        self
    }

    /// Enable or disable progress bar.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable progress bar
    pub fn with_progress_bar(mut self, enabled: bool) -> Self {
        self.config.general.enable_progress_bar = enabled;
        self
    }

    /// Set the worker idle timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Idle timeout in seconds
    pub fn with_worker_idle_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.general.worker_idle_timeout_seconds = timeout_seconds;
        self
    }

    // Parallel Configuration Methods

    /// Set the task queue size.
    ///
    /// # Arguments
    ///
    /// * `size` - Queue size
    pub fn with_task_queue_size(mut self, size: usize) -> Self {
        self.config.parallel.task_queue_size = size;
        self
    }

    /// Enable or disable task priorities.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable task priorities
    pub fn with_task_priorities(mut self, enabled: bool) -> Self {
        self.config.parallel.enable_task_priorities = enabled;
        self
    }

    /// Enable or disable auto-balancing of workers.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable auto-balancing
    pub fn with_auto_balance_workers(mut self, enabled: bool) -> Self {
        self.config.parallel.auto_balance_workers = enabled;
        self
    }

    /// Set the queue overflow strategy.
    ///
    /// # Arguments
    ///
    /// * `strategy` - Overflow strategy
    pub fn with_queue_overflow_strategy(mut self, strategy: OverflowStrategy) -> Self {
        self.config.parallel.queue_overflow_strategy = strategy;
        self
    }

    // Builder Methods

    /// Build a test configuration service with the configured settings.
    pub fn build_service(self) -> TestConfigService {
        TestConfigService::new(self.config)
    }

    /// Build a configuration object with the configured settings.
    pub fn build_config(self) -> Config {
        self.config
    }

    /// Get a reference to the current configuration being built.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get a mutable reference to the current configuration being built.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::service::ConfigService;

    #[test]
    fn test_builder_default() {
        let config = TestConfigBuilder::new().build_config();
        let default_config = Config::default();

        assert_eq!(config.ai.provider, default_config.ai.provider);
        assert_eq!(config.ai.model, default_config.ai.model);
    }

    #[test]
    fn test_builder_ai_configuration() {
        let config = TestConfigBuilder::new()
            .with_ai_provider("anthropic")
            .with_ai_model("claude-3")
            .with_ai_api_key("test-key")
            .with_max_sample_length(5000)
            .with_ai_temperature(0.7)
            .build_config();

        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.model, "claude-3");
        assert_eq!(config.ai.api_key, Some("test-key".to_string()));
        assert_eq!(config.ai.max_sample_length, 5000);
        assert_eq!(config.ai.temperature, 0.7);
    }

    #[test]
    fn test_builder_sync_configuration() {
        let config = TestConfigBuilder::new()
            .with_sync_threshold(0.9)
            .with_max_offset(60.0)
            .with_audio_sample_rate(48000)
            .with_dialogue_detection(false)
            .build_config();

        assert_eq!(config.sync.correlation_threshold, 0.9);
        assert_eq!(config.sync.max_offset_seconds, 60.0);
        assert_eq!(config.sync.audio_sample_rate, 48000);
        assert!(!config.sync.enable_dialogue_detection);
    }

    #[test]
    fn test_builder_service_creation() {
        let service = TestConfigBuilder::new()
            .with_ai_provider("test-provider")
            .build_service();

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "test-provider");
    }

    #[test]
    fn test_builder_chaining() {
        let config = TestConfigBuilder::new()
            .with_ai_provider("openai")
            .with_ai_model("gpt-4")
            .with_sync_threshold(0.8)
            .with_max_concurrent_jobs(8)
            .with_task_queue_size(200)
            .build_config();

        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4");
        assert_eq!(config.sync.correlation_threshold, 0.8);
        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    }
}
