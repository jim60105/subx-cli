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
///     .with_ai_model("gpt-4.1")
///     .with_vad_enabled(true)
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
    /// * `model` - The AI model name (e.g., "gpt-4.1", "claude-3")
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

    /// Set the AI max tokens parameter.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - Maximum tokens in response (1-100000)
    pub fn with_ai_max_tokens(mut self, max_tokens: u32) -> Self {
        self.config.ai.max_tokens = max_tokens;
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

    /// Set the AI request timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout_seconds` - Request timeout in seconds
    pub fn with_ai_request_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.ai.request_timeout_seconds = timeout_seconds;
        self
    }

    // Sync Configuration Methods

    /// Set the synchronization method.
    ///
    /// # Arguments
    ///
    /// * `method` - The sync method to use ("vad", "auto", "manual")
    pub fn with_sync_method(mut self, method: &str) -> Self {
        self.config.sync.default_method = method.to_string();
        self
    }

    /// Enable or disable local VAD.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable local VAD processing
    pub fn with_vad_enabled(mut self, enabled: bool) -> Self {
        self.config.sync.vad.enabled = enabled;
        self
    }

    /// Set VAD sensitivity.
    ///
    /// # Arguments
    ///
    /// * `sensitivity` - VAD sensitivity value (0.0-1.0)
    pub fn with_vad_sensitivity(mut self, sensitivity: f32) -> Self {
        self.config.sync.vad.sensitivity = sensitivity;
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
        self.config.parallel.overflow_strategy = strategy;
        self
    }

    /// Set the number of parallel workers and queue size, used for integration testing.
    pub fn with_parallel_settings(mut self, max_workers: usize, queue_size: usize) -> Self {
        self.config.general.max_concurrent_jobs = max_workers;
        self.config.parallel.task_queue_size = queue_size;
        self
    }

    // Translation Configuration Methods

    /// Set the translation batch size used for AI translation requests.
    pub fn with_translation_batch_size(mut self, batch_size: usize) -> Self {
        self.config.translation.batch_size = batch_size;
        self
    }

    /// Set the default target language for the `translate` command.
    pub fn with_translation_default_target_language(mut self, language: &str) -> Self {
        self.config.translation.default_target_language = Some(language.to_string());
        self
    }

    /// Clear the default target language for the `translate` command.
    pub fn without_translation_default_target_language(mut self) -> Self {
        self.config.translation.default_target_language = None;
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
    /// Configure AI base URL for mock server integration testing.
    ///
    /// Sets up the configuration to use a mock AI server for testing purposes.
    /// This is primarily used in integration tests to avoid making real API calls.
    ///
    /// # Arguments
    ///
    /// - `mock_url`: The URL of the mock server to use for AI API calls
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::config::TestConfigBuilder;
    ///
    /// let config = TestConfigBuilder::new()
    ///     .with_mock_ai_server("http://localhost:3000")
    ///     .build_config();
    /// ```
    pub fn with_mock_ai_server(mut self, mock_url: &str) -> Self {
        self.config.ai.base_url = mock_url.to_string();
        self.config.ai.api_key = Some("mock-api-key".to_string());
        self
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
            .with_sync_method("vad")
            .with_vad_enabled(true)
            .with_vad_sensitivity(0.8)
            .build_config();

        assert_eq!(config.sync.default_method, "vad");
        assert!(config.sync.vad.enabled);
        assert_eq!(config.sync.vad.sensitivity, 0.8);
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
            .with_ai_model("gpt-4.1")
            .with_sync_method("vad")
            .with_vad_sensitivity(0.5)
            .with_max_concurrent_jobs(8)
            .with_task_queue_size(200)
            .build_config();

        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1");
        assert_eq!(config.sync.default_method, "vad");
        assert_eq!(config.sync.vad.sensitivity, 0.5);
        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    }

    #[test]
    fn test_builder_ai_configuration_openrouter() {
        let config = TestConfigBuilder::new()
            .with_ai_provider("openrouter")
            .with_ai_model("deepseek/deepseek-r1-0528:free")
            .with_ai_api_key("test-openrouter-key")
            .build_config();
        assert_eq!(config.ai.provider, "openrouter");
        assert_eq!(config.ai.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(config.ai.api_key, Some("test-openrouter-key".to_string()));
    }

    #[test]
    fn test_builder_default_trait() {
        let config = TestConfigBuilder::default().build_config();
        let default_config = Config::default();
        assert_eq!(config.ai.provider, default_config.ai.provider);
        assert_eq!(config.ai.model, default_config.ai.model);
        assert_eq!(
            config.formats.default_output,
            default_config.formats.default_output
        );
    }

    #[test]
    fn test_builder_with_ai_base_url() {
        let config = TestConfigBuilder::new()
            .with_ai_base_url("https://custom.api.example.com/v2")
            .build_config();
        assert_eq!(config.ai.base_url, "https://custom.api.example.com/v2");
    }

    #[test]
    fn test_builder_with_ai_max_tokens() {
        let config = TestConfigBuilder::new()
            .with_ai_max_tokens(4096)
            .build_config();
        assert_eq!(config.ai.max_tokens, 4096);
    }

    #[test]
    fn test_builder_with_ai_retry() {
        let config = TestConfigBuilder::new()
            .with_ai_retry(5, 2000)
            .build_config();
        assert_eq!(config.ai.retry_attempts, 5);
        assert_eq!(config.ai.retry_delay_ms, 2000);
    }

    #[test]
    fn test_builder_with_ai_request_timeout() {
        let config = TestConfigBuilder::new()
            .with_ai_request_timeout(60)
            .build_config();
        assert_eq!(config.ai.request_timeout_seconds, 60);
    }

    #[test]
    fn test_builder_with_default_output_format() {
        let config = TestConfigBuilder::new()
            .with_default_output_format("ass")
            .build_config();
        assert_eq!(config.formats.default_output, "ass");
    }

    #[test]
    fn test_builder_with_preserve_styling_true() {
        let config = TestConfigBuilder::new()
            .with_preserve_styling(true)
            .build_config();
        assert!(config.formats.preserve_styling);
    }

    #[test]
    fn test_builder_with_preserve_styling_false() {
        let config = TestConfigBuilder::new()
            .with_preserve_styling(false)
            .build_config();
        assert!(!config.formats.preserve_styling);
    }

    #[test]
    fn test_builder_with_default_encoding() {
        let config = TestConfigBuilder::new()
            .with_default_encoding("gbk")
            .build_config();
        assert_eq!(config.formats.default_encoding, "gbk");
    }

    #[test]
    fn test_builder_with_encoding_detection_confidence() {
        let config = TestConfigBuilder::new()
            .with_encoding_detection_confidence(0.95)
            .build_config();
        assert!((config.formats.encoding_detection_confidence - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_with_backup_enabled_true() {
        let config = TestConfigBuilder::new()
            .with_backup_enabled(true)
            .build_config();
        assert!(config.general.backup_enabled);
    }

    #[test]
    fn test_builder_with_backup_enabled_false() {
        let config = TestConfigBuilder::new()
            .with_backup_enabled(false)
            .build_config();
        assert!(!config.general.backup_enabled);
    }

    #[test]
    fn test_builder_with_task_timeout() {
        let config = TestConfigBuilder::new()
            .with_task_timeout(600)
            .build_config();
        assert_eq!(config.general.task_timeout_seconds, 600);
    }

    #[test]
    fn test_builder_with_progress_bar_enabled() {
        let config = TestConfigBuilder::new()
            .with_progress_bar(true)
            .build_config();
        assert!(config.general.enable_progress_bar);
    }

    #[test]
    fn test_builder_with_progress_bar_disabled() {
        let config = TestConfigBuilder::new()
            .with_progress_bar(false)
            .build_config();
        assert!(!config.general.enable_progress_bar);
    }

    #[test]
    fn test_builder_with_worker_idle_timeout() {
        let config = TestConfigBuilder::new()
            .with_worker_idle_timeout(120)
            .build_config();
        assert_eq!(config.general.worker_idle_timeout_seconds, 120);
    }

    #[test]
    fn test_builder_with_task_priorities_enabled() {
        let config = TestConfigBuilder::new()
            .with_task_priorities(true)
            .build_config();
        assert!(config.parallel.enable_task_priorities);
    }

    #[test]
    fn test_builder_with_task_priorities_disabled() {
        let config = TestConfigBuilder::new()
            .with_task_priorities(false)
            .build_config();
        assert!(!config.parallel.enable_task_priorities);
    }

    #[test]
    fn test_builder_with_auto_balance_workers_enabled() {
        let config = TestConfigBuilder::new()
            .with_auto_balance_workers(true)
            .build_config();
        assert!(config.parallel.auto_balance_workers);
    }

    #[test]
    fn test_builder_with_auto_balance_workers_disabled() {
        let config = TestConfigBuilder::new()
            .with_auto_balance_workers(false)
            .build_config();
        assert!(!config.parallel.auto_balance_workers);
    }

    #[test]
    fn test_builder_with_queue_overflow_strategy_block() {
        let config = TestConfigBuilder::new()
            .with_queue_overflow_strategy(OverflowStrategy::Block)
            .build_config();
        assert_eq!(config.parallel.overflow_strategy, OverflowStrategy::Block);
    }

    #[test]
    fn test_builder_with_queue_overflow_strategy_drop() {
        let config = TestConfigBuilder::new()
            .with_queue_overflow_strategy(OverflowStrategy::Drop)
            .build_config();
        assert_eq!(config.parallel.overflow_strategy, OverflowStrategy::Drop);
    }

    #[test]
    fn test_builder_with_queue_overflow_strategy_expand() {
        let config = TestConfigBuilder::new()
            .with_queue_overflow_strategy(OverflowStrategy::Expand)
            .build_config();
        assert_eq!(config.parallel.overflow_strategy, OverflowStrategy::Expand);
    }

    #[test]
    fn test_builder_with_queue_overflow_strategy_drop_oldest() {
        let config = TestConfigBuilder::new()
            .with_queue_overflow_strategy(OverflowStrategy::DropOldest)
            .build_config();
        assert_eq!(
            config.parallel.overflow_strategy,
            OverflowStrategy::DropOldest
        );
    }

    #[test]
    fn test_builder_with_queue_overflow_strategy_reject() {
        let config = TestConfigBuilder::new()
            .with_queue_overflow_strategy(OverflowStrategy::Reject)
            .build_config();
        assert_eq!(config.parallel.overflow_strategy, OverflowStrategy::Reject);
    }

    #[test]
    fn test_builder_with_parallel_settings() {
        let config = TestConfigBuilder::new()
            .with_parallel_settings(16, 500)
            .build_config();
        assert_eq!(config.general.max_concurrent_jobs, 16);
        assert_eq!(config.parallel.task_queue_size, 500);
    }

    #[test]
    fn test_builder_config_ref() {
        let builder = TestConfigBuilder::new().with_ai_provider("ref-provider");
        let config_ref = builder.config();
        assert_eq!(config_ref.ai.provider, "ref-provider");
    }

    #[test]
    fn test_builder_config_mut() {
        let mut builder = TestConfigBuilder::new();
        builder.config_mut().ai.provider = "mutated-provider".to_string();
        let config = builder.build_config();
        assert_eq!(config.ai.provider, "mutated-provider");
    }

    #[test]
    fn test_builder_with_mock_ai_server() {
        let config = TestConfigBuilder::new()
            .with_mock_ai_server("http://localhost:8080")
            .build_config();
        assert_eq!(config.ai.base_url, "http://localhost:8080");
        assert_eq!(config.ai.api_key, Some("mock-api-key".to_string()));
    }

    #[test]
    fn test_builder_override_preserves_other_fields() {
        let config = TestConfigBuilder::new()
            .with_ai_provider("openai")
            .with_ai_model("gpt-4.1")
            .with_ai_temperature(0.9)
            .with_ai_provider("anthropic")
            .build_config();
        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.model, "gpt-4.1");
        assert!((config.ai.temperature - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_full_ai_configuration() {
        let config = TestConfigBuilder::new()
            .with_ai_provider("openai")
            .with_ai_model("gpt-4.1")
            .with_ai_api_key("sk-test-key")
            .with_ai_base_url("https://api.openai.com/v1")
            .with_max_sample_length(8000)
            .with_ai_temperature(0.5)
            .with_ai_max_tokens(2000)
            .with_ai_retry(2, 500)
            .with_ai_request_timeout(30)
            .build_config();

        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1");
        assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
        assert_eq!(config.ai.base_url, "https://api.openai.com/v1");
        assert_eq!(config.ai.max_sample_length, 8000);
        assert!((config.ai.temperature - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.ai.max_tokens, 2000);
        assert_eq!(config.ai.retry_attempts, 2);
        assert_eq!(config.ai.retry_delay_ms, 500);
        assert_eq!(config.ai.request_timeout_seconds, 30);
    }

    #[test]
    fn test_builder_full_formats_configuration() {
        let config = TestConfigBuilder::new()
            .with_default_output_format("vtt")
            .with_preserve_styling(true)
            .with_default_encoding("utf-16")
            .with_encoding_detection_confidence(0.75)
            .build_config();

        assert_eq!(config.formats.default_output, "vtt");
        assert!(config.formats.preserve_styling);
        assert_eq!(config.formats.default_encoding, "utf-16");
        assert!((config.formats.encoding_detection_confidence - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_full_general_configuration() {
        let config = TestConfigBuilder::new()
            .with_backup_enabled(true)
            .with_max_concurrent_jobs(2)
            .with_task_timeout(120)
            .with_progress_bar(false)
            .with_worker_idle_timeout(30)
            .build_config();

        assert!(config.general.backup_enabled);
        assert_eq!(config.general.max_concurrent_jobs, 2);
        assert_eq!(config.general.task_timeout_seconds, 120);
        assert!(!config.general.enable_progress_bar);
        assert_eq!(config.general.worker_idle_timeout_seconds, 30);
    }

    #[test]
    fn test_builder_full_parallel_configuration() {
        let config = TestConfigBuilder::new()
            .with_task_queue_size(256)
            .with_task_priorities(true)
            .with_auto_balance_workers(false)
            .with_queue_overflow_strategy(OverflowStrategy::Expand)
            .build_config();

        assert_eq!(config.parallel.task_queue_size, 256);
        assert!(config.parallel.enable_task_priorities);
        assert!(!config.parallel.auto_balance_workers);
        assert_eq!(config.parallel.overflow_strategy, OverflowStrategy::Expand);
    }

    #[test]
    fn test_builder_vad_disabled() {
        let config = TestConfigBuilder::new()
            .with_vad_enabled(false)
            .build_config();
        assert!(!config.sync.vad.enabled);
    }

    #[test]
    fn test_builder_vad_sensitivity_boundary_zero() {
        let config = TestConfigBuilder::new()
            .with_vad_sensitivity(0.0)
            .build_config();
        assert!((config.sync.vad.sensitivity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_vad_sensitivity_boundary_one() {
        let config = TestConfigBuilder::new()
            .with_vad_sensitivity(1.0)
            .build_config();
        assert!((config.sync.vad.sensitivity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_service_returns_correct_config() {
        let service = TestConfigBuilder::new()
            .with_ai_provider("test-ai")
            .with_ai_model("test-model")
            .with_backup_enabled(true)
            .build_service();

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "test-ai");
        assert_eq!(config.ai.model, "test-model");
        assert!(config.general.backup_enabled);
    }

    #[test]
    fn test_builder_parallel_settings_overrides_individual_methods() {
        let config = TestConfigBuilder::new()
            .with_max_concurrent_jobs(4)
            .with_task_queue_size(100)
            .with_parallel_settings(8, 200)
            .build_config();
        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    }
}
