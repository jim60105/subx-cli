//! Configuration management module for the SubX CLI application.
//!
//! This module provides functionality to initialize and load the application
//! configuration from multiple sources, including files, environment variables,
//! and command-line arguments.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::{init_config_manager, load_config, Config, Result};
//!
//! fn main() -> Result<()> {
//!     // Initialize global configuration manager and load settings
//!     init_config_manager()?;
//!     let config: Config = load_config()?;
//!     // Use the loaded configuration as needed
//!     println!("Loaded AI provider: {}", config.ai.provider);
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{Result, error::SubXError};
use log::debug;

// Submodules for unified configuration management core
pub mod cache;
pub mod manager;
pub mod partial;
pub mod source;
pub mod validator;

use crate::config::manager::ConfigManager;
use crate::config::source::{CliSource, EnvSource, FileSource};
use std::sync::{Mutex, OnceLock};

static GLOBAL_CONFIG_MANAGER: OnceLock<Mutex<ConfigManager>> = OnceLock::new();

/// Reset the global configuration manager (for testing only).
///
/// This function clears the internal OnceLock, allowing a fresh
/// ConfigManager to be created on the next call to `init_config_manager()`.
#[allow(invalid_reference_casting)]
pub fn reset_global_config_manager() {
    // Use ptr::write to reconstruct OnceLock, overwriting previous lock state.
    unsafe {
        // Get a mutable pointer to the static variable and overwrite previous state with a new OnceLock.
        let dst = &GLOBAL_CONFIG_MANAGER as *const _ as *mut OnceLock<Mutex<ConfigManager>>;
        std::ptr::write(dst, OnceLock::new());
    }
}

/// Initialize the global configuration manager.
///
/// This function builds a new ConfigManager with file, environment,
/// and CLI sources, loads the merged settings, and stores it in a
/// global lock for subsequent retrieval.
///
/// # Errors
///
/// Returns a `SubXError::Config` variant if loading or parsing the
/// configuration fails.
pub fn init_config_manager() -> Result<()> {
    let lock = GLOBAL_CONFIG_MANAGER.get_or_init(|| Mutex::new(ConfigManager::new()));

    // Get config file path (environment variables should be set at this point)
    let config_path = Config::config_file_path()?;
    debug!("init_config_manager: Using config path: {:?}", config_path);
    debug!(
        "init_config_manager: Config path exists: {}",
        config_path.exists()
    );

    let manager = ConfigManager::new()
        .add_source(Box::new(FileSource::new(config_path)))
        .add_source(Box::new(EnvSource::new()))
        .add_source(Box::new(CliSource::new()));
    debug!("init_config_manager: Created manager with 3 sources");

    manager.load().map_err(|e| {
        debug!("init_config_manager: Manager load failed: {}", e);
        SubXError::config(e.to_string())
    })?;
    debug!("init_config_manager: Manager loaded successfully");

    let mut guard = lock.lock().unwrap();
    *guard = manager;
    debug!("init_config_manager: Updated global manager");
    Ok(())
}

/// Load the complete application configuration.
///
/// Retrieves the global ConfigManager initialized by `init_config_manager()`,
/// reads the merged partial configuration, and converts it into a full `Config`.
///
/// # Errors
///
/// Returns a `SubXError::Config` if the global manager is not initialized
/// or if validation or conversion to the complete `Config` fails.
pub fn load_config() -> Result<Config> {
    debug!("load_config: Getting global config manager");
    let lock = GLOBAL_CONFIG_MANAGER.get().ok_or_else(|| {
        debug!("load_config: Global config manager not initialized");
        SubXError::config("global config manager not initialized; call init_config_manager() first")
    })?;
    debug!("load_config: Locking manager");
    let manager = lock.lock().unwrap();
    let config_lock = manager.config();
    debug!("load_config: Getting partial config");
    let partial_config = config_lock.read().unwrap();
    debug!(
        "load_config: partial_config.ai.max_sample_length = {:?}",
        partial_config.ai.max_sample_length
    );
    debug!("load_config: Converting to complete config");
    let config = partial_config.to_complete_config().map_err(|e| {
        debug!("load_config: to_complete_config failed: {}", e);
        SubXError::config(e.to_string())
    })?;
    debug!(
        "load_config: Final config.ai.max_sample_length = {}",
        config.ai.max_sample_length
    );
    Ok(config)
}

/// Full application configuration.
///
/// This struct aggregates all settings for AI integration, subtitle format
/// conversion, synchronization, general options, and parallel execution.
///
/// # Fields
///
/// - `ai`: AI service configuration parameters.
/// - `formats`: Subtitle format conversion settings.
/// - `sync`: Audio-subtitle synchronization options.
/// - `general`: General runtime options (e.g., backup and concurrency).
/// - `parallel`: Parallel processing parameters.
/// - `loaded_from`: Optional file path from which the configuration was loaded.
#[derive(Debug, Serialize, Deserialize, Clone)]
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
    /// Source path of the loaded configuration file, if any.
    #[serde(skip)]
    pub loaded_from: Option<PathBuf>,
}

// Unit test: Config management functionality
#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tempfile::TempDir;

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
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("[ai]"));
        assert!(toml_str.contains("[formats]"));
        assert!(toml_str.contains("[sync]"));
        assert!(toml_str.contains("[general]"));
    }

    #[test]
    #[serial]
    fn test_env_var_override() {
        // Clear environment variables to avoid interference between tests
        unsafe {
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("SUBX_AI_MODEL");
            env::set_var("OPENAI_API_KEY", "test-key-123");
            env::set_var("SUBX_AI_MODEL", "gpt-3.5-turbo");
        }

        let mut config = Config::default();
        config.apply_env_vars();
        assert!(config.ai.api_key.is_some());
        assert_eq!(config.ai.model, "gpt-3.5-turbo");

        unsafe {
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("SUBX_AI_MODEL");
        }
    }

    #[test]
    #[serial]
    fn test_config_validation_missing_api_key() {
        unsafe {
            env::remove_var("OPENAI_API_KEY");
        }
        let config = Config::default();
        // API Key validation is performed at runtime, does not affect loading
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_provider() {
        let mut config = Config::default();
        config.ai.provider = "invalid-provider".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_file_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let original_config = Config::default();
        let toml_content = toml::to_string_pretty(&original_config).unwrap();
        std::fs::write(&config_path, toml_content).unwrap();

        let file_content = std::fs::read_to_string(&config_path).unwrap();
        let loaded_config: Config = toml::from_str(&file_content).unwrap();

        assert_eq!(original_config.ai.model, loaded_config.ai.model);
        assert_eq!(
            original_config.formats.default_output,
            loaded_config.formats.default_output
        );
    }

    #[test]
    fn test_config_merge() {
        let mut base_config = Config::default();
        let mut override_config = Config::default();
        override_config.ai.model = "gpt-4".to_string();
        override_config.general.backup_enabled = true;

        base_config.merge(override_config);

        assert_eq!(base_config.ai.model, "gpt-4");
        assert!(base_config.general.backup_enabled);
    }

    #[test]
    fn test_global_config_manager_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let test_config = Config::default();
        let toml_content = toml::to_string_pretty(&test_config).unwrap();
        std::fs::write(&config_path, toml_content).unwrap();

        unsafe {
            std::env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        }

        assert!(init_config_manager().is_ok());

        let loaded_config = load_config().unwrap();
        assert_eq!(loaded_config.ai.model, test_config.ai.model);

        unsafe {
            std::env::remove_var("SUBX_CONFIG_PATH");
        }
    }

    #[test]
    fn test_env_var_override_with_new_system() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "test-key-from-env");
            std::env::set_var("SUBX_AI_MODEL", "gpt-4-from-env");
        }

        let _ = init_config_manager();
        let config = load_config().unwrap();

        assert_eq!(config.ai.api_key, Some("test-key-from-env".to_string()));
        assert_eq!(config.ai.model, "gpt-4-from-env");

        unsafe {
            std::env::remove_var("OPENAI_API_KEY");
            std::env::remove_var("SUBX_AI_MODEL");
        }
    }
}

/// AI service provider configuration
///
/// This struct contains all configuration options for AI service providers, including API key, model settings, and retry strategy.
/// Supports multiple AI providers, including OpenAI, Claude, etc.
///
/// # Fields
///
/// * `provider` - AI provider name (e.g. "openai", "claude")
/// * `api_key` - API key for authentication
/// * `model` - AI model name to use
/// * `base_url` - API base URL
/// * `max_sample_length` - Maximum sample length per request
/// * `temperature` - AI generation creativity parameter (0.0-1.0)
/// * `retry_attempts` - Number of retries on request failure
/// * `retry_delay_ms` - Retry interval in milliseconds
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::AIConfig;
///
/// let ai_config = AIConfig {
///     provider: "openai".to_string(),
///     api_key: Some("your-api-key".to_string()),
///     model: "gpt-4".to_string(),
///     base_url: "https://api.openai.com/v1".to_string(),
///     max_sample_length: 4000,
///     temperature: 0.3,
///     retry_attempts: 3,
///     retry_delay_ms: 1000,
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    /// AI provider name (e.g. "openai", "claude")
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

/// Subtitle format related configuration
///
/// Controls various options for subtitle format conversion and processing, including default output format,
/// style preservation, and encoding handling settings.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::FormatsConfig;
///
/// let config = FormatsConfig {
///     default_output: "srt".to_string(),
///     preserve_styling: true,
///     default_encoding: "utf-8".to_string(),
///     encoding_detection_confidence: 0.8,
/// };
/// ```
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

/// Audio synchronization related configuration
///
/// Controls various parameters for audio-subtitle synchronization algorithms, including maximum offset,
/// correlation threshold, and dialogue detection settings.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::SyncConfig;
///
/// let config = SyncConfig {
///     max_offset_seconds: 30.0,
///     audio_sample_rate: 44100,
///     correlation_threshold: 0.8,
///     dialogue_detection_threshold: 0.6,
///     min_dialogue_duration_ms: 500,
///     dialogue_merge_gap_ms: 200,
///     enable_dialogue_detection: true,
///     auto_detect_sample_rate: true,
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    /// Maximum allowed time offset (seconds)
    pub max_offset_seconds: f32,
    /// Audio processing sample rate (Hz)
    pub audio_sample_rate: u32,
    /// Correlation analysis threshold (0.0-1.0)
    pub correlation_threshold: f32,
    /// Dialogue detection threshold (0.0-1.0)
    pub dialogue_detection_threshold: f32,
    /// Minimum dialogue duration (milliseconds)
    pub min_dialogue_duration_ms: u64,
    /// Dialogue segment merge gap (milliseconds)
    pub dialogue_merge_gap_ms: u64,
    /// Whether to enable dialogue detection
    pub enable_dialogue_detection: bool,
    /// Whether to auto-detect original sample rate
    pub auto_detect_sample_rate: bool,
}

impl SyncConfig {
    /// Whether to auto-detect original sample rate
    pub fn auto_detect_sample_rate(&self) -> bool {
        self.auto_detect_sample_rate
    }
}

/// General configuration
///
/// Controls general application behavior options, including backup, parallel processing, and user interface settings.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::GeneralConfig;
///
/// let config = GeneralConfig {
///     backup_enabled: true,
///     max_concurrent_jobs: 4,
///     task_timeout_seconds: 300,
///     enable_progress_bar: true,
///     worker_idle_timeout_seconds: 60,
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    /// Whether to enable automatic backup
    pub backup_enabled: bool,
    /// Maximum number of concurrent jobs
    pub max_concurrent_jobs: usize,
    /// Timeout for a single task (seconds)
    pub task_timeout_seconds: u64,
    /// Whether to show progress bar
    pub enable_progress_bar: bool,
    /// Worker idle timeout (seconds)
    pub worker_idle_timeout_seconds: u64,
}

/// Parallel processing related configuration
///
/// Controls various parameters for parallel task processing, including queue size, priority management, and load balancing strategy.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::{ParallelConfig, OverflowStrategy};
///
/// let config = ParallelConfig {
///     task_queue_size: 100,
///     enable_task_priorities: true,
///     auto_balance_workers: true,
///     queue_overflow_strategy: OverflowStrategy::Block,
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    /// Maximum size of the task queue
    pub task_queue_size: usize,
    /// Whether to enable task priority management
    pub enable_task_priorities: bool,
    /// Whether to auto-balance worker load
    pub auto_balance_workers: bool,
    /// Strategy to apply when the task queue reaches its maximum size.
    pub queue_overflow_strategy: OverflowStrategy,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        ParallelConfig {
            task_queue_size: 100,
            enable_task_priorities: true,
            auto_balance_workers: true,
            queue_overflow_strategy: OverflowStrategy::Block,
        }
    }
}

/// Strategy to apply when the parallel task queue is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverflowStrategy {
    /// Block until space is available.
    Block,
    /// Drop the oldest task in the queue.
    DropOldest,
    /// Reject new tasks when full.
    Reject,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ai: AIConfig {
                provider: "openai".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                max_sample_length: 2000,
                temperature: 0.3,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
            formats: FormatsConfig {
                default_output: "srt".to_string(),
                preserve_styling: true,
                default_encoding: "utf-8".to_string(),
                encoding_detection_confidence: 0.7,
            },
            sync: SyncConfig {
                max_offset_seconds: 30.0,
                audio_sample_rate: 16000,
                correlation_threshold: 0.7,
                dialogue_detection_threshold: 0.01,
                min_dialogue_duration_ms: 500,
                dialogue_merge_gap_ms: 500,
                enable_dialogue_detection: true,
                auto_detect_sample_rate: true,
            },
            general: GeneralConfig {
                backup_enabled: false,
                max_concurrent_jobs: 4,
                task_timeout_seconds: 3600,
                enable_progress_bar: true,
                worker_idle_timeout_seconds: 300,
            },
            parallel: ParallelConfig::default(),
            loaded_from: None,
        }
    }
}

impl Config {
    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Config::config_file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml = toml::to_string_pretty(self)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    /// Get configuration file path
    pub fn config_file_path() -> Result<PathBuf> {
        debug!("config_file_path: Checking SUBX_CONFIG_PATH environment variable");
        if let Ok(custom) = std::env::var("SUBX_CONFIG_PATH") {
            debug!("config_file_path: Using custom path from env: {}", custom);
            let path = PathBuf::from(custom);
            debug!("config_file_path: Custom path exists: {}", path.exists());
            return Ok(path);
        }
        debug!("config_file_path: SUBX_CONFIG_PATH not set, using default");
        let dir = dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?;
        let default_path = dir.join("subx").join("config.toml");
        debug!("config_file_path: Default path: {:?}", default_path);
        Ok(default_path)
    }

    #[allow(dead_code)]
    fn apply_env_vars(&mut self) {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.ai.api_key = Some(key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            self.ai.model = model;
        }
    }

    #[allow(dead_code)]
    fn validate(&self) -> Result<()> {
        if self.ai.provider != "openai" {
            return Err(SubXError::config(format!(
                "Unsupported AI provider: {}",
                self.ai.provider
            )));
        }
        Ok(())
    }

    /// Get value by key (simple version)
    pub fn get_value(&self, key: &str) -> Result<String> {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(SubXError::config(format!(
                "Invalid config key format: {}",
                key
            )));
        }
        match parts[0] {
            "ai" => match parts[1] {
                "provider" => Ok(self.ai.provider.clone()),
                "api_key" => Ok(self.ai.api_key.clone().unwrap_or_default()),
                "model" => Ok(self.ai.model.clone()),
                "base_url" => Ok(self.ai.base_url.clone()),
                _ => Err(SubXError::config(format!("Invalid AI config key: {}", key))),
            },
            "formats" => match parts[1] {
                "default_output" => Ok(self.formats.default_output.clone()),
                _ => Err(SubXError::config(format!(
                    "Invalid Formats config key: {}",
                    key
                ))),
            },
            _ => Err(SubXError::config(format!(
                "Invalid config section: {}",
                parts[0]
            ))),
        }
    }

    #[allow(dead_code)]
    fn merge(&mut self, other: Config) {
        *self = other;
    }
}
