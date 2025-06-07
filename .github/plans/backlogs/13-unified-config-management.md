# Product Backlog #13: 統一配置管理系統重構

## 領域範圍
配置系統架構重構、配置載入機制統一、配置驗證強化

## 背景描述

根據配置檔案使用情況分析，發現目前的配置管理系統存在以下問題：
1. 配置載入分散在各個命令中，缺乏統一機制
2. 命令列參數與配置檔案優先權不一致
3. 配置驗證不足，容易產生執行時錯誤
4. 缺乏配置熱重載和動態更新能力

此重構旨在建立一個統一、可靠、易擴展的配置管理系統。

## 需求分析

### 功能需求

#### 1. 統一配置載入機制
- 建立單一配置載入入口點
- 實作配置來源優先權管理
- 支援多種配置來源（檔案、環境變數、命令列）
- 提供配置載入狀態追蹤

#### 2. 配置驗證強化
- 實作配置值格式驗證
- 加入配置相依性檢查
- 提供清楚的錯誤訊息
- 支援配置值範圍檢查

#### 3. 配置優先權管理
- 定義清楚的優先權順序
- 實作配置來源透明度
- 支援配置覆蓋規則
- 提供配置來源追蹤

#### 4. 動態配置支援
- 實作配置熱重載機制
- 支援配置變更通知
- 提供配置快取管理
- 實作配置版本控制

### 非功能需求

#### 1. 效能需求
- 配置載入時間 < 100ms
- 支援配置快取以避免重複載入
- 記憶體使用量控制在合理範圍

#### 2. 可靠性需求
- 配置載入失敗時提供回退機制
- 無效配置不影響系統穩定性
- 支援配置檔案損壞恢復

#### 3. 可維護性需求
- 模組化配置管理架構
- 清楚的配置項目類別劃分
- 易於擴展新配置項目

## 技術設計

### 系統架構

```
配置管理系統架構:

┌─────────────────────────────────────────────────────────┐
│                 配置管理層 (Config Manager)                │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │ 載入器管理   │  │ 驗證器管理   │  │ 快取管理     │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
├─────────────────────────────────────────────────────────┤
│                 配置來源層 (Config Sources)               │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │ 檔案來源     │  │ 環境變數     │  │ 命令列參數   │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
├─────────────────────────────────────────────────────────┤
│                 配置模型層 (Config Models)                │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │ AI 配置模型  │  │ 格式配置模型 │  │ 同步配置模型 │    │
│  └─────────────┘  └─────────────┘  └─────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### 核心模組設計

#### 1. 配置管理器 (ConfigManager)
```rust
// src/config/manager.rs
use std::sync::{Arc, RwLock};
use std::path::Path;
use tokio::sync::watch;

#[derive(Debug)]
pub struct ConfigManager {
    config: Arc<RwLock<Config>>,
    sources: Vec<Box<dyn ConfigSource>>,
    validators: Vec<Box<dyn ConfigValidator>>,
    cache: ConfigCache,
    change_notifier: watch::Sender<ConfigChangeEvent>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(ConfigChangeEvent::Initial);
        
        Self {
            config: Arc::new(RwLock::new(Config::default())),
            sources: Vec::new(),
            validators: Vec::new(),
            cache: ConfigCache::new(),
            change_notifier: tx,
        }
    }
    
    pub fn add_source(mut self, source: Box<dyn ConfigSource>) -> Self {
        self.sources.push(source);
        self
    }
    
    pub fn add_validator(mut self, validator: Box<dyn ConfigValidator>) -> Self {
        self.validators.push(validator);
        self
    }
    
    pub async fn load(&mut self) -> Result<(), ConfigError> {
        let mut merged_config = Config::default();
        
        // 按優先權順序載入配置
        for source in &self.sources {
            let partial_config = source.load().await?;
            merged_config = merged_config.merge(partial_config)?;
        }
        
        // 驗證配置
        for validator in &self.validators {
            validator.validate(&merged_config)?;
        }
        
        // 更新配置和快取
        {
            let mut config = self.config.write().unwrap();
            *config = merged_config;
        }
        
        self.cache.update(&merged_config);
        
        // 發送變更通知
        let _ = self.change_notifier.send(ConfigChangeEvent::Updated);
        
        Ok(())
    }
    
    pub fn get_config(&self) -> Arc<RwLock<Config>> {
        Arc::clone(&self.config)
    }
    
    pub fn subscribe_changes(&self) -> watch::Receiver<ConfigChangeEvent> {
        self.change_notifier.subscribe()
    }
}
```

#### 2. 配置來源抽象 (ConfigSource)
```rust
// src/config/source.rs
use async_trait::async_trait;

#[async_trait]
pub trait ConfigSource: Send + Sync {
    async fn load(&self) -> Result<PartialConfig, ConfigError>;
    fn priority(&self) -> u8; // 0 = 最高優先權
    fn source_name(&self) -> &'static str;
}

// 檔案來源實作
pub struct FileSource {
    path: PathBuf,
    format: ConfigFormat,
}

#[async_trait]
impl ConfigSource for FileSource {
    async fn load(&self) -> Result<PartialConfig, ConfigError> {
        let content = tokio::fs::read_to_string(&self.path).await
            .map_err(|e| ConfigError::FileRead(self.path.clone(), e))?;
        
        match self.format {
            ConfigFormat::Toml => toml::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string())),
            ConfigFormat::Json => serde_json::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string())),
            ConfigFormat::Yaml => serde_yaml::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string())),
        }
    }
    
    fn priority(&self) -> u8 { 10 } // 中等優先權
    
    fn source_name(&self) -> &'static str { "file" }
}

// 環境變數來源實作
pub struct EnvSource {
    prefix: String,
}

#[async_trait]
impl ConfigSource for EnvSource {
    async fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut config = PartialConfig::default();
        
        // 讀取環境變數並轉換為配置
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.ai.api_key = Some(api_key);
        }
        
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            config.ai.model = Some(model);
        }
        
        // ... 其他環境變數處理
        
        Ok(config)
    }
    
    fn priority(&self) -> u8 { 5 } // 高優先權
    
    fn source_name(&self) -> &'static str { "environment" }
}

// 命令列參數來源實作
pub struct ArgsSource {
    args: ConfigArgs,
}

#[async_trait]
impl ConfigSource for ArgsSource {
    async fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut config = PartialConfig::default();
        
        // 將命令列參數轉換為配置
        if let Some(ref model) = self.args.ai_model {
            config.ai.model = Some(model.clone());
        }
        
        if let Some(temperature) = self.args.ai_temperature {
            config.ai.temperature = Some(temperature);
        }
        
        // ... 其他參數處理
        
        Ok(config)
    }
    
    fn priority(&self) -> u8 { 0 } // 最高優先權
    
    fn source_name(&self) -> &'static str { "command_line" }
}
```

#### 3. 配置驗證器 (ConfigValidator)
```rust
// src/config/validator.rs
pub trait ConfigValidator: Send + Sync {
    fn validate(&self, config: &Config) -> Result<(), ConfigError>;
    fn validator_name(&self) -> &'static str;
}

// AI 配置驗證器
pub struct AIConfigValidator;

impl ConfigValidator for AIConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        // 驗證 API 金鑰格式
        if let Some(ref api_key) = config.ai.api_key {
            if !api_key.starts_with("sk-") {
                return Err(ConfigError::InvalidValue(
                    "ai.api_key".to_string(),
                    "OpenAI API 金鑰必須以 'sk-' 開頭".to_string(),
                ));
            }
        }
        
        // 驗證模型名稱
        let valid_models = ["gpt-4", "gpt-4-turbo", "gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo"];
        if !valid_models.contains(&config.ai.model.as_str()) {
            return Err(ConfigError::InvalidValue(
                "ai.model".to_string(),
                format!("不支援的模型: {}，支援的模型: {:?}", 
                    config.ai.model, valid_models),
            ));
        }
        
        // 驗證溫度範圍
        if config.ai.temperature < 0.0 || config.ai.temperature > 2.0 {
            return Err(ConfigError::InvalidValue(
                "ai.temperature".to_string(),
                "溫度值必須在 0.0 到 2.0 之間".to_string(),
            ));
        }
        
        // 驗證重試次數
        if config.ai.retry_attempts > 10 {
            return Err(ConfigError::InvalidValue(
                "ai.retry_attempts".to_string(),
                "重試次數不能超過 10 次".to_string(),
            ));
        }
        
        Ok(())
    }
    
    fn validator_name(&self) -> &'static str { "ai_config" }
}

// 同步配置驗證器
pub struct SyncConfigValidator;

impl ConfigValidator for SyncConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        // 驗證最大偏移時間
        if config.sync.max_offset_seconds <= 0.0 || config.sync.max_offset_seconds > 300.0 {
            return Err(ConfigError::InvalidValue(
                "sync.max_offset_seconds".to_string(),
                "最大偏移秒數必須在 0.0 到 300.0 之間".to_string(),
            ));
        }
        
        // 驗證相關性閾值
        if config.sync.correlation_threshold < 0.0 || config.sync.correlation_threshold > 1.0 {
            return Err(ConfigError::InvalidValue(
                "sync.correlation_threshold".to_string(),
                "相關性閾值必須在 0.0 到 1.0 之間".to_string(),
            ));
        }
        
        Ok(())
    }
    
    fn validator_name(&self) -> &'static str { "sync_config" }
}
```

#### 4. 配置快取管理 (ConfigCache)
```rust
// src/config/cache.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct ConfigCache {
    entries: HashMap<String, CacheEntry>,
    default_ttl: Duration,
}

struct CacheEntry {
    value: serde_json::Value,
    created_at: Instant,
    ttl: Duration,
}

impl ConfigCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::from_secs(300), // 5 分鐘
        }
    }
    
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let entry = self.entries.get(key)?;
        
        // 檢查是否過期
        if entry.created_at.elapsed() > entry.ttl {
            return None;
        }
        
        // 反序列化值
        serde_json::from_value(entry.value.clone()).ok()
    }
    
    pub fn set<T>(&mut self, key: String, value: T, ttl: Option<Duration>)
    where
        T: serde::Serialize,
    {
        let json_value = serde_json::to_value(value).unwrap();
        let entry = CacheEntry {
            value: json_value,
            created_at: Instant::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        };
        
        self.entries.insert(key, entry);
    }
    
    pub fn update(&mut self, config: &Config) {
        // 更新配置快取
        self.set("full_config".to_string(), config, None);
        self.set("ai_config".to_string(), &config.ai, None);
        self.set("formats_config".to_string(), &config.formats, None);
        self.set("sync_config".to_string(), &config.sync, None);
        self.set("general_config".to_string(), &config.general, None);
    }
    
    pub fn clear(&mut self) {
        self.entries.clear();
    }
    
    pub fn cleanup_expired(&mut self) {
        self.entries.retain(|_, entry| {
            entry.created_at.elapsed() <= entry.ttl
        });
    }
}
```

### 部分配置模型 (PartialConfig)
```rust
// src/config/partial.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialConfig {
    pub ai: PartialAIConfig,
    pub formats: PartialFormatsConfig,
    pub sync: PartialSyncConfig,
    pub general: PartialGeneralConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialAIConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub max_sample_length: Option<usize>,
    pub temperature: Option<f32>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

// ... 其他部分配置結構

// 配置合併邏輯
impl Config {
    pub fn merge(mut self, partial: PartialConfig) -> Result<Self, ConfigError> {
        // AI 配置合併
        if let Some(provider) = partial.ai.provider {
            self.ai.provider = provider;
        }
        if let Some(api_key) = partial.ai.api_key {
            self.ai.api_key = Some(api_key);
        }
        if let Some(model) = partial.ai.model {
            self.ai.model = model;
        }
        if let Some(max_sample_length) = partial.ai.max_sample_length {
            self.ai.max_sample_length = max_sample_length;
        }
        if let Some(temperature) = partial.ai.temperature {
            self.ai.temperature = temperature;
        }
        if let Some(retry_attempts) = partial.ai.retry_attempts {
            self.ai.retry_attempts = retry_attempts;
        }
        if let Some(retry_delay_ms) = partial.ai.retry_delay_ms {
            self.ai.retry_delay_ms = retry_delay_ms;
        }
        
        // ... 其他配置區塊的合併邏輯
        
        Ok(self)
    }
}
```

## 實作計劃

### 階段 1: 核心架構建立 (預估工時: 8 小時)

#### 1.1 建立配置管理器模組
- [ ] 建立 `src/config/` 模組結構
- [ ] 實作 `ConfigManager` 核心類別
- [ ] 定義 `ConfigSource` 和 `ConfigValidator` trait
- [ ] 實作基本的錯誤處理

#### 1.2 實作配置來源
- [ ] 實作 `FileSource` 檔案配置來源
- [ ] 實作 `EnvSource` 環境變數來源
- [ ] 實作 `ArgsSource` 命令列參數來源
- [ ] 建立配置優先權管理

#### 1.3 建立部分配置系統
- [ ] 定義 `PartialConfig` 結構
- [ ] 實作配置合併邏輯
- [ ] 處理配置衝突解決

### 階段 2: 驗證和快取系統 (預估工時: 6 小時)

#### 2.1 實作配置驗證器
- [ ] 實作 `AIConfigValidator`
- [ ] 實作 `SyncConfigValidator`
- [ ] 實作 `FormatsConfigValidator`
- [ ] 實作 `GeneralConfigValidator`

#### 2.2 建立快取管理系統
- [ ] 實作 `ConfigCache` 類別
- [ ] 加入快取過期機制
- [ ] 實作快取更新和清理

#### 2.3 錯誤處理強化
- [ ] 定義詳細的 `ConfigError` 類型
- [ ] 實作錯誤恢復機制
- [ ] 加入使用者友善的錯誤訊息

### 階段 3: 動態配置支援 (預估工時: 4 小時)

#### 3.1 實作配置變更通知
- [ ] 建立配置變更事件系統
- [ ] 實作配置訂閱機制
- [ ] 加入配置熱重載功能

#### 3.2 配置檔案監控
- [ ] 實作檔案變更監控
- [ ] 自動重載配置檔案
- [ ] 處理檔案損壞情況

### 階段 4: 整合現有系統 (預估工時: 8 小時)

#### 4.1 更新命令系統
- [ ] 修改所有命令使用新配置管理器
- [ ] 移除舊的配置載入邏輯
- [ ] 確保命令列參數優先權正確

#### 4.2 更新服務層
- [ ] 修改 AI 服務使用統一配置
- [ ] 更新格式轉換服務配置
- [ ] 修改同步引擎配置載入

#### 4.3 向後相容性處理
- [ ] 實作舊配置檔案相容性
- [ ] 加入配置遷移提示
- [ ] 處理配置升級邏輯

### 階段 5: 測試和文件 (預估工時: 6 小時)

#### 5.1 單元測試
- [ ] 配置管理器測試
- [ ] 配置來源測試
- [ ] 配置驗證器測試
- [ ] 配置快取測試

#### 5.2 整合測試
- [ ] 端到端配置載入測試
- [ ] 多來源配置合併測試
- [ ] 錯誤恢復測試
- [ ] 效能測試

#### 5.3 文件更新
- [ ] API 文件更新
- [ ] 配置指南更新
- [ ] 遷移指南建立

## 使用範例

### 基本使用
```rust
// 在應用程式啟動時
let mut config_manager = ConfigManager::new()
    .add_source(Box::new(FileSource::new("~/.config/subx/config.toml")))
    .add_source(Box::new(EnvSource::new("SUBX_".to_string())))
    .add_source(Box::new(ArgsSource::new(args)))
    .add_validator(Box::new(AIConfigValidator))
    .add_validator(Box::new(SyncConfigValidator));

config_manager.load().await?;

let config = config_manager.get_config();

// 在命令中使用
let ai_config = {
    let config = config.read().unwrap();
    config.ai.clone()
};

let ai_client = OpenAIClient::new(
    ai_config.api_key.unwrap(),
    ai_config.model,
    ai_config.temperature,
    ai_config.retry_attempts,
    ai_config.retry_delay_ms,
)?;
```

### 配置變更監聽
```rust
let mut change_receiver = config_manager.subscribe_changes();

tokio::spawn(async move {
    while let Ok(event) = change_receiver.changed().await {
        match *change_receiver.borrow() {
            ConfigChangeEvent::Updated => {
                println!("配置已更新，重新載入服務...");
                // 重新初始化相關服務
            }
            ConfigChangeEvent::Error(ref err) => {
                eprintln!("配置載入錯誤: {}", err);
            }
            _ => {}
        }
    }
});
```

## 測試計劃

### 單元測試覆蓋
- [ ] 配置管理器功能測試 (90% 覆蓋率)
- [ ] 配置來源實作測試 (95% 覆蓋率)
- [ ] 配置驗證器測試 (100% 覆蓋率)
- [ ] 配置快取功能測試 (85% 覆蓋率)

### 整合測試場景
- [ ] 多來源配置載入測試
- [ ] 配置優先權正確性測試
- [ ] 錯誤恢復機制測試
- [ ] 配置熱重載測試
- [ ] 效能壓力測試

### 相容性測試
- [ ] 舊配置檔案載入測試
- [ ] 不同作業系統配置路徑測試
- [ ] Unicode 配置值處理測試

## 驗收標準

### 功能驗收
- [ ] 所有配置來源正確載入
- [ ] 配置優先權按預期工作
- [ ] 配置驗證捕捉所有無效值
- [ ] 配置變更即時生效
- [ ] 錯誤處理友善且詳細

### 程式碼品質驗收
- [ ] 通過所有 clippy 檢查
- [ ] 程式碼覆蓋率 > 85%
- [ ] 文件覆蓋率 > 90%

## 風險評估

### 技術風險
- **高風險**: 大規模重構可能引入回歸錯誤
- **緩解措施**: 階段性實作，每階段充分測試

### 相容性風險
- **中等風險**: 可能破壞現有配置檔案
- **緩解措施**: 實作強健的向後相容性支援

### 效能風險
- **低風險**: 新系統可能影響啟動效能
- **緩解措施**: 配置快取和延遲載入

## 後續改進計劃

### 配置 UI 支援
- 建立配置檔案編輯器
- 實作配置驗證即時回饋
- 提供配置範本和精靈

### 高級功能
- 實作配置環境切換 (開發/生產)
- 加入配置加密支援
- 建立配置審計日誌

### 雲端整合
- 支援雲端配置同步
- 實作配置版本控制
- 加入團隊配置共享功能

## 完成效益

### 技術效益
- **統一性**: 所有配置透過單一系統管理
- **可靠性**: 強化的驗證和錯誤處理
- **靈活性**: 支援多種配置來源和格式
- **效能**: 配置快取減少重複載入

### 開發效益
- **可維護性**: 模組化配置管理架構
- **可擴展性**: 易於新增配置項目和來源
- **可測試性**: 清楚的介面和相依性分離

### 使用者效益
- **易用性**: 清楚的錯誤訊息和配置指引
- **靈活性**: 多種配置方式滿足不同需求
- **穩定性**: 配置錯誤不影響系統運作
