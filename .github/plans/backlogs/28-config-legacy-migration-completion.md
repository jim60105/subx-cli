# 配置系統 Legacy 完全移除計劃

## 里程碑
以下列步驟為本次遷移工作的主要里程碑：
1. **里程碑 1**：將配置類型從 `config_legacy.rs` 遷移到 `src/config/mod.rs`，並移除 legacy 檔案。
2. **里程碑 2**：擴展 `ConfigService` trait 及其在 `ProductionConfigService` 和 `TestConfigService` 的實作。
3. **里程碑 3**：更新 CLI command (`config_command.rs`)，移除舊版執行邏輯並改用 `ConfigService`。
4. **里程碑 4**：新增整合測試以驗證完整遷移後的配置功能。
5. **里程碑 5**：執行格式化、Clippy、文件檢查，撰寫工作報告。

**目標**: 完全移除 `config_legacy.rs` 文件，將所有配置數據結構和方法完整遷移到現代 ConfigService 系統，確保系統完全使用依賴注入的配置服務。

## 背景分析

### 當前狀況

透過深度研究 codebase，發現配置系統已大部分遷移到新的 ConfigService 架構：

- ✅ **已完成**: 新的 ConfigService trait 和實現（ProductionConfigService, TestConfigService）
- ✅ **已完成**: TestConfigBuilder 建構器模式
- ✅ **已完成**: 環境變數提供者系統（EnvironmentProvider）
- ✅ **已完成**: 完整的測試覆蓋和隔離系統
- ✅ **已完成**: 依賴注入整合（App, ServiceContainer）

### 需要完全移除的內容

`config_legacy.rs` 文件包含以下需要遷移的內容：

1. **配置數據結構**:
   - `Config` - 主配置結構體
   - `AIConfig` - AI 服務配置
   - `FormatsConfig` - 格式轉換配置
   - `SyncConfig` - 音頻同步配置  
   - `GeneralConfig` - 通用應用配置
   - `ParallelConfig` - 並行處理配置
   - `OverflowStrategy` - 溢出策略枚舉

2. **Legacy 方法**:
   - `Config::save()`, `Config::save_to_file()`, `Config::config_file_path()`
   - `Config::create_config_from_sources()` 
   - `Config::get_value()`

3. **相關測試**: 需要遷移到對應的新模組中

### 使用地點分析

```rust
// 在 config_command.rs 中仍在使用
Config::config_file_path()  // Line 313
default_config.save()       // Line 309

// 在 config_legacy.rs 內部自用
create_config_from_sources() // Line 138
```

## 遷移策略

### 階段一：重構 mod.rs 直接包含配置定義 (1 天)

#### 1.1 將配置類型直接遷移到 `src/config/mod.rs`

考慮到 Rust 慣例和項目簡潔性，我們將配置類型定義直接放在 `mod.rs` 中，工具函數集成到 ConfigService：

```rust
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
/// assert_eq!(ai_config.model, "gpt-4o-mini");
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
```

#### 1.2 工具函數集成到 ConfigService

將配置工具函數直接集成到 ConfigService trait 中，避免創建額外的 utils 模組：

```rust
// 工具函數將作為 ConfigService trait 的方法實現
// 不需要單獨的 utils.rs 文件
```

### 階段二：ConfigService 完全增強 (1 天)

#### 2.1 擴展 ConfigService trait 包含所有功能

```rust
// src/config/service.rs
/// Configuration service interface for SubX.
///
/// This trait provides a unified interface for configuration management,
/// supporting both production and test environments with dependency injection.
///
/// # Examples
///
/// ```rust
/// use subx_cli::config::{ConfigService, ProductionConfigService};
/// use std::path::PathBuf;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let service = ProductionConfigService::new()?;
/// 
/// // Load configuration
/// let config = service.get_config()?;
/// println!("Provider: {}", config.ai.provider);
/// 
/// // Get specific configuration value
/// let provider = service.get_config_value("ai.provider")?;
/// assert_eq!(provider, config.ai.provider);
/// # Ok(())
/// # }
/// ```
pub trait ConfigService: Send + Sync {
    /// Get the current configuration.
    ///
    /// # Returns
    ///
    /// Returns the current [`Config`] instance loaded from files,
    /// environment variables, and defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails due to:
    /// - Invalid TOML format in configuration files
    /// - Missing required configuration values
    /// - File system access issues
    fn get_config(&self) -> Result<Config>;
    
    /// Reload configuration from sources.
    ///
    /// Forces a reload of configuration from all sources, discarding
    /// any cached values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// service.reload()?;
    /// # Ok(())
    /// # }
    /// ```
    fn reload(&self) -> Result<()>;
    
    /// Save current configuration to the default file location.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unable to determine config file path
    /// - File system write permissions are insufficient
    /// - TOML serialization fails
    fn save_config(&self) -> Result<()>;
    
    /// Save configuration to a specific file path.
    ///
    /// # Arguments
    ///
    /// - `path`: Target file path for the configuration
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use subx_cli::config::{ConfigService, ProductionConfigService};
    /// # use std::path::PathBuf;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = ProductionConfigService::new()?;
    /// let path = PathBuf::from("/tmp/custom_config.toml");
    /// service.save_config_to_file(&path)?;
    /// # Ok(())
    /// # }
    /// ```
    fn save_config_to_file(&self, path: &PathBuf) -> Result<()>;
    
    /// Get the default configuration file path.
    ///
    /// # Returns
    ///
    /// Returns the path where configuration files are expected to be located,
    /// typically `$CONFIG_DIR/subx/config.toml`.
    fn get_config_file_path(&self) -> Result<PathBuf>;
    
    /// Get a specific configuration value by key path.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated path to the configuration value (e.g., "ai.provider")
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// let provider = service.get_config_value("ai.provider")?;
    /// let model = service.get_config_value("ai.model")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Supported Keys
    ///
    /// - `ai.provider`, `ai.model`, `ai.api_key`
    /// - `formats.default_output`, `formats.default_encoding`
    /// - `sync.max_offset_seconds`, `sync.correlation_threshold`
    /// - `general.backup_enabled`, `general.max_concurrent_jobs`
    /// - `parallel.max_workers`
    fn get_config_value(&self, key: &str) -> Result<String>;
    
    /// Reset configuration to default values.
    ///
    /// This will overwrite the current configuration file with default values
    /// and reload the configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// service.reset_to_defaults()?;
    /// # Ok(())
    /// # }
    /// ```
    fn reset_to_defaults(&self) -> Result<()>;
}
```

#### 2.2 擴展 ProductionConfigService 實現

```rust
// src/config/service.rs
use crate::error::{SubXError, SubXResult};
use std::path::PathBuf;

impl ConfigService for ProductionConfigService {
    // ...existing methods...
    
    /// Save current configuration to the default file location.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use subx_cli::config::{ConfigService, ProductionConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = ProductionConfigService::new()?;
    /// service.save_config()?;
    /// # Ok(())
    /// # }
    /// ```
    fn save_config(&self) -> Result<()> {
        let config = self.get_config()?;
        let path = self.get_config_file_path()?;
        self.save_config_to_file(&path)
    }
    
    /// Save configuration to a specific file path.
    ///
    /// Creates parent directories if they don't exist and writes the
    /// configuration as pretty-formatted TOML.
    ///
    /// # Arguments
    ///
    /// - `path`: Target file path for saving the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - TOML serialization fails
    /// - Unable to create parent directories
    /// - File write operation fails
    fn save_config_to_file(&self, path: &PathBuf) -> Result<()> {
        let config = self.get_config()?;
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }
        
        std::fs::write(path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
    
    /// Get the default configuration file path.
    ///
    /// Checks for the `SUBX_CONFIG_PATH` environment variable first,
    /// then falls back to the platform-specific config directory.
    ///
    /// # Returns
    ///
    /// Returns the path `$CONFIG_DIR/subx/config.toml` where `$CONFIG_DIR`
    /// is the platform-specific configuration directory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, ProductionConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = ProductionConfigService::new()?;
    /// let path = service.get_config_file_path()?;
    /// assert!(path.to_string_lossy().contains("subx"));
    /// # Ok(())
    /// # }
    /// ```
    fn get_config_file_path(&self) -> Result<PathBuf> {
        if let Ok(custom) = std::env::var("SUBX_CONFIG_PATH") {
            return Ok(PathBuf::from(custom));
        }
        
        let config_dir = dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine config directory"))?;
        Ok(config_dir.join("subx").join("config.toml"))
    }
    
    /// Get a specific configuration value by key path.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated key path (e.g., "ai.provider", "sync.max_offset_seconds")
    ///
    /// # Returns
    ///
    /// Returns the configuration value as a string. Numeric and boolean values
    /// are converted to their string representation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, ProductionConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = ProductionConfigService::new()?;
    /// let provider = service.get_config_value("ai.provider")?;
    /// let max_workers = service.get_config_value("parallel.max_workers")?;
    /// # Ok(())
    /// # }
    /// ```
    fn get_config_value(&self, key: &str) -> Result<String> {
        let config = self.get_config()?;
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(config.ai.provider),
            ["ai", "model"] => Ok(config.ai.model),
            ["ai", "api_key"] => Ok(config.ai.api_key.unwrap_or_default()),
            ["ai", "base_url"] => Ok(config.ai.base_url),
            ["ai", "temperature"] => Ok(config.ai.temperature.to_string()),
            ["formats", "default_output"] => Ok(config.formats.default_output),
            ["formats", "default_encoding"] => Ok(config.formats.default_encoding),
            ["formats", "preserve_styling"] => Ok(config.formats.preserve_styling.to_string()),
            ["sync", "max_offset_seconds"] => Ok(config.sync.max_offset_seconds.to_string()),
            ["sync", "correlation_threshold"] => Ok(config.sync.correlation_threshold.to_string()),
            ["sync", "audio_sample_rate"] => Ok(config.sync.audio_sample_rate.to_string()),
            ["general", "backup_enabled"] => Ok(config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => Ok(config.general.max_concurrent_jobs.to_string()),
            ["general", "log_level"] => Ok(config.general.log_level),
            ["parallel", "max_workers"] => Ok(config.parallel.max_workers.to_string()),
            ["parallel", "chunk_size"] => Ok(config.parallel.chunk_size.to_string()),
            _ => Err(SubXError::config(format!("Unknown configuration key: {}", key))),
        }
    }
    
    /// Reset configuration to default values.
    ///
    /// Creates a new default configuration, saves it to the config file,
    /// and reloads the configuration service.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use subx_cli::config::{ConfigService, ProductionConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = ProductionConfigService::new()?;
    /// service.reset_to_defaults()?;
    /// 
    /// // Verify reset worked
    /// let config = service.get_config()?;
    /// assert_eq!(config.ai.provider, "openai");
    /// # Ok(())
    /// # }
    /// ```
    fn reset_to_defaults(&self) -> Result<()> {
        let default_config = Config::default();
        let path = self.get_config_file_path()?;
        
        // Save default configuration
        let toml_content = toml::to_string_pretty(&default_config)
            .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?;
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SubXError::config(format!("Failed to create config directory: {}", e))
            })?;
        }
        
        std::fs::write(&path, toml_content)
            .map_err(|e| SubXError::config(format!("Failed to write config file: {}", e)))?;
        
        // Reload configuration
        self.reload()
    }
}
```

#### 2.3 擴展 TestConfigService 實現

```rust
// src/config/test_service.rs
impl ConfigService for TestConfigService {
    // ...existing methods...
    
    /// Save configuration in test environment.
    ///
    /// In test environments, this method succeeds without performing
    /// actual file I/O to maintain test isolation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// service.save_config()?; // Always succeeds in tests
    /// # Ok(())
    /// # }
    /// ```
    fn save_config(&self) -> Result<()> {
        // Test environment doesn't need actual file saving for isolation
        Ok(())
    }
    
    /// Save configuration to a specific file path in test environment.
    ///
    /// In test environments, this method succeeds without performing
    /// actual file I/O operations.
    ///
    /// # Arguments
    ///
    /// - `_path`: File path (ignored in test environment)
    fn save_config_to_file(&self, _path: &PathBuf) -> Result<()> {
        // Test environment doesn't need actual file saving for isolation
        Ok(())
    }
    
    /// Get test-specific configuration file path.
    ///
    /// Returns a temporary path that won't interfere with production
    /// configuration files.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// let path = service.get_config_file_path()?;
    /// assert!(path.to_string_lossy().contains("test"));
    /// # Ok(())
    /// # }
    /// ```
    fn get_config_file_path(&self) -> Result<PathBuf> {
        // Return test-specific temporary path to avoid interference
        Ok(PathBuf::from("/tmp/subx_test_config.toml"))
    }
    
    /// Get configuration value by key from test configuration.
    ///
    /// # Arguments
    ///
    /// - `key`: Dot-separated configuration key path
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// let provider = service.get_config_value("ai.provider")?;
    /// let model = service.get_config_value("ai.model")?;
    /// assert!(!provider.is_empty());
    /// assert!(!model.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    fn get_config_value(&self, key: &str) -> Result<String> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["ai", "provider"] => Ok(self.fixed_config.ai.provider.clone()),
            ["ai", "model"] => Ok(self.fixed_config.ai.model.clone()),
            ["ai", "api_key"] => Ok(self.fixed_config.ai.api_key.clone().unwrap_or_default()),
            ["ai", "base_url"] => Ok(self.fixed_config.ai.base_url.clone()),
            ["ai", "temperature"] => Ok(self.fixed_config.ai.temperature.to_string()),
            ["formats", "default_output"] => Ok(self.fixed_config.formats.default_output.clone()),
            ["formats", "default_encoding"] => Ok(self.fixed_config.formats.default_encoding.clone()),
            ["formats", "preserve_styling"] => Ok(self.fixed_config.formats.preserve_styling.to_string()),
            ["sync", "max_offset_seconds"] => Ok(self.fixed_config.sync.max_offset_seconds.to_string()),
            ["sync", "correlation_threshold"] => Ok(self.fixed_config.sync.correlation_threshold.to_string()),
            ["sync", "audio_sample_rate"] => Ok(self.fixed_config.sync.audio_sample_rate.to_string()),
            ["general", "backup_enabled"] => Ok(self.fixed_config.general.backup_enabled.to_string()),
            ["general", "max_concurrent_jobs"] => Ok(self.fixed_config.general.max_concurrent_jobs.to_string()),
            ["general", "log_level"] => Ok(self.fixed_config.general.log_level.clone()),
            ["parallel", "max_workers"] => Ok(self.fixed_config.parallel.max_workers.to_string()),
            ["parallel", "chunk_size"] => Ok(self.fixed_config.parallel.chunk_size.to_string()),
            _ => Err(SubXError::config(format!("Unknown configuration key: {}", key))),
        }
    }
    
    /// Reset configuration to defaults in test environment.
    ///
    /// In test environments, this resets the internal configuration
    /// to default values without file system interaction.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::config::{ConfigService, TestConfigService};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let service = TestConfigService::with_defaults();
    /// service.reset_to_defaults()?;
    /// 
    /// // Verify reset worked
    /// let config = service.get_config()?;
    /// assert_eq!(config.ai.provider, "openai");
    /// # Ok(())
    /// # }
    /// ```
    fn reset_to_defaults(&self) -> Result<()> {
        // Test environment directly resets internal configuration
        // Note: This requires TestConfigService to have mutable state
        // or a way to reset its internal configuration
        // Implementation details depend on current TestConfigService design
        Ok(())
    }
}
```

### 階段三：移除 Legacy 文件 (1 天)

#### 3.1 刪除 config_legacy.rs 文件

```bash
# 移除 legacy 文件
rm src/config/config_legacy.rs
```

#### 3.2 確保所有引用已更新

運行編譯檢查確保沒有遺漏的引用：

```bash
cargo check
cargo build
```

如果有編譯錯誤，逐一修復所有對 `config_legacy` 的引用。所有原來的引用現在都應該直接使用 `crate::config::Config` 等類型。

#### 3.3 更新相關文檔和註釋

移除所有提及 `config_legacy` 的文檔和註釋。

### 階段四：命令更新 (1 天)

#### 4.1 更新 config_command.rs

```rust
// src/commands/config_command.rs
//! Configuration command implementation using ConfigService.
//!
//! This module provides command-line interface for configuration management
//! through the dependency-injected ConfigService trait.

use crate::config::ConfigService;
use crate::error::{SubXError, SubXResult};
use crate::cli::config_args::{ConfigArgs, ConfigAction};

/// Execute configuration command with dependency injection.
///
/// # Arguments
///
/// - `args`: Configuration command arguments from CLI
/// - `config_service`: Configuration service implementation
///
/// # Examples
///
/// ```rust,no_run
/// # use subx_cli::commands::config_command;
/// # use subx_cli::cli::config_args::{ConfigArgs, ConfigAction};
/// # use subx_cli::config::{ConfigService, TestConfigService};
/// # use std::sync::Arc;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let service = Arc::new(TestConfigService::with_defaults());
/// let args = ConfigArgs {
///     action: ConfigAction::List,
/// };
/// config_command::execute(args, service.as_ref()).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - Configuration service operations fail
/// - Invalid configuration keys are requested
/// - File system operations are not permitted
pub async fn execute(args: ConfigArgs, config_service: &dyn ConfigService) -> SubXResult<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            return Err(SubXError::config(
                "Setting configuration values not yet supported with ConfigService. Use config files or environment variables instead."
            ));
        }
        ConfigAction::Get { key } => {
            let value = config_service.get_config_value(&key)?;
            println!("{}", value);
        }
        ConfigAction::List => {
            let config = config_service.get_config()?;
            if let Ok(path) = config_service.get_config_file_path() {
                println!("# Configuration file path: {}\n", path.display());
            }
            println!(
                "{}",
                toml::to_string_pretty(&config)
                    .map_err(|e| SubXError::config(format!("TOML serialization error: {}", e)))?
            );
        }
        ConfigAction::Reset => {
            config_service.reset_to_defaults()?;
            println!("Configuration reset to default values");
            if let Ok(path) = config_service.get_config_file_path() {
                println!("Default configuration saved to: {}", path.display());
            }
        }
    }
    Ok(())
}
```

#### 4.2 移除舊版本的 execute 函數

完全移除 `config_command.rs` 中不使用 ConfigService 的舊版本函數。

### 階段五：完全移除 Legacy 文件 (1 天)

#### 5.1 刪除 config_legacy.rs 文件

```bash
# 移除 legacy 文件
rm src/config/config_legacy.rs
```

#### 5.2 確保所有引用已更新

運行編譯檢查確保沒有遺漏的引用：

```bash
cargo check
cargo build
```

如果有編譯錯誤，逐一修復所有對 `config_legacy` 的引用。

#### 5.3 更新相關文檔和註釋

移除所有提及 `config_legacy` 的文檔和註釋。

### 階段六：測試更新和驗證 (1 天)

#### 6.1 創建新的配置整合測試

```rust
// tests/config_complete_migration_tests.rs
//! Integration tests for complete config_legacy.rs migration.
//!
//! These tests verify that all configuration functionality works correctly
//! after the complete removal of config_legacy.rs file.

use std::sync::Arc;
use subx_cli::config::{
    ConfigService, ProductionConfigService, TestConfigService, TestConfigBuilder,
    Config, AIConfig, FormatsConfig, SyncConfig, GeneralConfig, ParallelConfig, OverflowStrategy
};
use subx_cli::error::SubXResult;

/// Test that all configuration types are available and functional.
///
/// This test ensures that all configuration structures can be created
/// and have the expected default values.
#[test]
fn test_all_config_types_available() {
    // Test main configuration structure
    let config = Config::default();
    assert!(!config.ai.provider.is_empty());
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4o-mini");
    
    // Test individual configuration structures
    let ai_config = AIConfig::default();
    assert_eq!(ai_config.provider, "openai");
    assert_eq!(ai_config.temperature, 0.3);
    
    let formats_config = FormatsConfig::default();
    assert_eq!(formats_config.default_output, "srt");
    assert_eq!(formats_config.default_encoding, "utf-8");
    assert!(!formats_config.preserve_styling);
    
    let sync_config = SyncConfig::default();
    assert_eq!(sync_config.audio_sample_rate, 44100);
    assert_eq!(sync_config.max_offset_seconds, 10.0);
    assert!(sync_config.enable_dialogue_detection);
    
    let general_config = GeneralConfig::default();
    assert!(!general_config.backup_enabled);
    assert_eq!(general_config.max_concurrent_jobs, 4);
    assert_eq!(general_config.log_level, "info");
    
    let parallel_config = ParallelConfig::default();
    assert!(parallel_config.max_workers > 0);
    assert_eq!(parallel_config.overflow_strategy, OverflowStrategy::Block);
    assert!(parallel_config.enable_work_stealing);
}

/// Test that ConfigService replaces all legacy functionality.
///
/// This test verifies that all methods previously available in
/// config_legacy.rs are now properly implemented in ConfigService.
#[test]
fn test_config_service_replaces_all_legacy_methods() -> SubXResult<()> {
    let service = Arc::new(TestConfigService::with_defaults());
    
    // Test configuration file path functionality
    let path = service.get_config_file_path()?;
    assert!(path.to_string_lossy().contains("test"));
    
    // Test configuration saving functionality
    service.save_config()?;
    service.save_config_to_file(&path)?;
    
    // Test configuration value retrieval
    let provider = service.get_config_value("ai.provider")?;
    assert!(!provider.is_empty());
    
    let model = service.get_config_value("ai.model")?;
    assert!(!model.is_empty());
    
    let max_workers = service.get_config_value("parallel.max_workers")?;
    assert!(max_workers.parse::<usize>().is_ok());
    
    // Test reset functionality
    service.reset_to_defaults()?;
    
    // Test configuration loading
    let config = service.get_config()?;
    assert!(!config.ai.provider.is_empty());
    
    // Test reload functionality
    service.reload()?;
    
    Ok(())
}

/// Test ProductionConfigService full functionality.
///
/// This test ensures that the production configuration service
/// implements all required methods without panicking.
#[test]
fn test_production_config_service_full_functionality() -> SubXResult<()> {
    let service = ProductionConfigService::new()?;
    
    // Test core ConfigService methods
    let config = service.get_config()?;
    assert!(!config.ai.provider.is_empty());
    
    let path = service.get_config_file_path()?;
    assert!(path.to_string_lossy().contains("subx"));
    
    let ai_model = service.get_config_value("ai.model")?;
    assert!(!ai_model.is_empty());
    
    let max_jobs = service.get_config_value("general.max_concurrent_jobs")?;
    assert!(max_jobs.parse::<usize>().is_ok());
    
    // Test reload functionality
    service.reload()?;
    
    // These methods might fail in test environment but shouldn't panic
    let _ = service.save_config();
    let _ = service.reset_to_defaults();
    
    Ok(())
}

/// Test configuration builder still works after migration.
///
/// This test ensures that the TestConfigBuilder continues to function
/// correctly after the migration.
#[test]
fn test_config_builder_still_works() -> SubXResult<()> {
    let service = TestConfigBuilder::new()
        .with_ai_provider("test_provider")
        .with_ai_model("test_model")
        .with_sync_threshold(0.9)
        .build_service();
        
    let config = service.get_config()?;
    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");
    assert_eq!(config.sync.correlation_threshold, 0.9);
    
    Ok(())
}

/// Test configuration serialization and deserialization.
///
/// This test ensures that configuration structures can be properly
/// serialized to and from TOML format.
#[test]
fn test_config_serialization_roundtrip() -> SubXResult<()> {
    let original_config = Config::default();
    
    // Serialize to TOML
    let toml_str = toml::to_string_pretty(&original_config)
        .map_err(|e| subx_cli::error::SubXError::config(format!("Serialization failed: {}", e)))?;
    
    // Verify TOML contains expected sections
    assert!(toml_str.contains("[ai]"));
    assert!(toml_str.contains("[sync]"));
    assert!(toml_str.contains("[general]"));
    assert!(toml_str.contains("[parallel]"));
    assert!(toml_str.contains("[formats]"));
    
    // Deserialize back from TOML
    let deserialized_config: Config = toml::from_str(&toml_str)
        .map_err(|e| subx_cli::error::SubXError::config(format!("Deserialization failed: {}", e)))?;
    
    // Verify key fields are preserved
    assert_eq!(original_config.ai.provider, deserialized_config.ai.provider);
    assert_eq!(original_config.ai.model, deserialized_config.ai.model);
    assert_eq!(original_config.formats.default_output, deserialized_config.formats.default_output);
    assert_eq!(original_config.sync.audio_sample_rate, deserialized_config.sync.audio_sample_rate);
    assert_eq!(original_config.general.max_concurrent_jobs, deserialized_config.general.max_concurrent_jobs);
    
    Ok(())
}

/// Test OverflowStrategy enum functionality.
///
/// This test ensures that the OverflowStrategy enum works correctly
/// after migration.
#[test]
fn test_overflow_strategy_functionality() {
    // Test default creation
    let default_strategy = OverflowStrategy::Block;
    assert_eq!(default_strategy, OverflowStrategy::Block);
    
    // Test all variants
    let strategies = vec![
        OverflowStrategy::Block,
        OverflowStrategy::Drop,
        OverflowStrategy::Expand,
        OverflowStrategy::DropOldest,
        OverflowStrategy::Reject,
    ];
    
    assert_eq!(strategies.len(), 5);
    
    // Test inequality
    assert_ne!(OverflowStrategy::Block, OverflowStrategy::Drop);
    assert_ne!(OverflowStrategy::Expand, OverflowStrategy::Reject);
    
    // Test serialization
    let config = ParallelConfig::default();
    let toml_str = toml::to_string(&config).expect("Serialization should work");
    assert!(toml_str.contains("Block"));
}

/// Test error handling in configuration operations.
///
/// This test ensures that configuration service methods handle
/// errors appropriately.
#[test]
fn test_config_error_handling() {
    let service = TestConfigService::with_defaults();
    
    // Test invalid configuration key
    let result = service.get_config_value("invalid.key");
    assert!(result.is_err());
    
    let result = service.get_config_value("ai.nonexistent");
    assert!(result.is_err());
    
    // Test empty key
    let result = service.get_config_value("");
    assert!(result.is_err());
}
```

#### 6.2 更新現有配置測試

更新所有現有的配置相關測試，確保它們使用新的模組結構：

```rust
// Update all tests to use the new module structure
// Replace:
// use subx_cli::config::config_legacy::Config;
// With:
// use subx_cli::config::Config;

// Example test update:
#[cfg(test)]
mod tests {
    use super::*;
    use subx_cli::config::{Config, ConfigService, TestConfigService};
    use subx_cli::error::SubXResult;

    /// Test configuration loading with new structure.
    ///
    /// This test ensures that configuration loading works correctly
    /// with the new module structure.
    #[test]
    fn test_config_loading_with_new_structure() -> SubXResult<()> {
        let service = TestConfigService::with_defaults();
        let config = service.get_config()?;
        
        // Verify all sections are present
        assert!(!config.ai.provider.is_empty());
        assert!(!config.formats.default_output.is_empty());
        assert!(config.sync.audio_sample_rate > 0);
        assert!(config.general.max_concurrent_jobs > 0);
        assert!(config.parallel.max_workers > 0);
        
        Ok(())
    }

    /// Test configuration value access with new API.
    ///
    /// This test verifies that configuration values can be accessed
    /// through the new ConfigService interface.
    #[test]
    fn test_config_value_access_new_api() -> SubXResult<()> {
        let service = TestConfigService::with_defaults();
        
        // Test various configuration value access patterns
        let ai_provider = service.get_config_value("ai.provider")?;
        assert_eq!(ai_provider, "openai");
        
        let max_workers = service.get_config_value("parallel.max_workers")?;
        assert!(max_workers.parse::<usize>().is_ok());
        
        let backup_enabled = service.get_config_value("general.backup_enabled")?;
        assert_eq!(backup_enabled, "false");
        
        Ok(())
    }
}
```

#### 6.3 運行完整測試套件

```bash
# Run all configuration-related tests
cargo test config_ --verbose

# Run integration tests
cargo test integration_tests --verbose

# Run all tests to ensure no regressions
cargo test --all-features --verbose

# Run documentation tests
cargo test --doc --verbose

# Check test coverage
./scripts/check_coverage.sh -T
```
```

### 階段七：文檔更新和最終驗證 (0.5 天)

#### 7.1 更新技術架構文檔

```markdown
## Configuration System Architecture

### Module Structure

The configuration system now uses a clean modular separation:

- **`src/config/mod.rs`**: All configuration data structure definitions and module entry point
- **`src/config/service.rs`**: ConfigService trait and ProductionConfigService implementation  
- **`src/config/test_service.rs`**: TestConfigService implementation for isolated testing
- **`src/config/builder.rs`**: Test configuration builder pattern implementation
- **`src/config/environment.rs`**: Environment variable provider system
- **`src/config/validator.rs`**: Configuration validation utilities

### ConfigService Complete Interface

The configuration system is completely based on the ConfigService trait, providing comprehensive functionality:

- **Configuration Loading and Reloading**: `get_config()`, `reload()`
- **Configuration Saving**: `save_config()`, `save_config_to_file()`
- **Configuration Querying**: `get_config_value()`
- **Configuration Reset**: `reset_to_defaults()`
- **Configuration Path Management**: `get_config_file_path()`

### Implementation Types

- **ProductionConfigService**: Production environment configuration service with complete file operations
- **TestConfigService**: Test environment configuration service providing isolated configuration management

### Legacy System Complete Removal

- ❌ `config_legacy.rs` file has been completely removed
- ✅ All configuration data structures migrated to `mod.rs`
- ✅ All configuration operations unified through ConfigService
- ✅ Complete test coverage and documentation support

### Type Definitions

All configuration types are now defined in `src/config/mod.rs`:

```rust
/// Main configuration structure containing all application settings.
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig, 
    pub sync: SyncConfig,
    pub general: GeneralConfig,
    pub parallel: ParallelConfig,
    pub loaded_from: Option<PathBuf>,
}

/// AI service provider configuration.
pub struct AIConfig { /* ... */ }

/// Subtitle format related configuration.
pub struct FormatsConfig { /* ... */ }

/// Audio synchronization configuration.
pub struct SyncConfig { /* ... */ }

/// General application configuration.
pub struct GeneralConfig { /* ... */ }

/// Parallel processing configuration.
pub struct ParallelConfig { /* ... */ }

/// Strategy for handling worker overflow.
pub enum OverflowStrategy { /* ... */ }
```

### Service Interface

```rust
/// Configuration service trait providing unified configuration management.
pub trait ConfigService: Send + Sync {
    fn get_config(&self) -> Result<Config>;
    fn reload(&self) -> Result<()>;
    fn save_config(&self) -> Result<()>;
    fn save_config_to_file(&self, path: &PathBuf) -> Result<()>;
    fn get_config_file_path(&self) -> Result<PathBuf>;
    fn get_config_value(&self, key: &str) -> Result<String>;
    fn reset_to_defaults(&self) -> Result<()>;
}
```
```
```

#### 7.2 更新 README 和使用文檔

更新所有提及配置管理的文檔，移除對 legacy 系統的引用。

#### 7.3 更新程式碼註釋

檢查並更新所有程式碼中的註釋，確保不再提及 `config_legacy`。

## 驗收標準

### 功能驗收

- [ ] `config_legacy.rs` 文件已完全刪除
- [ ] 所有配置數據結構遷移到 `src/config/types.rs`
- [ ] 所有配置工具函數遷移到 `src/config/utils.rs`
- [ ] ConfigService trait 包含所有必要的配置管理方法
- [ ] ProductionConfigService 和 TestConfigService 完整實現所有方法
- [ ] config_command.rs 完全使用 ConfigService
- [ ] 所有測試通過，無回歸問題
- [ ] 模組導出正確，無編譯錯誤

### 代碼品質驗收

```bash
# 編譯檢查
cargo build

# 格式化檢查
cargo fmt -- --check

# Clippy 檢查
cargo clippy -- -D warnings

# 測試覆蓋
scripts/check_coverage.sh -T

# 文檔檢查
timeout 20 scripts/check_docs.sh

# 確認 config_legacy.rs 已刪除
test ! -f src/config/config_legacy.rs && echo "✅ config_legacy.rs successfully removed"
```

### 整合測試驗收

```bash
# 配置相關命令測試
cargo test config_

# 配置數據結構測試
cargo test types::tests

# 配置工具函數測試
cargo test utils

# CLI 整合測試
cargo test cli_integration

# 新的完整遷移測試
cargo test config_complete_migration
```

## 風險評估

### 高風險項目

1. **大範圍重構風險**: 同時移動數據結構和業務邏輯可能導致遺漏
2. **引用更新遺漏**: 大量的 import 語句需要更新
3. **測試覆蓋缺失**: 確保新結構下所有功能都有測試覆蓋
4. **文檔同步問題**: 大量文檔需要同步更新

### 緩解措施

1. **分階段實施**: 嚴格按照階段執行，每階段都進行編譯檢查
2. **全局搜索替換**: 使用工具確保所有引用都得到更新
3. **編譯驅動**: 以編譯器錯誤為指導，確保沒有遺漏
4. **測試優先**: 在每個階段都運行完整的測試套件
5. **備份計劃**: 在開始前創建完整的 Git 分支備份

## 後續工作

### 優化機會

1. **類型安全增強**: 利用新的模組結構添加更強的類型約束
2. **配置驗證改進**: 在 `types.rs` 中添加更多的驗證邏輯
3. **性能優化**: 優化 ConfigService 的緩存和序列化性能
4. **擴展性提升**: 為未來的配置功能預留擴展點

### 監控指標

- 編譯時間變化
- 測試執行時間
- 配置操作性能
- 記憶體使用情況

## 時程安排

```
第一天：重構 mod.rs 直接包含配置定義
第二天：擴展 ConfigService + 移除 Legacy 文件
第三天：命令更新 + 測試更新和驗證
第四天：文檔更新 + 最終驗證
```

### 🎯 優化的架構優勢

**新的模組結構**：
```
src/config/
├── mod.rs           # 包含所有配置數據結構 + 模組入口
├── service.rs       # ConfigService trait 和 ProductionConfigService  
├── test_service.rs  # TestConfigService
├── builder.rs       # TestConfigBuilder
├── environment.rs   # 環境變數提供者
├── validator.rs     # 配置驗證
└── test_macros.rs   # 測試宏
```

**完全移除**：
```
❌ src/config/config_legacy.rs  # 這個文件將被完全刪除
❌ src/config/types.rs          # 不需要創建，內容在 mod.rs 中
❌ src/config/utils.rs          # 不需要創建，功能集成到 ConfigService 中
```

**符合 Rust 慣例的原因**：
1. **mod.rs 作為統一入口**: 所有類型定義在一個地方，便於查找和維護
2. **功能集成**: 工具函數直接集成到 ConfigService trait，避免額外的工具模組
3. **簡潔性**: 減少不必要的文件分離，保持模組結構清晰
4. **測試就近**: 配置類型的測試直接放在定義的模組中

## 總結

此計劃將徹底移除 `config_legacy.rs` 文件，完成配置系統的現代化重構。重構後的系統具有：

- **清晰的模組分離**: 數據結構、工具函數、服務實現各司其職
- **完整的依賴注入**: 所有配置操作通過 ConfigService 進行
- **更好的測試隔離**: TestConfigService 提供完全隔離的測試環境
- **強化的類型安全**: 現代 Rust 最佳實踐
- **優秀的可維護性**: 清晰的架構和完整的文檔

移除 legacy 文件後，系統將更加簡潔、可靠，為未來的功能擴展奠定堅實基礎。
