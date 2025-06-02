# Product Backlog #03: 配置管理系統

## 領域範圍
應用程式配置檔案處理、環境變數管理、配置驗證

## 完成項目

### 1. 配置檔案結構設計
- [ ] 定義 TOML 配置檔案格式
- [ ] 設計分層配置結構 (AI、格式、同步設定)
- [ ] 實作配置檔案的讀寫功能
- [ ] 建立配置檔案預設值

### 2. 配置檔案路徑管理
- [ ] Linux/macOS: `~/.config/subx/config.toml`
- [ ] Windows: `%APPDATA%\subx\config.toml`
- [ ] 自動建立配置目錄
- [ ] 支援自訂配置檔案路徑

### 3. 環境變數支援
- [ ] `OPENAI_API_KEY` 環境變數讀取
- [ ] `SUBX_CONFIG_PATH` 自訂配置路徑
- [ ] 環境變數優先權設計
- [ ] 配置來源追蹤

### 4. 配置驗證
- [ ] API 金鑰格式驗證
- [ ] 數值範圍檢查
- [ ] 必要設定檢查
- [ ] 設定衝突檢測

### 5. Config 命令實作
- [ ] `subx config set key value` 設定值
- [ ] `subx config get key` 取得值
- [ ] `subx config list` 列出所有設定
- [ ] `subx config reset` 重置為預設值

### 6. 配置遷移機制
- [ ] 版本相容性檢查
- [ ] 舊版配置自動遷移
- [ ] 備份舊配置檔案

## 技術設計

### 配置結構定義
```rust
// src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub ai: AIConfig,
    pub formats: FormatsConfig,
    pub sync: SyncConfig,
    pub general: GeneralConfig,
    #[serde(skip)] // 不序列化到配置檔案
    pub loaded_from: Option<PathBuf>, // 新增欄位追蹤配置來源
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String, // 考慮使用 enum AIProviderType
    pub api_key: Option<String>,
    pub model: String,
    pub max_sample_length: usize,
    pub temperature: f32,
    pub retry_attempts: u32, // 新增重試次數
    pub retry_delay_ms: u64, // 新增重試延遲
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    pub default_output: String, // 考慮使用 enum OutputSubtitleFormat
    pub preserve_styling: bool,
    pub default_encoding: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub audio_sample_rate: u32,
    pub correlation_threshold: f32,
    pub dialogue_detection_threshold: f32, // 新增對話檢測閾值
    pub min_dialogue_duration_ms: u64, // 新增最小對話持續時間
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    pub default_confidence: u8,
    pub max_concurrent_jobs: usize,
    pub log_level: String, // 新增日誌級別配置
}
```

### 配置管理器
```rust
// src/config.rs
impl Config {
    /// 載入配置（環境變數 > 配置檔案 > 預設值）
    pub fn load() -> crate::Result<Self> {
        let mut config = Self::default();
        let mut loaded_from_path: Option<PathBuf> = None;

        // 1. 載入配置檔案
        if let Ok(config_path) = Self::config_file_path() {
            if config_path.exists() {
                let file_content = std::fs::read_to_string(&config_path)?;
                let file_config: Self = toml::from_str(&file_content)
                    .map_err(|e| crate::SubXError::Config(format!("TOML 解析錯誤: {}", e)))?;
                config.merge(file_config);
                loaded_from_path = Some(config_path);
            } else {
                // 如果配置檔案不存在，可以考慮儲存預設配置
                // config.save()?; // 首次執行時儲存預設配置
            }
        }
        
        // 2. 環境變數覆蓋
        config.apply_env_vars();
        config.loaded_from = loaded_from_path;
        
        // 3. 驗證配置
        config.validate()?;
        
        Ok(config)
    }
    
    /// 儲存配置到檔案
    pub fn save(&self) -> crate::Result<()> {
        let config_path = Self::config_file_path()?;
        
        // 確保目錄存在
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let toml_content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, toml_content)?;
        
        Ok(())
    }
    
    /// 取得配置檔案路徑
    pub fn config_file_path() -> crate::Result<PathBuf> {
        if let Ok(custom_path) = std::env::var("SUBX_CONFIG_PATH") {
            return Ok(PathBuf::from(custom_path));
        }
        
        let config_dir = dirs::config_dir()
            .ok_or_else(|| crate::SubXError::Config("無法確定配置目錄".to_string()))?;
        
        Ok(config_dir.join("subx").join("config.toml"))
    }
}
```

### 預設配置
```rust
// src/config.rs
impl Default for Config {
    fn default() -> Self {
        Self {
            ai: AIConfig {
                provider: "openai".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                max_sample_length: 2000,
                temperature: 0.3,
                retry_attempts: 3, // 新增預設值
                retry_delay_ms: 1000, // 新增預設值
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
                dialogue_detection_threshold: 0.01, // 新增預設值
                min_dialogue_duration_ms: 500, // 新增預設值
            },
            general: GeneralConfig {
                backup_enabled: false,
                default_confidence: 80,
                max_concurrent_jobs: num_cpus::get_physical(), // 使用 CPU 核心數作為預設並行任務數
                log_level: "info".to_string(), // 新增預設日誌級別
            },
            loaded_from: None,
        }
    }
}
```

### 環境變數處理
```rust
// src/config.rs
impl Config {
    fn apply_env_vars(&mut self) {
        // OpenAI API Key
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            self.ai.api_key = Some(api_key);
        }
        
        // 其他環境變數...
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            self.ai.model = model;
        }
    }
    
    fn validate(&self) -> crate::Result<()> {
        // API Key 驗證
        if self.ai.api_key.is_none() && std::env::var("OPENAI_API_KEY").is_err() {
            // 僅在嚴格模式或特定命令下才報錯，允許某些命令在無 API Key 時執行
            // 例如 `subx config list` 或 `subx --help`
            // 此處暫時保留原邏輯，但建議在命令執行層面做更細緻的檢查
            // return Err(crate::SubXError::Config(
            //     "未設定 OpenAI API Key，請使用 'subx config set ai.api_key <key>' 或設定 OPENAI_API_KEY 環境變數".to_string()
            // ));
        }

        if self.ai.provider != "openai" { // 簡單示例，實際應更通用
             return Err(crate::SubXError::Config(
                format!("不支援的 AI provider: {}", self.ai.provider)
            ));
        }

        Ok(())
    }
}
```

### Config 命令實作
```rust
// src/commands/config_command.rs
use crate::cli::{ConfigArgs, ConfigAction}; // 更新 ConfigAction 的引入路徑
use crate::config::Config;
use anyhow::Result; // 使用 anyhow::Result

pub async fn execute(args: ConfigArgs) -> Result<()> { // 改為 Result<()>
    match args.action {
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            // 這裡需要一個更通用的方式來設定值，例如使用 reflection 或 macro
            // 暫時簡化處理，假設 key 的格式為 section.field
            let parts: Vec<&str> = key.splitn(2, '.').collect();
            if parts.len() == 2 {
                match parts[0] {
                    "ai" => match parts[1] {
                        "api_key" => config.ai.api_key = Some(value),
                        "model" => config.ai.model = value,
                        // ... 其他 ai 欄位
                        _ => return Err(crate::SubXError::Config(format!("無效的 AI 配置鍵: {}", key)).into()),
                    },
                    "formats" => match parts[1] {
                        "default_output" => config.formats.default_output = value,
                        // ... 其他 formats 欄位
                        _ => return Err(crate::SubXError::Config(format!("無效的 Formats 配置鍵: {}", key)).into()),
                    },
                    // ... 其他 section
                    _ => return Err(crate::SubXError::Config(format!("無效的配置區段: {}", parts[0])).into()),
                }
            } else {
                return Err(crate::SubXError::Config(format!("無效的配置鍵格式: {} (應為 section.field)", key)).into());
            }
            config.save()?;
            println!("設定 {} = {}", key, config.get_value(&key)?); // 確認儲存的值
        }
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            let value = config.get_value(&key)?;
            println!("{}", value);
        }
        ConfigAction::List => {
            let config = Config::load()?;
            if let Some(path) = &config.loaded_from {
                println!("# 配置檔案路徑: {}\n", path.display());
            }
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Reset => {
            let default_config = Config::default();
            default_config.save()?;
            println!("配置已重置為預設值");
            if let Ok(path) = Config::config_file_path() {
                println!("預設配置已儲存至: {}", path.display());
            }
        }
    }
    Ok(())
}

// Config 結構中新增 get_value 和 set_value 的輔助方法 (不在 Backlog 中，但建議)
impl Config {
    // 輔助方法：根據 key 獲取值 (簡化版)
    fn get_value(&self, key: &str) -> Result<String> {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() == 2 {
            match parts[0] {
                "ai" => match parts[1] {
                    "provider" => Ok(self.ai.provider.clone()),
                    "api_key" => Ok(self.ai.api_key.clone().unwrap_or_default()),
                    "model" => Ok(self.ai.model.clone()),
                    _ => Err(crate::SubXError::Config(format!("無效的 AI 配置鍵: {}", key)).into()),
                },
                // ... 其他 section 和 field
                _ => Err(crate::SubXError::Config(format!("無效的配置區段: {}", parts[0])).into()),
            }
        } else {
            Err(crate::SubXError::Config(format!("無效的配置鍵格式: {}", key)).into())
        }
    }
}
```

## 驗收標準
1. 配置檔案正確讀寫
2. 環境變數優先權正確運作
3. 配置驗證功能完整
4. Config 命令所有操作正常
5. 跨平台路徑處理正確

## 估計工時
2-3 天

## 相依性
- 依賴 Backlog #02 (CLI 介面框架)

## 風險評估
- 低風險：配置管理是常見需求
- 注意事項：確保敏感資訊（API Key）的安全處理
