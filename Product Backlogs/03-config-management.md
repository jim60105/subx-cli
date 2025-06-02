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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    pub max_sample_length: usize,
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatsConfig {
    pub default_output: String,
    pub preserve_styling: bool,
    pub default_encoding: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub audio_sample_rate: u32,
    pub correlation_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub backup_enabled: bool,
    pub default_confidence: u8,
    pub max_concurrent_jobs: usize,
}
```

### 配置管理器
```rust
// src/config.rs
impl Config {
    /// 載入配置（環境變數 > 配置檔案 > 預設值）
    pub fn load() -> crate::Result<Self> {
        let mut config = Self::default();
        
        // 1. 載入配置檔案
        if let Some(file_config) = Self::load_from_file()? {
            config.merge(file_config);
        }
        
        // 2. 環境變數覆蓋
        config.apply_env_vars();
        
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
            },
            general: GeneralConfig {
                backup_enabled: false,
                default_confidence: 80,
                max_concurrent_jobs: 4,
            },
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
        if self.ai.api_key.is_none() {
            return Err(crate::SubXError::Config(
                "未設定 OpenAI API Key，請使用 'subx config set ai.api_key <key>' 或設定 OPENAI_API_KEY 環境變數".to_string()
            ));
        }
        
        // 數值範圍驗證
        if self.general.default_confidence > 100 {
            return Err(crate::SubXError::Config(
                "預設信心度必須在 0-100 之間".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### Config 命令實作
```rust
// src/commands/config_command.rs
use crate::cli::ConfigArgs;
use crate::config::Config;

pub async fn execute(args: ConfigArgs) -> crate::Result<()> {
    match args.action {
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            config.set_value(&key, &value)?;
            config.save()?;
            println!("設定 {} = {}", key, value);
        }
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            let value = config.get_value(&key)?;
            println!("{}", value);
        }
        ConfigAction::List => {
            let config = Config::load()?;
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigAction::Reset => {
            let default_config = Config::default();
            default_config.save()?;
            println!("配置已重置為預設值");
        }
    }
    Ok(())
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
