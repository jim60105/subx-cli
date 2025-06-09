//! SubX 配置管理模組
//! Backlog #03: 配置管理系統實作

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{error::SubXError, Result};

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

/// 初始化全域配置管理器
pub fn init_config_manager() -> Result<()> {
    let lock = GLOBAL_CONFIG_MANAGER.get_or_init(|| Mutex::new(ConfigManager::new()));

    // 獲取配置檔案路徑（此時環境變數應該已經設定）
    let config_path = Config::config_file_path()?;

    let manager = ConfigManager::new()
        .add_source(Box::new(FileSource::new(config_path)))
        .add_source(Box::new(EnvSource::new()))
        .add_source(Box::new(CliSource::new()));
    manager
        .load()
        .map_err(|e| SubXError::config(e.to_string()))?;
    let mut guard = lock.lock().unwrap();
    *guard = manager;
    Ok(())
}

/// 載入應用程式配置（替代 Config::load()）
pub fn load_config() -> Result<Config> {
    let lock = GLOBAL_CONFIG_MANAGER.get().ok_or_else(|| {
        SubXError::config("配置管理器尚未初始化，請先呼叫 init_config_manager()".to_string())
    })?;
    let manager = lock.lock().unwrap();
    let config_lock = manager.config();
    let partial_config = config_lock.read().unwrap();
    let config = partial_config
        .to_complete_config()
        .map_err(|e| SubXError::config(e.to_string()))?;
    Ok(config)
}

/// 應用程式配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig,
    pub sync: SyncConfig,
    pub general: GeneralConfig,
    pub parallel: ParallelConfig,
    #[serde(skip)]
    pub loaded_from: Option<PathBuf>,
}

// 單元測試: Config 組態管理功能
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
        // 清除環境變數以避免測試間干擾
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("SUBX_AI_MODEL");
        env::set_var("OPENAI_API_KEY", "test-key-123");
        env::set_var("SUBX_AI_MODEL", "gpt-3.5-turbo");

        let mut config = Config::default();
        config.apply_env_vars();
        assert!(config.ai.api_key.is_some());
        assert_eq!(config.ai.model, "gpt-3.5-turbo");

        env::remove_var("OPENAI_API_KEY");
        env::remove_var("SUBX_AI_MODEL");
    }

    #[test]
    #[serial]
    fn test_config_validation_missing_api_key() {
        env::remove_var("OPENAI_API_KEY");
        let config = Config::default();
        // API Key 驗證於執行時進行，不影響載入
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

        std::env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());

        assert!(init_config_manager().is_ok());

        let loaded_config = load_config().unwrap();
        assert_eq!(loaded_config.ai.model, test_config.ai.model);

        std::env::remove_var("SUBX_CONFIG_PATH");
    }

    #[test]
    fn test_env_var_override_with_new_system() {
        std::env::set_var("OPENAI_API_KEY", "test-key-from-env");
        std::env::set_var("SUBX_AI_MODEL", "gpt-4-from-env");

        let _ = init_config_manager();
        let config = load_config().unwrap();

        assert_eq!(config.ai.api_key, Some("test-key-from-env".to_string()));
        assert_eq!(config.ai.model, "gpt-4-from-env");

        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("SUBX_AI_MODEL");
    }
}

/// AI 相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: String,
    pub max_sample_length: usize,
    pub temperature: f32,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

/// 字幕格式相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    pub default_output: String,
    pub preserve_styling: bool,
    pub default_encoding: String,
    /// 編碼檢測信心度閾值（0.0-1.0）
    pub encoding_detection_confidence: f32,
}

/// 音訊同步相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub audio_sample_rate: u32,
    pub correlation_threshold: f32,
    pub dialogue_detection_threshold: f32,
    pub min_dialogue_duration_ms: u64,
    /// 對話片段合併間隔（毫秒）
    pub dialogue_merge_gap_ms: u64,
    /// 是否啟用對話檢測
    pub enable_dialogue_detection: bool,
    /// 重採樣品質（low, medium, high, best）
    pub resample_quality: String,
    /// 是否自動檢測原始採樣率
    pub auto_detect_sample_rate: bool,
    /// 是否啟用智慧重採樣
    pub enable_smart_resampling: bool,
}

impl SyncConfig {
    /// 取得重採樣品質設定
    pub fn resample_quality(&self) -> String {
        self.resample_quality.clone()
    }
    /// 是否啟用自動檢測原始採樣率
    pub fn auto_detect_sample_rate(&self) -> bool {
        self.auto_detect_sample_rate
    }
    /// 是否啟用智慧重採樣
    pub fn enable_smart_resampling(&self) -> bool {
        self.enable_smart_resampling
    }
}

/// 一般配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    pub max_concurrent_jobs: usize,
    pub task_timeout_seconds: u64,
    pub enable_progress_bar: bool,
    pub worker_idle_timeout_seconds: u64,
}

/// 並行處理相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParallelConfig {
    pub cpu_intensive_limit: usize,
    pub io_intensive_limit: usize,
    pub task_queue_size: usize,
    pub enable_task_priorities: bool,
    pub auto_balance_workers: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        ParallelConfig {
            cpu_intensive_limit: 2,
            io_intensive_limit: 8,
            task_queue_size: 100,
            enable_task_priorities: true,
            auto_balance_workers: true,
        }
    }
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
                resample_quality: "high".to_string(),
                auto_detect_sample_rate: true,
                enable_smart_resampling: true,
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
    /// 儲存配置到檔案
    pub fn save(&self) -> Result<()> {
        let path = Config::config_file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml = toml::to_string_pretty(self)
            .map_err(|e| SubXError::config(format!("TOML 序列化錯誤: {}", e)))?;
        std::fs::write(path, toml)?;
        Ok(())
    }

    /// 取得配置檔案路徑
    pub fn config_file_path() -> Result<PathBuf> {
        if let Ok(custom) = std::env::var("SUBX_CONFIG_PATH") {
            return Ok(PathBuf::from(custom));
        }
        let dir = dirs::config_dir().ok_or_else(|| SubXError::config("無法確定配置目錄"))?;
        Ok(dir.join("subx").join("config.toml"))
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
                "不支援的 AI provider: {}",
                self.ai.provider
            )));
        }
        Ok(())
    }

    /// 依鍵名取得值 (簡易版)
    pub fn get_value(&self, key: &str) -> Result<String> {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(SubXError::config(format!("無效的配置鍵格式: {}", key)));
        }
        match parts[0] {
            "ai" => match parts[1] {
                "provider" => Ok(self.ai.provider.clone()),
                "api_key" => Ok(self.ai.api_key.clone().unwrap_or_default()),
                "model" => Ok(self.ai.model.clone()),
                "base_url" => Ok(self.ai.base_url.clone()),
                _ => Err(SubXError::config(format!("無效的 AI 配置鍵: {}", key))),
            },
            "formats" => match parts[1] {
                "default_output" => Ok(self.formats.default_output.clone()),
                _ => Err(SubXError::config(format!("無效的 Formats 配置鍵: {}", key))),
            },
            _ => Err(SubXError::config(format!("無效的配置區段: {}", parts[0]))),
        }
    }

    #[allow(dead_code)]
    fn merge(&mut self, other: Config) {
        *self = other;
    }
}
