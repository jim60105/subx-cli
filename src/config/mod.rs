// src/config/mod.rs
#![allow(deprecated)]
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
//! # Validation System
//!
//! The configuration system provides a layered validation architecture:
//!
//! - [`validation`] - Low-level validation functions for individual values
//! - [`validator`] - High-level configuration section validators
//! - [`field_validator`] - Key-value validation for configuration service
//!
//! ## Architecture
//!
//! ```text
//! ConfigService
//!      ↓
//! field_validator (key-value validation)
//!      ↓
//! validation (primitive validation functions)
//!
//! validator (section validation)
//!      ↓
//! validation (primitive validation functions)
//! ```
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
pub mod field_validator;
pub mod service;
pub mod test_macros;
pub mod test_service;
pub mod validation;
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
/// assert_eq!(config.ai.provider, "free");
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

/// AI service configuration parameters.
///
/// This structure defines all configuration options for AI providers,
/// including authentication, model parameters, retry behavior, and timeouts.
///
/// # Examples
///
/// Creating a default configuration:
/// ```rust
/// use subx_cli::config::AIConfig;
///
/// let ai_config = AIConfig::default();
/// assert_eq!(ai_config.provider, "free");
/// assert_eq!(ai_config.model, "deepseek/deepseek-r1-0528:free");
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
    /// Maximum tokens in response.
    pub max_tokens: u32,
    /// Number of retries on request failure.
    pub retry_attempts: u32,
    /// Retry interval in milliseconds.
    pub retry_delay_ms: u64,
    /// HTTP request timeout in seconds.
    /// This controls how long to wait for a response from the AI service.
    /// For slow networks or complex requests, you may need to increase this value.
    pub request_timeout_seconds: u64,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: "free".to_string(),
            api_key: None,
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            max_sample_length: 3000,
            temperature: 0.3,
            max_tokens: 10000,
            retry_attempts: 3,
            retry_delay_ms: 1000,
            // Set to 120 seconds to handle slow networks and complex AI requests
            // This is especially important for users with high-latency connections
            request_timeout_seconds: 120,
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

/// Audio synchronization configuration supporting VAD speech detection.
///
/// This configuration struct defines settings for subtitle-audio synchronization,
/// including method selection, timing constraints, and VAD-specific parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    /// Default synchronization method ("vad", "auto")
    pub default_method: String,
    /// Maximum allowed time offset in seconds
    pub max_offset_seconds: f32,
    /// Local VAD related settings
    pub vad: VadConfig,

    // Deprecated legacy fields, preserved for backward compatibility
    /// Deprecated: correlation threshold for audio analysis
    #[deprecated]
    #[serde(skip)]
    pub correlation_threshold: f32,
    /// Deprecated: dialogue detection threshold
    #[deprecated]
    #[serde(skip)]
    pub dialogue_detection_threshold: f32,
    /// Deprecated: minimum dialogue duration in milliseconds
    #[deprecated]
    #[serde(skip)]
    pub min_dialogue_duration_ms: u32,
    /// Deprecated: dialogue merge gap in milliseconds
    #[deprecated]
    #[serde(skip)]
    pub dialogue_merge_gap_ms: u32,
    /// Deprecated: enable dialogue detection flag
    #[deprecated]
    #[serde(skip)]
    pub enable_dialogue_detection: bool,
    /// Deprecated: audio sample rate
    #[deprecated]
    #[serde(skip)]
    pub audio_sample_rate: u32,
    /// Deprecated: auto-detect sample rate flag  
    #[deprecated]
    #[serde(skip)]
    pub auto_detect_sample_rate: bool,
}

/// Local Voice Activity Detection configuration.
///
/// This struct defines parameters for local VAD processing, including sensitivity,
/// audio chunking, and speech segment filtering. Adjust these fields to control
/// how strictly speech is detected and how short segments are filtered out.
///
/// # Fields
///
/// - `enabled`: Whether local VAD is enabled
/// - `sensitivity`: Speech detection sensitivity (0.0-1.0). Lower values are stricter and less likely to classify audio as speech.
/// - `padding_chunks`: Number of non-speech chunks to include before and after detected speech
/// - `min_speech_duration_ms`: Minimum duration (ms) for a segment to be considered valid speech
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::VadConfig;
///
/// let vad = VadConfig::default();
/// assert!(vad.enabled);
/// assert_eq!(vad.sensitivity, 0.25);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VadConfig {
    /// Whether to enable local VAD method.
    pub enabled: bool,
    /// Speech detection sensitivity (0.0-1.0).
    ///
    /// Lower values are stricter: a smaller value means the detector is less likely to classify a chunk as speech.
    /// For example, 0.25 is more strict than 0.75.
    pub sensitivity: f32,
    /// Number of non-speech chunks to pad before and after detected speech.
    pub padding_chunks: u32,
    /// Minimum speech duration in milliseconds.
    ///
    /// Segments shorter than this value will be discarded as noise or non-speech.
    pub min_speech_duration_ms: u32,
}

#[allow(deprecated)]
impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            default_method: "auto".to_string(),
            max_offset_seconds: 60.0,
            vad: VadConfig::default(),
            correlation_threshold: 0.8,
            dialogue_detection_threshold: 0.6,
            min_dialogue_duration_ms: 500,
            dialogue_merge_gap_ms: 200,
            enable_dialogue_detection: true,
            audio_sample_rate: 44100,
            auto_detect_sample_rate: true,
        }
    }
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.25, // Default changed to 0.25, the smaller the value, the stricter the detection
            padding_chunks: 3,
            min_speech_duration_ms: 300,
        }
    }
}

/// General configuration settings for the SubX CLI tool.
///
/// This struct contains general settings that control the overall behavior
/// of the application, including backup policies, processing limits, and
/// user interface preferences.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::GeneralConfig;
///
/// let config = GeneralConfig::default();
/// assert_eq!(config.max_concurrent_jobs, 4);
/// assert!(!config.backup_enabled);
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    /// Enable automatic backup of original files.
    pub backup_enabled: bool,
    /// Maximum number of concurrent processing jobs.
    pub max_concurrent_jobs: usize,
    /// Task timeout in seconds.
    pub task_timeout_seconds: u64,
    /// Workspace directory for CLI commands (override current working directory).
    pub workspace: std::path::PathBuf,
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
            task_timeout_seconds: 300,
            // Default workspace is current directory
            workspace: std::path::PathBuf::from("."),
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
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    /// Maximum number of worker threads.
    pub max_workers: usize,
    /// Strategy for handling task overflow when queues are full.
    ///
    /// Determines the behavior when the task queue reaches capacity.
    /// - [`OverflowStrategy::Block`] - Block until space is available
    /// - [`OverflowStrategy::Drop`] - Drop new tasks when full
    /// - [`OverflowStrategy::Expand`] - Dynamically expand queue size
    pub overflow_strategy: OverflowStrategy,
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
            overflow_strategy: OverflowStrategy::Block,
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
        assert_eq!(config.ai.provider, "free");
        assert_eq!(config.ai.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(config.formats.default_output, "srt");
        assert!(!config.general.backup_enabled);
        assert_eq!(config.general.max_concurrent_jobs, 4);
    }

    #[test]
    fn test_ai_config_defaults() {
        let ai_config = AIConfig::default();
        assert_eq!(ai_config.provider, "free");
        assert_eq!(ai_config.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(ai_config.temperature, 0.3);
        assert_eq!(ai_config.max_sample_length, 3000);
        assert_eq!(ai_config.max_tokens, 10000);
    }

    #[test]
    fn test_ai_config_max_tokens_configuration() {
        let mut ai_config = AIConfig::default();
        ai_config.max_tokens = 5000;
        assert_eq!(ai_config.max_tokens, 5000);

        // Test with different value
        ai_config.max_tokens = 20000;
        assert_eq!(ai_config.max_tokens, 20000);
    }

    #[test]
    fn test_new_sync_config_defaults() {
        let sync = SyncConfig::default();
        assert_eq!(sync.default_method, "auto");
        assert_eq!(sync.max_offset_seconds, 60.0);
        assert!(sync.vad.enabled);
    }

    #[test]
    fn test_sync_config_validation() {
        let mut sync = SyncConfig::default();

        // Valid configuration should pass validation
        assert!(sync.validate().is_ok());

        // Invalid default_method
        sync.default_method = "invalid".to_string();
        assert!(sync.validate().is_err());

        // Reset and test other invalid values
        sync = SyncConfig::default();
        sync.max_offset_seconds = -1.0;
        assert!(sync.validate().is_err());
    }

    #[test]
    fn test_vad_config_validation() {
        let mut vad = VadConfig::default();

        // Valid configuration
        assert!(vad.validate().is_ok());

        // Invalid sensitivity
        vad.sensitivity = 1.5;
        assert!(vad.validate().is_err());
    }

    #[test]
    fn test_config_serialization_with_new_sync() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();

        // Ensure new configuration structure exists in serialized output
        assert!(toml_str.contains("[sync]"));
        assert!(toml_str.contains("[sync.vad]"));
        assert!(toml_str.contains("default_method"));
        // Whisper-related fields removed, should not appear in serialized output
        assert!(!toml_str.contains("[sync.whisper]"));
        assert!(!toml_str.contains("analysis_window_seconds"));
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

// Re-export commonly used validation functions
pub use field_validator::validate_field;
pub use validator::validate_config;
