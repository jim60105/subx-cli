//! Configuration data structures for the SubX CLI application.
//!
//! This module provides configuration type definitions used throughout
//! the application. These types are now instantiated through the new
//! configuration service system rather than global configuration managers.

use crate::{Result, error::SubXError};
use log::debug;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// config crate imports for new configuration system
use config::{Config as ConfigBuilder, Environment, File};

/// Full application configuration.
///
/// This struct aggregates all settings for AI integration, subtitle format
/// conversion, synchronization, general options, and parallel execution.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// AI service configuration parameters.
    pub ai: AIConfig,
    /// Subtitle format conversion settings.
    pub formats: FormatsConfig,
    /// Audio-subtitle synchronization options.
    pub sync: SyncConfig,
    /// General runtime options (e.g., backup enabled, job limits).
    pub general: GeneralConfig,
    /// Parallel processing parameters.
    pub parallel: ParallelConfig,
    /// Optional file path from which the configuration was loaded.
    pub loaded_from: Option<PathBuf>,
}

impl Config {
    /// Get the user configuration file path.
    ///
    /// This method determines the appropriate configuration file path based on
    /// environment variables and user directories.
    ///
    /// # Returns
    ///
    /// Returns the path to the user's configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration directory cannot be determined.
    pub fn config_file_path() -> Result<PathBuf> {
        // Check for custom config path from environment
        if let Ok(custom) = std::env::var("SUBX_CONFIG_PATH") {
            let path = PathBuf::from(custom);
            return Ok(path);
        }

        // Use default user config directory
        let config_dir = dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?;
        let default_path = config_dir.join("subx").join("config.toml");
        Ok(default_path)
    }

    /// Save configuration to TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where to save the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or file writing fails.
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }

        std::fs::write(path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Save configuration to default file path.
    ///
    /// # Errors
    ///
    /// Returns an error if saving fails.
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        self.save_to_file(&config_path)
    }

    /// Get a configuration value by key path.
    ///
    /// # Arguments
    ///
    /// * `key` - Dot-separated key path (e.g., "ai.provider")
    ///
    /// # Errors
    ///
    /// Returns an error if the key is not found.
    pub fn get_value(&self, key: &str) -> Result<String> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(self.ai.provider.clone()),
            ["ai", "model"] => Ok(self.ai.model.clone()),
            ["ai", "api_key"] => Ok(self.ai.api_key.clone().unwrap_or_default()),
            ["formats", "default_output"] => Ok(self.formats.default_output.clone()),
            ["sync", "max_offset_seconds"] => Ok(self.sync.max_offset_seconds.to_string()),
            ["sync", "correlation_threshold"] => Ok(self.sync.correlation_threshold.to_string()),
            ["general", "backup_enabled"] => Ok(self.general.backup_enabled.to_string()),
            ["parallel", "max_workers"] => Ok(self.parallel.max_workers.to_string()),
            _ => Err(SubXError::config(format!(
                "Unknown configuration key: {}",
                key
            ))),
        }
    }

    /// Create configuration from sources.
    ///
    /// This method builds a configuration by merging settings from multiple sources
    /// in order of precedence: defaults, config file, environment variables, and
    /// command-line overrides.
    ///
    /// # Returns
    ///
    /// Returns a complete configuration loaded from all available sources.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading or parsing fails.
    pub fn create_config_from_sources() -> Result<Self> {
        let config_path = Self::config_file_path()?;

        let settings = ConfigBuilder::builder()
            // Start with defaults
            .add_source(ConfigBuilder::try_from(&Self::default())?)
            // Add file source if it exists
            .add_source(File::from(config_path).required(false))
            // Add environment variables with SUBX_ prefix
            .add_source(Environment::with_prefix("SUBX").separator("_"))
            .build()
            .map_err(|e| {
                debug!("create_config_from_sources: Config build failed: {}", e);
                SubXError::config(format!("Configuration build failed: {}", e))
            })?;

        let config: Self = settings.try_deserialize().map_err(|e| {
            debug!("create_config_from_sources: Deserialization failed: {}", e);
            SubXError::config(format!("Configuration deserialization failed: {}", e))
        })?;

        debug!("create_config_from_sources: Configuration loaded successfully");
        Ok(config)
    }
}

/// AI service provider configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    /// AI provider name (e.g. "openai", "anthropic")
    pub provider: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// AI model name to use
    pub model: String,
    /// API base URL
    pub base_url: String,
    /// Maximum sample length per request
    pub max_sample_length: usize,
    /// AI generation creativity parameter (0.0-1.0)
    pub temperature: f32,
    /// Number of retries on request failure
    pub retry_attempts: u32,
    /// Retry interval in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_sample_length: 3000,
            temperature: 0.3,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Subtitle format related configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    /// Default output format (e.g. "srt", "ass", "vtt")
    pub default_output: String,
    /// Whether to preserve style information during format conversion
    pub preserve_styling: bool,
    /// Default character encoding (e.g. "utf-8", "gbk")
    pub default_encoding: String,
    /// Encoding detection confidence threshold (0.0-1.0)
    pub encoding_detection_confidence: f32,
}

impl Default for FormatsConfig {
    fn default() -> Self {
        Self {
            default_output: "srt".to_string(),
            preserve_styling: false,
            default_encoding: "utf-8".to_string(),
            encoding_detection_confidence: 0.8,
        }
    }
}

/// Audio synchronization related configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    /// Maximum offset in seconds for synchronization
    pub max_offset_seconds: f32,
    /// Audio sample rate for processing
    pub audio_sample_rate: u32,
    /// Correlation threshold for sync quality (0.0-1.0)
    pub correlation_threshold: f32,
    /// Dialogue detection threshold (0.0-1.0)
    pub dialogue_detection_threshold: f32,
    /// Minimum dialogue duration in milliseconds
    pub min_dialogue_duration_ms: u32,
    /// Gap between dialogues for merging (milliseconds)
    pub dialogue_merge_gap_ms: u32,
    /// Enable dialogue detection
    pub enable_dialogue_detection: bool,
    /// Auto-detect sample rate from audio files
    pub auto_detect_sample_rate: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_offset_seconds: 10.0,
            audio_sample_rate: 44100,
            correlation_threshold: 0.8,
            dialogue_detection_threshold: 0.6,
            min_dialogue_duration_ms: 500,
            dialogue_merge_gap_ms: 200,
            enable_dialogue_detection: true,
            auto_detect_sample_rate: true,
        }
    }
}

/// General application configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    /// Enable automatic backup of original files
    pub backup_enabled: bool,
    /// Maximum number of concurrent processing jobs
    pub max_concurrent_jobs: usize,
    /// Temporary directory for processing
    pub temp_dir: Option<PathBuf>,
    /// Log level for application output
    pub log_level: String,
    /// Cache directory for storing processed data
    pub cache_dir: Option<PathBuf>,
    /// Task timeout in seconds
    pub task_timeout_seconds: u64,
    /// Enable progress bar display
    pub enable_progress_bar: bool,
    /// Worker idle timeout in seconds
    pub worker_idle_timeout_seconds: u64,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            backup_enabled: false,
            max_concurrent_jobs: 4,
            temp_dir: None,
            log_level: "info".to_string(),
            cache_dir: None,
            task_timeout_seconds: 300,
            enable_progress_bar: true,
            worker_idle_timeout_seconds: 60,
        }
    }
}

/// Parallel processing configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    /// Maximum number of worker threads
    pub max_workers: usize,
    /// Chunk size for parallel processing
    pub chunk_size: usize,
    /// Overflow strategy when workers are busy
    pub overflow_strategy: OverflowStrategy,
    /// Enable work stealing between workers
    pub enable_work_stealing: bool,
    /// Task queue size
    pub task_queue_size: usize,
    /// Enable task priorities
    pub enable_task_priorities: bool,
    /// Auto-balance workers
    pub auto_balance_workers: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_workers: num_cpus::get(),
            chunk_size: 1000,
            overflow_strategy: OverflowStrategy::Block,
            enable_work_stealing: true,
            task_queue_size: 1000,
            enable_task_priorities: false,
            auto_balance_workers: true,
        }
    }
}

/// Strategy for handling overflow when all workers are busy.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OverflowStrategy {
    /// Block until a worker becomes available
    Block,
    /// Drop new tasks when all workers are busy
    Drop,
    /// Create additional temporary workers
    Expand,
    /// Drop oldest tasks in queue
    DropOldest,
    /// Reject new tasks
    Reject,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = Config::default();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4o-mini");
        assert_eq!(config.formats.default_output, "srt");
        assert!(!config.general.backup_enabled);
        assert_eq!(config.general.max_concurrent_jobs, 4);
    }

    #[test]
    fn test_ai_config_defaults() {
        let ai_config = AIConfig::default();
        assert_eq!(ai_config.provider, "openai");
        assert_eq!(ai_config.model, "gpt-4o-mini");
        assert_eq!(ai_config.temperature, 0.3);
        assert_eq!(ai_config.max_sample_length, 3000);
    }

    #[test]
    fn test_sync_config_defaults() {
        let sync_config = SyncConfig::default();
        assert_eq!(sync_config.max_offset_seconds, 10.0);
        assert_eq!(sync_config.correlation_threshold, 0.8);
        assert_eq!(sync_config.audio_sample_rate, 44100);
        assert!(sync_config.enable_dialogue_detection);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[ai]"));
        assert!(toml_str.contains("[sync]"));
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[parallel]"));
    }
}
