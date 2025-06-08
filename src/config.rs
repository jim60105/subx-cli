//! SubX 配置管理模組
//! Backlog #03: 配置管理系統實作

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{error::SubXError, Result};

// Submodules for unified configuration management core
pub mod manager;
pub mod partial;
pub mod source;

/// 應用程式配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig,
    pub sync: SyncConfig,
    pub general: GeneralConfig,
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
    fn test_old_config_file_still_works() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
max_sample_length = 2000
api_key = "dummy-key"
temperature = 0.5
retry_attempts = 2
retry_delay_ms = 100

[formats]
default_output = "srt"
preserve_styling = true
default_encoding = "utf-8"

[sync]
max_offset_seconds = 10.0
audio_sample_rate = 16000
correlation_threshold = 0.5
dialogue_detection_threshold = 0.02
min_dialogue_duration_ms = 1000

[general]
backup_enabled = true
default_confidence = 80
log_level = "debug"
max_concurrent_jobs = 4
"#;
        std::fs::write(&config_path, config_content).unwrap();
        std::env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        let config = Config::load().unwrap();
        assert_eq!(config.general.max_concurrent_jobs, 4);
        assert_eq!(config.formats.default_encoding, "utf-8");
    }
}

/// AI 相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
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
}

/// 音訊同步相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub audio_sample_rate: u32,
    pub correlation_threshold: f32,
    pub dialogue_detection_threshold: f32,
    pub min_dialogue_duration_ms: u64,
}

/// 一般配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    pub max_concurrent_jobs: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ai: AIConfig {
                provider: "openai".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                max_sample_length: 2000,
                temperature: 0.3,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
            formats: FormatsConfig {
                default_output: "srt".to_string(),
                preserve_styling: true,
                default_encoding: "utf-8".to_string(),
            },
            sync: SyncConfig {
                max_offset_seconds: 30.0,
                audio_sample_rate: 16000,
                correlation_threshold: 0.7,
                dialogue_detection_threshold: 0.01,
                min_dialogue_duration_ms: 500,
            },
            general: GeneralConfig {
                backup_enabled: false,
                max_concurrent_jobs: 4,
            },
            loaded_from: None,
        }
    }
}

impl Config {
    /// 載入配置（環境變數 > 配置檔案 > 預設值）
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        let mut loaded_from_path: Option<PathBuf> = None;

        // 從配置檔案載入
        if let Ok(path) = Config::config_file_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let file_config: Config = toml::from_str(&content)
                    .map_err(|e| SubXError::config(format!("TOML 解析錯誤: {}", e)))?;
                config.merge(file_config);
                loaded_from_path = Some(path);
            }
        }

        // 套用環境變數覆蓋
        config.apply_env_vars();
        config.loaded_from = loaded_from_path;

        // 驗證配置
        config.validate()?;
        Ok(config)
    }

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

    fn apply_env_vars(&mut self) {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.ai.api_key = Some(key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            self.ai.model = model;
        }
    }

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
                _ => Err(SubXError::config(format!("無效的 AI 配置鍵: {}", key))),
            },
            "formats" => match parts[1] {
                "default_output" => Ok(self.formats.default_output.clone()),
                _ => Err(SubXError::config(format!("無效的 Formats 配置鍵: {}", key))),
            },
            _ => Err(SubXError::config(format!("無效的配置區段: {}", parts[0]))),
        }
    }

    fn merge(&mut self, other: Config) {
        *self = other;
    }
}
