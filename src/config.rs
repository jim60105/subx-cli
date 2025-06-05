//! SubX 配置管理模組
//! Backlog #03: 配置管理系統實作

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{error::SubXError, Result};

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
    pub default_confidence: u8,
    pub max_concurrent_jobs: usize,
    pub log_level: String,
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
                default_confidence: 80,
                max_concurrent_jobs: num_cpus::get_physical(),
                log_level: "info".to_string(),
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
