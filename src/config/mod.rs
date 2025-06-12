// src/config/mod.rs
//! Configuration management module for SubX.
//!
//! This module provides the complete configuration service system with
//! dependency injection support and comprehensive type definitions.
//!
//! # Key Components
//!
//! - [`Config`] - Main configuration structure containing all settings
//! - [`ConfigService`] - Service interface for configuration management
//! - [`ProductionConfigService`] - Production implementation with file I/O
//! - [`TestConfigService`] - Test implementation with controlled behavior
//! - [`TestConfigBuilder`] - Builder pattern for test configurations
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::config::{Config, ConfigService, ProductionConfigService};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a production configuration service
//! let config_service = ProductionConfigService::new()?;
//!
//! // Load configuration
//! let config = config_service.get_config()?;
//! println!("AI Provider: {}", config.ai.provider);
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The configuration system uses dependency injection to provide testable
//! and maintainable configuration management. All configuration access
//! should go through the [`ConfigService`] trait.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Configuration service system
pub mod builder;
pub mod environment;
pub mod service;
pub mod test_macros;
pub mod test_service;
pub mod validator;

// ============================================================================
// Configuration Type Definitions
// ============================================================================

/// Full application configuration for SubX.
///
/// This struct aggregates all settings for AI integration, subtitle format
/// conversion, synchronization, general options, and parallel execution.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::Config;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::default();
/// assert_eq!(config.ai.provider, "openai");
/// assert_eq!(config.formats.default_output, "srt");
/// # Ok(())
/// # }
/// ```
///
/// # Serialization
///
/// This struct can be serialized to/from TOML format for configuration files.
///
/// ```rust
/// use subx_cli::config::Config;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::default();
/// let toml_str = toml::to_string(&config)?;
/// assert!(toml_str.contains("[ai]"));
/// # Ok(())
/// # }
/// ```
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

/// AI service provider configuration.
///
/// Contains all settings required for AI provider integration,
/// including authentication, model selection, and retry behavior.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::AIConfig;
///
/// let ai_config = AIConfig::default();
/// assert_eq!(ai_config.provider, "openai");
/// assert_eq!(ai_config.model, "gpt-4.1-mini");
/// assert_eq!(ai_config.temperature, 0.3);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    /// AI provider name (e.g. "openai", "anthropic").
    pub provider: String,
    /// API key for authentication.
    pub api_key: Option<String>,
    /// AI model name to use.
    pub model: String,
    /// API base URL.
    pub base_url: String,
    /// Maximum sample length per request.
    pub max_sample_length: usize,
    /// AI generation creativity parameter (0.0-1.0).
    pub temperature: f32,
    /// Number of retries on request failure.
    pub retry_attempts: u32,
    /// Retry interval in milliseconds.
    pub retry_delay_ms: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: None,
            model: "gpt-4.1-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_sample_length: 3000,
            temperature: 0.3,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Subtitle format related configuration.
///
/// Controls how subtitle files are processed, including format conversion,
/// encoding detection, and style preservation.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::FormatsConfig;
///
/// let formats = FormatsConfig::default();
/// assert_eq!(formats.default_output, "srt");
/// assert_eq!(formats.default_encoding, "utf-8");
/// assert!(!formats.preserve_styling);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    /// Default output format (e.g. "srt", "ass", "vtt").
    pub default_output: String,
    /// Whether to preserve style information during format conversion.
    pub preserve_styling: bool,
    /// Default character encoding (e.g. "utf-8", "gbk").
    pub default_encoding: String,
    /// Encoding detection confidence threshold (0.0-1.0).
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
///
/// Contains settings for audio-subtitle synchronization operations,
/// including offset limits, sample rates, and dialogue detection.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::SyncConfig;
///
/// let sync = SyncConfig::default();
/// assert_eq!(sync.max_offset_seconds, 10.0);
/// assert_eq!(sync.audio_sample_rate, 44100);
/// assert!(sync.enable_dialogue_detection);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    /// Maximum offset in seconds for synchronization.
    pub max_offset_seconds: f32,
    /// Audio sample rate for processing.
    pub audio_sample_rate: u32,
    /// Correlation threshold for sync quality (0.0-1.0).
    pub correlation_threshold: f32,
    /// Dialogue detection threshold (0.0-1.0).
    pub dialogue_detection_threshold: f32,
    /// Minimum dialogue duration in milliseconds.
    pub min_dialogue_duration_ms: u32,
    /// Gap between dialogues for merging (milliseconds).
    pub dialogue_merge_gap_ms: u32,
    /// Enable dialogue detection.
    pub enable_dialogue_detection: bool,
    /// Auto-detect sample rate from audio files.
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
///
/// Contains general runtime options that affect the overall behavior
/// of the application, including backup settings, job limits, and logging.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::GeneralConfig;
///
/// let general = GeneralConfig::default();
/// assert!(!general.backup_enabled);
/// assert_eq!(general.max_concurrent_jobs, 4);
/// assert_eq!(general.log_level, "info");
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    /// Enable automatic backup of original files.
    pub backup_enabled: bool,
    /// Maximum number of concurrent processing jobs.
    pub max_concurrent_jobs: usize,
    /// Temporary directory for processing.
    pub temp_dir: Option<PathBuf>,
    /// Log level for application output.
    pub log_level: String,
    /// Cache directory for storing processed data.
    pub cache_dir: Option<PathBuf>,
    /// Task timeout in seconds.
    pub task_timeout_seconds: u64,
    /// Enable progress bar display.
    pub enable_progress_bar: bool,
    /// Worker idle timeout in seconds.
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
///
/// Controls how parallel processing is performed, including worker
/// management, task distribution, and overflow handling strategies.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::{ParallelConfig, OverflowStrategy};
///
/// let parallel = ParallelConfig::default();
/// assert!(parallel.max_workers > 0);
/// assert_eq!(parallel.overflow_strategy, OverflowStrategy::Block);
/// assert!(parallel.enable_work_stealing);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    /// Maximum number of worker threads.
    pub max_workers: usize,
    /// Chunk size for parallel processing.
    pub chunk_size: usize,
    /// Overflow strategy when workers are busy.
    pub overflow_strategy: OverflowStrategy,
    /// Enable work stealing between workers.
    pub enable_work_stealing: bool,
    /// Task queue size.
    pub task_queue_size: usize,
    /// Enable task priorities.
    pub enable_task_priorities: bool,
    /// Auto-balance workers.
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
///
/// This enum defines different strategies for handling situations where
/// all worker threads are occupied and new tasks arrive.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::OverflowStrategy;
///
/// let strategy = OverflowStrategy::Block;
/// assert_eq!(strategy, OverflowStrategy::Block);
///
/// // Comparison and serialization
/// let strategies = vec![
///     OverflowStrategy::Block,
///     OverflowStrategy::Drop,
///     OverflowStrategy::Expand,
/// ];
/// assert_eq!(strategies.len(), 3);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OverflowStrategy {
    /// Block until a worker becomes available.
    ///
    /// This is the safest option as it ensures all tasks are processed,
    /// but may cause the application to become unresponsive.
    Block,
    /// Drop new tasks when all workers are busy.
    ///
    /// Use this when task loss is acceptable and responsiveness is critical.
    Drop,
    /// Create additional temporary workers.
    ///
    /// This can help with load spikes but may consume excessive resources.
    Expand,
    /// Drop oldest tasks in queue.
    ///
    /// Prioritizes recent tasks over older ones in the queue.
    DropOldest,
    /// Reject new tasks.
    ///
    /// Similar to Drop but may provide error feedback to the caller.
    Reject,
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = Config::default();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4.1-mini");
        assert_eq!(config.formats.default_output, "srt");
        assert!(!config.general.backup_enabled);
        assert_eq!(config.general.max_concurrent_jobs, 4);
    }

    #[test]
    fn test_ai_config_defaults() {
        let ai_config = AIConfig::default();
        assert_eq!(ai_config.provider, "openai");
        assert_eq!(ai_config.model, "gpt-4.1-mini");
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

    #[test]
    fn test_overflow_strategy_equality() {
        assert_eq!(OverflowStrategy::Block, OverflowStrategy::Block);
        assert_ne!(OverflowStrategy::Block, OverflowStrategy::Drop);
    }
}

// ============================================================================
// Public API Re-exports
// ============================================================================

// Re-export the configuration service system
pub use builder::TestConfigBuilder;
pub use environment::{EnvironmentProvider, SystemEnvironmentProvider, TestEnvironmentProvider};
pub use service::{ConfigService, ProductionConfigService};
pub use test_service::TestConfigService;
