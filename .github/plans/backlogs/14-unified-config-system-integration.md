# Product Backlog #14: 統一配置管理系統整合

## 概述

**目標**: 完全遷移至新的統一配置管理系統，移除舊配置機制，實現單一配置管理架構
**優先級**: 高
**預估工作量**: 5 天
**前置需求**: Backlog #13 (統一配置管理系統) 已完成

## 業務價值

### 問題陳述
當前 SubX 存在雙軌配置系統並存的問題：
- **舊系統**: `src/config.rs` 中的直接 TOML 載入機制 (`Config::load()`)
- **新系統**: `src/config/` 中的統一配置管理架構 (`ConfigManager`)

**具體問題分析**:
1. 所有命令檔案 (`src/commands/*.rs`) 仍使用 `Config::load()`
2. 新系統的多來源載入、動態監控、配置驗證等進階功能無法發揮作用
3. 雙軌系統增加維護複雜度和程式碼重複

### 解決方案價值
- **簡化維護**: 統一配置管理邏輯，減少程式碼重複
- **增強功能**: 獲得多來源載入和動態配置監控能力
- **提升穩定性**: 統一的配置驗證和錯誤處理機制
- **改善體驗**: 更好的配置錯誤訊息和使用者指引

## 功能需求與實作細節

### Epic 1: 建立統一配置載入介面
**作為** 開發者
**我想要** 建立一個簡潔的全域配置載入介面
**以便** 所有程式碼都可以一致地存取配置

#### User Story 1.1: 建立全域配置管理器
**實作檔案**: `src/config.rs`

**目標**: 移除舊的 `Config::load()` 實作，建立基於 `ConfigManager` 的全域配置載入函數

**詳細實作步驟**:

1. **移除舊實作** - 移除 `Config::load()` 方法
```rust
// 移除整個 impl Config 區塊中的 load() 方法 (第 229-250 行)
// 保留其他方法: save(), config_file_path(), apply_env_vars(), validate(), get_value(), merge()
```

2. **加入新的依賴項目**
```rust
// 在檔案頂部加入
use std::sync::OnceLock;
use crate::config::manager::ConfigManager;
use crate::config::source::{FileSource, EnvSource, CliSource};
use crate::config::partial::PartialConfig;
```

3. **建立全域配置管理器**
```rust
// 建立全域配置管理器實例
static GLOBAL_CONFIG_MANAGER: OnceLock<ConfigManager> = OnceLock::new();

/// 初始化全域配置管理器
pub fn init_config_manager() -> crate::Result<()> {
    let manager = ConfigManager::new()
        .add_source(Box::new(FileSource::new(Config::config_file_path()?)))
        .add_source(Box::new(EnvSource::new()))
        .add_source(Box::new(CliSource::new()));
    
    manager.load().map_err(|e| SubXError::config(e.to_string()))?;
    
    GLOBAL_CONFIG_MANAGER.set(manager)
        .map_err(|_| SubXError::config("配置管理器已經初始化".to_string()))?;
    
    Ok(())
}
```

4. **建立新的配置載入函數**
```rust
/// 載入應用程式配置（替代 Config::load()）
pub fn load_config() -> crate::Result<Config> {
    let manager = GLOBAL_CONFIG_MANAGER.get()
        .ok_or_else(|| SubXError::config("配置管理器尚未初始化，請先呼叫 init_config_manager()".to_string()))?;
    
    let config_lock = manager.config();
    let partial_config = config_lock.read().unwrap();
    
    // 轉換 PartialConfig 為 Config
    let config = partial_config.to_complete_config()?;
    Ok(config)
}
```

#### User Story 1.2: 實作 PartialConfig 到 Config 的轉換
**實作檔案**: `src/config/partial.rs`

**目標**: 加入將 `PartialConfig` 轉換為完整 `Config` 的方法

**詳細實作步驟**:

1. **加入轉換方法**
```rust
impl PartialConfig {
    /// 轉換為完整配置，使用預設值填充缺少的欄位
    pub fn to_complete_config(&self) -> Result<crate::config::Config, crate::config::manager::ConfigError> {
        use crate::config::{Config, AIConfig, FormatsConfig, SyncConfig, GeneralConfig};
        
        let default_config = Config::default();
        
        let ai = AIConfig {
            provider: self.ai.provider.clone().unwrap_or(default_config.ai.provider),
            api_key: self.ai.api_key.clone().or(default_config.ai.api_key),
            model: self.ai.model.clone().unwrap_or(default_config.ai.model),
            max_sample_length: self.ai.max_sample_length.unwrap_or(default_config.ai.max_sample_length),
            temperature: self.ai.temperature.unwrap_or(default_config.ai.temperature),
            retry_attempts: self.ai.retry_attempts.unwrap_or(default_config.ai.retry_attempts),
            retry_delay_ms: self.ai.retry_delay_ms.unwrap_or(default_config.ai.retry_delay_ms),
        };
        
        let formats = FormatsConfig {
            default_output: self.formats.default_output.clone().unwrap_or(default_config.formats.default_output),
            preserve_styling: self.formats.preserve_styling.unwrap_or(default_config.formats.preserve_styling),
            default_encoding: self.formats.default_encoding.clone().unwrap_or(default_config.formats.default_encoding),
        };
        
        let sync = SyncConfig {
            max_offset_seconds: self.sync.max_offset_seconds.unwrap_or(default_config.sync.max_offset_seconds),
            audio_sample_rate: self.sync.audio_sample_rate.unwrap_or(default_config.sync.audio_sample_rate),
            correlation_threshold: self.sync.correlation_threshold.unwrap_or(default_config.sync.correlation_threshold),
            dialogue_detection_threshold: self.sync.dialogue_detection_threshold.unwrap_or(default_config.sync.dialogue_detection_threshold),
            min_dialogue_duration_ms: self.sync.min_dialogue_duration_ms.unwrap_or(default_config.sync.min_dialogue_duration_ms),
        };
        
        let general = GeneralConfig {
            backup_enabled: self.general.backup_enabled.unwrap_or(default_config.general.backup_enabled),
            max_concurrent_jobs: self.general.max_concurrent_jobs.unwrap_or(default_config.general.max_concurrent_jobs),
        };
        
        Ok(Config {
            ai,
            formats,
            sync,
            general,
            loaded_from: None,
        })
    }
}
```

#### User Story 1.3: 實作環境變數和 CLI 配置來源
**實作檔案**: `src/config/source.rs`

**目標**: 完成環境變數和命令列參數配置來源的實作

**詳細實作步驟**:

1. **實作環境變數來源**
```rust
/// Environment variable configuration source.
pub struct EnvSource;

impl EnvSource {
    pub fn new() -> Self {
        Self
    }
}

impl ConfigSource for EnvSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut config = PartialConfig::default();
        
        // AI 相關環境變數
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.ai.api_key = Some(api_key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            config.ai.model = Some(model);
        }
        if let Ok(provider) = std::env::var("SUBX_AI_PROVIDER") {
            config.ai.provider = Some(provider);
        }
        
        // 其他環境變數...
        if let Ok(backup) = std::env::var("SUBX_BACKUP_ENABLED") {
            config.general.backup_enabled = Some(backup.parse().unwrap_or(false));
        }
        
        Ok(config)
    }
    
    fn priority(&self) -> u8 {
        5 // 中等優先權，高於檔案但低於 CLI
    }
    
    fn source_name(&self) -> &'static str {
        "environment"
    }
}
```

2. **實作 CLI 參數來源**
```rust
/// Command line arguments configuration source.
pub struct CliSource {
    // 可以在初始化時傳入 CLI 參數
}

impl CliSource {
    pub fn new() -> Self {
        Self {}
    }
}

impl ConfigSource for CliSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        // 基本實作，後續可以根據需要擴展
        Ok(PartialConfig::default())
    }
    
    fn priority(&self) -> u8 {
        1 // 最高優先權
    }
    
    fn source_name(&self) -> &'static str {
        "cli"
    }
}
```

### Epic 2: 應用程式初始化整合
**作為** 應用程式
**我想要** 在啟動時初始化統一配置管理器
**以便** 所有後續的配置存取都使用新系統

#### User Story 2.1: 修改主程式初始化
**實作檔案**: `src/main.rs`

**目標**: 在應用程式啟動時初始化配置管理器

**詳細實作步驟**:

```rust
// src/main.rs
use subx_cli::config::init_config_manager;

#[tokio::main]
async fn main() {
    // 初始化日誌
    env_logger::init();

    // 初始化配置管理器
    if let Err(e) = init_config_manager() {
        eprintln!("配置初始化失敗: {}", e.user_friendly_message());
        std::process::exit(1);
    }

    let result = subx_cli::cli::run().await;
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_friendly_message());
            std::process::exit(e.exit_code());
        }
    }
}
```

#### User Story 2.2: 修改 CLI 模組
**實作檔案**: `src/cli/mod.rs` (如果存在) 或相關 CLI 處理檔案

**目標**: 確保 CLI 參數能正確傳遞給配置系統

### Epic 3: 命令層重構
**作為** CLI 命令
**我想要** 使用新的配置管理系統
**以便** 獲得統一的配置載入和驗證

#### User Story 3.1: 更新所有命令檔案
**實作檔案**: 
- `src/commands/convert_command.rs`
- `src/commands/match_command.rs` 
- `src/commands/sync_command.rs`
- `src/commands/config_command.rs`

**目標**: 替換所有 `Config::load()` 呼叫為 `load_config()`

**詳細實作步驟**:

1. **convert_command.rs** (第 9 行)
```rust
// 修改前:
let app_config = Config::load()?;

// 修改後:
let app_config = crate::config::load_config()?;
```

2. **match_command.rs** (第 12, 31 行)
```rust
// 修改前:
let config = Config::load()?;

// 修改後:  
let config = crate::config::load_config()?;
```

3. **sync_command.rs** (第 13 行)
```rust
// 修改前:
let app_config = Config::load()?;

// 修改後:
let app_config = crate::config::load_config()?;
```

4. **config_command.rs** (第 10, 44, 49 行)
```rust
// 修改前:
let mut config = Config::load()?;
let config = Config::load()?;

// 修改後:
let mut config = crate::config::load_config()?;
let config = crate::config::load_config()?;
```

#### User Story 3.2: 更新匯入語句
**實作檔案**: 所有命令檔案

**目標**: 更新匯入語句以使用新的配置載入函數

**詳細實作步驟**:

在每個命令檔案中，更新匯入語句:
```rust
// 修改前:
use crate::config::Config;

// 修改後:
use crate::config::{Config, load_config};
```

### Epic 4: 核心模組整合
**作為** 核心引擎模組
**我想要** 使用統一的配置管理
**以便** 獲得一致的配置存取和驗證

#### User Story 4.1: 更新核心引擎模組
**實作檔案**: `src/core/matcher/engine.rs`

**目標**: 更新 matcher 引擎使用新配置系統

**詳細實作步驟**:

找到配置載入的地方並替換:
```rust
// 修改前:
use crate::config::Config;
// ... 在程式碼中的某處
let config = Config::load()?;

// 修改後:
use crate::config::{Config, load_config};
// ... 在程式碼中的某處  
let config = load_config()?;
```

### Epic 5: 測試系統重構
**作為** 開發團隊
**我想要** 確保所有測試使用新的配置系統
**以便** 保證系統品質和可靠性

#### User Story 5.1: 更新配置相關測試
**實作檔案**: `src/config.rs` (測試區塊)

**目標**: 移除舊系統測試，建立新系統測試

**詳細實作步驟**:

1. **移除舊測試** - 移除與 `Config::load()` 相關的測試
2. **建立新測試**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_global_config_manager_initialization() {
        // 測試全域配置管理器初始化
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        // 建立測試配置檔案
        let test_config = Config::default();
        let toml_content = toml::to_string_pretty(&test_config).unwrap();
        std::fs::write(&config_path, toml_content).unwrap();
        
        std::env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
        
        // 測試初始化
        assert!(init_config_manager().is_ok());
        
        // 測試載入
        let loaded_config = load_config().unwrap();
        assert_eq!(loaded_config.ai.model, test_config.ai.model);
        
        std::env::remove_var("SUBX_CONFIG_PATH");
    }
    
    #[test]
    fn test_env_var_override_with_new_system() {
        std::env::set_var("OPENAI_API_KEY", "test-key-from-env");
        std::env::set_var("SUBX_AI_MODEL", "gpt-4-from-env");
        
        // 重新初始化配置管理器以載入環境變數
        let _ = init_config_manager();
        let config = load_config().unwrap();
        
        assert_eq!(config.ai.api_key, Some("test-key-from-env".to_string()));
        assert_eq!(config.ai.model, "gpt-4-from-env");
        
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("SUBX_AI_MODEL");
    }
    
    // 保留現有的其他測試...
}
```

#### User Story 5.2: 建立整合測試
**實作檔案**: `tests/config_integration_tests.rs` (新檔案)

**目標**: 建立端到端配置整合測試

**詳細實作步驟**:

```rust
//! 配置系統整合測試

use std::env;
use tempfile::TempDir;
use subx_cli::config::{init_config_manager, load_config, Config};

#[test]
fn test_full_config_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    // 建立測試配置檔案
    let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
max_sample_length = 3000

[general]
backup_enabled = true
max_concurrent_jobs = 8
"#;
    
    std::fs::write(&config_path, config_content).unwrap();
    env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
    env::set_var("OPENAI_API_KEY", "env-api-key");
    
    // 測試完整流程
    assert!(init_config_manager().is_ok());
    let config = load_config().unwrap();
    
    // 驗證檔案配置載入
    assert_eq!(config.ai.model, "gpt-4");
    assert_eq!(config.ai.max_sample_length, 3000);
    assert_eq!(config.general.max_concurrent_jobs, 8);
    
    // 驗證環境變數覆蓋
    assert_eq!(config.ai.api_key, Some("env-api-key".to_string()));
    
    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_API_KEY");
}
```

## 技術概要與架構設計

### 當前架構分析

#### 舊配置系統 (`src/config.rs`)
```rust
// 目前使用的載入方式
pub fn load() -> Result<Self> {
    let mut config = Config::default();
    // 直接 TOML 檔案載入
    if let Ok(path) = Config::config_file_path() {
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let file_config: Config = toml::from_str(&content)?;
            config.merge(file_config);
        }
    }
    // 環境變數覆蓋
    config.apply_env_vars();
    config.validate()?;
    Ok(config)
}
```

**問題**:
1. 無法支援多來源配置合併的細粒度控制
2. 缺乏配置監控和動態更新能力  
3. 配置驗證邏輯分散且不統一
4. 無法支援配置快取和效能最佳化

#### 新配置系統 (`src/config/`)
```rust
// 新系統架構
ConfigManager -> ConfigSource[] -> PartialConfig -> Config
    ↓              ↓                   ↓           ↓
  管理器        多來源載入          部分配置     完整配置
  (快取)       (檔案/環境/CLI)      (可選欄位)   (必填欄位)
```

**優勢**:
1. **多來源支援**: 檔案、環境變數、CLI 參數各自獨立處理
2. **優先權控制**: 可配置的來源優先權系統
3. **動態監控**: 檔案變更自動重新載入
4. **配置快取**: 避免重複載入和解析
5. **統一驗證**: 集中式配置驗證邏輯

### 核心變更

#### 1. 配置載入流程重構
```rust
// 舊流程 (直接)
Config::load() -> 直接 TOML 解析 -> Config

// 新流程 (通過管理器)
init_config_manager() -> ConfigManager 建立 -> 多來源註冊
load_config() -> ConfigManager.load() -> PartialConfig 合併 -> Config
```

#### 2. 全域配置管理器模式
```rust
// 使用 OnceLock 實作單例模式
static GLOBAL_CONFIG_MANAGER: OnceLock<ConfigManager> = OnceLock::new();

// 初始化一次，全域使用
pub fn init_config_manager() -> Result<()>
pub fn load_config() -> Result<Config>
```

#### 3. 配置來源優先權
```
優先權 (數字越小越高):
1. CLI 參數      (priority = 1)
2. 環境變數      (priority = 5) 
3. 配置檔案      (priority = 10)
4. 預設值        (priority = 255)
```

### 相依性管理

#### 新增相依項目
需要在 `Cargo.toml` 中確認以下相依項目已存在:
```toml
[dependencies]
notify = "6.0"          # 檔案監控
tokio = { version = "1.0", features = ["sync"] }  # 非同步支援
```

#### 模組相依關係
```
src/config.rs (公開介面)
├── src/config/manager.rs (配置管理器核心)
├── src/config/source.rs (配置來源實作)  
├── src/config/partial.rs (部分配置結構)
├── src/config/cache.rs (配置快取)
└── src/config/validator.rs (配置驗證)
```

### 錯誤處理策略

#### 配置錯誤分類
1. **初始化錯誤**: 配置管理器重複初始化
2. **載入錯誤**: 檔案不存在、格式錯誤、權限問題
3. **驗證錯誤**: 配置值不符合業務規則
4. **轉換錯誤**: PartialConfig 到 Config 轉換失敗

#### 錯誤處理實作
```rust
// 統一錯誤轉換
impl From<config::manager::ConfigError> for SubXError {
    fn from(err: config::manager::ConfigError) -> Self {
        SubXError::config(format!("配置系統錯誤: {}", err))
    }
}
```

## 實作計畫

### 第 1 階段: 基礎建設重構 (1.5 天)
**目標**: 建立新配置系統基礎，完成核心介面建立

#### 任務 1.1: 建立統一配置載入介面 (4 小時)
**負責檔案**: `src/config.rs`

**執行步驟**:
1. **移除舊實作**
   ```bash
   # 找到並移除 Config::load() 方法 (約第 229-250 行)
   # 保留: save(), config_file_path(), apply_env_vars(), validate(), get_value(), merge()
   ```

2. **加入新依賴和全域管理器**
   ```rust
   // 在檔案頂部加入匯入
   use std::sync::OnceLock;
   use crate::config::manager::ConfigManager;
   use crate::config::source::{FileSource, EnvSource, CliSource};
   
   // 建立全域配置管理器
   static GLOBAL_CONFIG_MANAGER: OnceLock<ConfigManager> = OnceLock::new();
   ```

3. **實作初始化和載入函數** (參考上面的程式碼範例)

4. **驗證編譯**
   ```bash
   cargo check
   # 預期會看到所有使用 Config::load() 的地方出現編譯錯誤
   # 這是正常的，代表我們成功識別了所有需要修改的地方
   ```

#### 任務 1.2: 完善配置來源實作 (4 小時)
**負責檔案**: `src/config/source.rs`

**執行步驟**:
1. **實作 EnvSource** (參考上面的程式碼範例)
2. **實作 CliSource** (基本版本)
3. **測試配置來源**
   ```bash
   cargo test config::source
   ```

#### 任務 1.3: 實作配置轉換邏輯 (4 小時)  
**負責檔案**: `src/config/partial.rs`

**執行步驟**:
1. **加入 to_complete_config() 方法** (參考上面的程式碼範例)
2. **加入必要的匯入語句**
3. **測試轉換邏輯**
   ```bash
   cargo test config::partial
   ```

### 第 2 階段: 應用程式初始化整合 (0.5 天)
**目標**: 修改應用程式啟動流程，整合新配置系統

#### 任務 2.1: 修改主程式 (2 小時)
**負責檔案**: `src/main.rs`

**執行步驟**:
1. **加入配置初始化** (參考上面的程式碼範例)
2. **測試應用程式啟動**
   ```bash
   cargo run -- --help
   # 應該正常啟動並顯示說明
   ```

#### 任務 2.2: 修改 lib.rs 匯出 (2 小時)
**負責檔案**: `src/lib.rs`

**執行步驟**:
1. **更新公開介面匯出**
   ```rust
   // 確保匯出新的配置函數
   pub use config::{init_config_manager, load_config};
   ```

### 第 3 階段: 命令層全面重構 (2 天)
**目標**: 更新所有 CLI 命令使用新配置系統

#### 任務 3.1: 更新配置相關命令 (1 天)
**負責檔案**: `src/commands/config_command.rs`

**執行步驟**:
1. **替換配置載入**
   ```rust
   // 第 10 行: let mut config = Config::load()?;
   let mut config = crate::config::load_config()?;
   
   // 第 44 行: let config = Config::load()?;  
   let config = crate::config::load_config()?;
   
   // 第 49 行: let config = Config::load()?;
   let config = crate::config::load_config()?;
   ```

2. **更新匯入語句**
   ```rust
   // 修改匯入
   use crate::config::{Config, load_config};
   ```

3. **測試配置命令**
   ```bash
   cargo run -- config get ai.model
   cargo run -- config set ai.model gpt-4
   cargo run -- config list
   ```

#### 任務 3.2: 更新轉換命令 (2 小時)
**負責檔案**: `src/commands/convert_command.rs`

**執行步驟**:
1. **替換第 9 行的配置載入**
2. **更新匯入語句**  
3. **測試轉換功能**
   ```bash
   # 建立測試檔案並測試轉換
   echo "Test subtitle" > test.srt
   cargo run -- convert test.srt --format vtt
   ```

#### 任務 3.3: 更新匹配命令 (2 小時)
**負責檔案**: `src/commands/match_command.rs`

**執行步驟**:
1. **替換第 12, 31 行的配置載入**
2. **更新匯入語句**
3. **測試匹配功能**

#### 任務 3.4: 更新同步命令 (2 小時)
**負責檔案**: `src/commands/sync_command.rs`

**執行步驟**:
1. **替換第 13 行的配置載入**
2. **更新匯入語句**
3. **測試同步功能**

#### 任務 3.5: 批次測試所有命令 (2 小時)
**執行步驟**:
```bash
# 測試所有主要命令
cargo run -- --help
cargo run -- config list
cargo run -- convert --help
cargo run -- match --help  
cargo run -- sync --help

# 確保所有命令都能正常啟動且無配置錯誤
```

### 第 4 階段: 核心模組整合 (0.5 天)
**目標**: 整合核心引擎模組使用新配置系統

#### 任務 4.1: 更新 matcher 引擎 (2 小時)
**負責檔案**: `src/core/matcher/engine.rs`

**執行步驟**:
1. **檢查配置使用情況**
   ```bash
   grep -n "Config::load" src/core/matcher/engine.rs
   ```

2. **替換配置載入** (如果存在)
3. **更新匯入語句**

#### 任務 4.2: 檢查其他核心模組 (2 小時)
**執行步驟**:
```bash
# 搜尋所有可能的配置載入
grep -r "Config::load" src/core/
grep -r "use crate::config::Config" src/core/

# 逐一修改發現的檔案
```

### 第 5 階段: 測試系統重建 (1 天)
**目標**: 建立完整的測試覆蓋和驗證

#### 任務 5.1: 重構配置測試 (4 小時)
**負責檔案**: `src/config.rs` (測試區塊)

**執行步驟**:
1. **移除舊測試**
   - 移除 `test_old_config_file_still_works` 測試
   - 移除其他與 `Config::load()` 相關的測試

2. **建立新測試** (參考上面的程式碼範例)

3. **執行配置測試**
   ```bash
   cargo test config::tests
   ```

#### 任務 5.2: 建立整合測試 (4 小時)
**負責檔案**: `tests/config_integration_tests.rs` (新檔案)

**執行步驟**:
1. **建立新測試檔案** (參考上面的程式碼範例)
2. **執行整合測試**
   ```bash
   cargo test config_integration
   ```

### 第 6 階段: 品質保證和文件 (0.5 天)
**目標**: 確保程式碼品質和完整性

#### 任務 6.1: 程式碼品質檢查 (2 小時)
**執行步驟**:
```bash
# 格式化程式碼
cargo fmt

# Clippy 靜態分析  
cargo clippy -- -D warnings

# 完整測試套件
cargo test

# 產生覆蓋率報告
cargo llvm-cov --all-features --workspace --html
```

#### 任務 6.2: 文件更新 (2 小時)
**負責檔案**: 
- `README.md`
- `CHANGELOG.md`  
- `src/config.rs` (文件註解)

**執行步驟**:
1. **更新 API 文件**
   ```rust
   /// 初始化全域配置管理器
   /// 
   /// 這個函數應該在應用程式啟動時呼叫一次。它會設定配置來源
   /// (檔案、環境變數、CLI 參數) 並載入初始配置。
   /// 
   /// # Errors
   /// 
   /// 當配置檔案格式錯誤或配置管理器已經初始化時返回錯誤。
   pub fn init_config_manager() -> crate::Result<()>
   ```

2. **更新 CHANGELOG.md**
   ```markdown
   ## [Unreleased]
   
   ### Changed
   - 重構配置系統，統一使用 ConfigManager 進行配置管理
   - 移除舊的 Config::load() 方法，替換為 load_config() 函數
   - 支援多來源配置合併（檔案、環境變數、CLI 參數）
   
   ### Added  
   - 新增配置動態監控功能
   - 新增配置快取機制提升效能
   - 新增統一的配置驗證邏輯
   ```
## 驗收標準

### 功能驗收標準

#### 基本功能
- [ ] 應用程式正常啟動和關閉
  ```bash
  cargo run -- --help  # 應該正常顯示說明
  ```
- [ ] 所有 CLI 命令功能完全正常
  ```bash
  cargo run -- config list      # 顯示所有配置
  cargo run -- convert --help   # 顯示轉換說明
  cargo run -- match --help     # 顯示匹配說明
  cargo run -- sync --help      # 顯示同步說明
  ```
- [ ] 配置檔案正確載入和解析
  ```bash
  # 建立測試配置檔案
  mkdir -p ~/.config/subx
  cat > ~/.config/subx/config.toml << EOF
  [ai]
  model = "gpt-4"
  [general]
  backup_enabled = true
  EOF
  
  cargo run -- config get ai.model  # 應該顯示 "gpt-4"
  ```
- [ ] 環境變數覆蓋機制正常運作
  ```bash
  export SUBX_AI_MODEL="gpt-3.5-turbo"
  cargo run -- config get ai.model  # 應該顯示 "gpt-3.5-turbo"
  unset SUBX_AI_MODEL
  ```
- [ ] 命令列參數具有最高優先權 (待 CLI 參數整合後測試)

#### 配置系統整合
- [ ] 全域配置管理器成功初始化
  - 應用程式啟動時無配置相關錯誤
  - 配置管理器只初始化一次
- [ ] 多來源配置合併邏輯正確
  - 環境變數能正確覆蓋檔案配置
  - 預設值在沒有其他配置時生效
- [ ] 配置驗證機制有效運作
  ```bash
  # 測試無效配置
  export SUBX_AI_PROVIDER="invalid-provider"
  cargo run -- config list  # 應該顯示配置錯誤
  unset SUBX_AI_PROVIDER
  ```
- [ ] 配置快取機制提升效能
  - 重複配置載入時間顯著減少
  - 配置變更後正確更新快取
- [ ] 配置載入錯誤提供清楚訊息
  ```bash
  # 測試格式錯誤的配置檔案
  echo "invalid-toml-content" > ~/.config/subx/config.toml
  cargo run -- config list  # 應該顯示清楚的錯誤訊息
  ```

#### 程式碼品質
- [ ] 所有 `Config::load()` 呼叫已移除
  ```bash
  grep -r "Config::load" src/ && echo "發現殘留的舊配置載入" || echo "✓ 舊配置載入已完全移除"
  ```
- [ ] 所有命令使用新的 `load_config()` 函數
  ```bash
  grep -r "load_config" src/commands/ | wc -l  # 應該有多個結果
  ```

### 品質標準

#### 程式碼品質
- [ ] 通過 `cargo clippy -- -D warnings` 檢查
  ```bash
  cargo clippy -- -D warnings
  ```
- [ ] 通過 `cargo fmt -- --check` 格式檢查
  ```bash
  cargo fmt -- --check
  ```
- [ ] 所有公開 API 都有文件註解
  ```bash
  # 檢查公開函數是否有文件
  grep -A 1 "pub fn" src/config.rs | grep "///"
  ```

#### 測試覆蓋率
- [ ] 配置相關程式碼覆蓋率 ≥ 80%
  ```bash
  cargo llvm-cov --all-features --workspace --html
  # 檢查 target/llvm-cov/html/index.html 中的覆蓋率報告
  ```
- [ ] 整合測試涵蓋所有主要功能
  ```bash
  cargo test config_integration
  ```
- [ ] 所有公開 API 都有對應測試
  ```bash
  cargo test config::tests
  ```

#### 效能標準
- [ ] 配置載入時間 < 100ms (冷啟動)
- [ ] 配置載入時間 < 10ms (熱快取)
- [ ] 記憶體使用無明顯增加

### 回歸測試

#### 核心功能回歸
- [ ] 現有配置檔案格式完全相容
  ```bash
  # 使用現有的配置檔案格式測試
  cargo run -- config get ai.provider  # 應該正常運作
  ```
- [ ] 所有現有 CLI 命令行為一致
- [ ] 配置檔案儲存功能正常
  ```bash
  cargo run -- config set ai.model gpt-4
  cargo run -- config get ai.model  # 應該顯示 "gpt-4"
  ```

#### 錯誤處理回歸
- [ ] 配置檔案不存在時使用預設值
- [ ] 配置格式錯誤時顯示清楚錯誤訊息
- [ ] 權限問題時顯示適當錯誤訊息

### 風險管理

#### 技術風險與緩解

**風險 1: 配置載入失敗導致應用程式無法啟動**
- **緩解措施**: 
  - 實作完善的錯誤處理和回退機制
  - 提供詳細的錯誤訊息和解決建議
  - 在配置載入失敗時使用預設配置

**風險 2: 效能衰退影響使用者體驗**
- **緩解措施**:
  - 實作配置快取機制
  - 建立效能基準測試
  - 監控配置載入時間

**風險 3: 配置格式不相容導致現有使用者受影響**
- **緩解措施**:
  - 維持現有配置檔案格式完全相容
  - 提供配置遷移指南
  - 實作配置檔案驗證和修復工具

#### 測試策略
```bash
# 完整的測試腳本
#!/bin/bash
set -e

echo "=== 開始統一配置系統驗收測試 ==="

# 1. 程式碼品質檢查
echo "1. 檢查程式碼品質..."
cargo fmt -- --check
cargo clippy -- -D warnings

# 2. 單元測試
echo "2. 執行單元測試..."
cargo test

# 3. 整合測試
echo "3. 執行整合測試..."
cargo test config_integration

# 4. 功能測試
echo "4. 執行功能測試..."
cargo run -- --help >/dev/null
cargo run -- config list >/dev/null

# 5. 覆蓋率檢查
echo "5. 產生覆蓋率報告..."
cargo llvm-cov --all-features --workspace --html

# 6. 回歸測試
echo "6. 執行回歸測試..."
cargo run -- config get ai.provider >/dev/null

echo "=== 所有測試通過！==="
```

## 定義完成 (DoD)

### 程式碼交付

#### 核心實作完成
- [ ] **移除舊配置系統**: `src/config.rs` 中的 `Config::load()` 方法已完全移除
- [ ] **新配置系統整合**: 所有程式碼使用新的 `load_config()` 函數
- [ ] **全域管理器建立**: `init_config_manager()` 和 `load_config()` 函數正常運作
- [ ] **配置來源完整**: 檔案、環境變數、CLI 參數來源全部實作

#### 檔案修改清單
**必須修改的檔案**:
- `src/config.rs` - 核心配置介面重構
- `src/config/partial.rs` - 加入 `to_complete_config()` 方法
- `src/config/source.rs` - 完成 `EnvSource` 和 `CliSource` 實作
- `src/main.rs` - 加入配置管理器初始化
- `src/commands/convert_command.rs` - 替換配置載入呼叫
- `src/commands/match_command.rs` - 替換配置載入呼叫
- `src/commands/sync_command.rs` - 替換配置載入呼叫
- `src/commands/config_command.rs` - 替換配置載入呼叫
- `src/core/matcher/engine.rs` - 更新配置載入 (如適用)

**新建立的檔案**:
- `tests/config_integration_tests.rs` - 整合測試

#### 程式碼品質檢查
- [ ] 通過 `cargo clippy -- -D warnings` 無任何警告
- [ ] 通過 `cargo fmt -- --check` 格式檢查
- [ ] 通過 `cargo check` 編譯檢查
- [ ] 通過 `cargo test` 所有測試

### 測試交付

#### 測試覆蓋率要求
- [ ] **單元測試**: 配置相關程式碼覆蓋率 ≥ 80%
  ```bash
  cargo llvm-cov --all-features --workspace --html
  # 檢查 target/llvm-cov/html/index.html
  ```
- [ ] **整合測試**: 端到端配置載入測試通過
  ```bash
  cargo test config_integration --verbose
  ```
- [ ] **回歸測試**: 所有現有功能測試通過
  ```bash
  cargo test --workspace
  ```

#### 效能基準
- [ ] **配置載入時間**: 冷啟動 < 100ms，熱快取 < 10ms
- [ ] **記憶體使用**: 配置快取記憶體使用 < 1MB
- [ ] **並發安全**: 多執行緒配置存取測試通過

### 文件交付

#### API 文件完整性
- [ ] **公開函數文件**: 所有 `pub fn` 都有 `///` 文件註解
  ```rust
  /// 初始化全域配置管理器
  /// 
  /// 這個函數應該在應用程式啟動時呼叫一次...
  pub fn init_config_manager() -> crate::Result<()>
  ```
- [ ] **模組文件**: 所有配置相關模組都有模組級文件
- [ ] **錯誤文件**: 所有錯誤情況都有清楚的文件說明

#### 使用者文件更新
- [ ] **README.md 更新**: 配置系統使用說明更新
- [ ] **CHANGELOG.md 更新**: 詳細記錄配置系統變更
  ```markdown
  ## [Unreleased]
  
  ### Changed
  - 統一配置管理系統，移除舊的 Config::load() 方法
  - 支援多來源配置合併（檔案、環境變數、CLI 參數）
  
  ### Added
  - 配置動態監控和自動重載功能
  - 配置快取機制提升載入效能
  ```

#### 技術文件建立
- [ ] **配置遷移指南**: 從舊系統到新系統的遷移說明
- [ ] **故障排除指南**: 常見配置問題和解決方案
- [ ] **開發者指南**: 新配置系統的使用方式和最佳實踐

### 驗證交付

#### 功能驗證完成
- [ ] **所有驗收標準通過**: 參考上面的驗收標準清單
- [ ] **使用者情境測試**: 典型使用者工作流程測試通過
  ```bash
  # 典型使用者情境
  1. 首次使用 (無配置檔案)
  2. 建立配置檔案
  3. 修改配置
  4. 環境變數覆蓋測試
  5. 配置錯誤處理測試
  ```
- [ ] **邊界條件測試**: 異常情況和邊界條件處理正確

#### 相容性驗證
- [ ] **向後相容性**: 現有配置檔案格式完全相容
- [ ] **API 相容性**: 外部 API 使用者不受影響 (如適用)
- [ ] **平台相容性**: 所有支援的作業系統平台測試通過

### 部署準備

#### Git 提交準備
- [ ] **提交歷史清理**: 合併相關的提交，確保提交歷史清晰
- [ ] **提交訊息規範**: 使用 Conventional Commits 格式
  ```
  feat(config): implement unified configuration management system
  
  - Remove legacy Config::load() method
  - Add ConfigManager-based global configuration
  - Support multi-source config loading (file, env, cli)
  - Add configuration caching and validation
  
  BREAKING CHANGE: Config::load() method removed, use load_config() instead
  
  Signed-off-by: 🤖 GitHub Copilot <github-copilot[bot]@users.noreply.github.com>
  ```

#### 發布準備
- [ ] **版本號更新**: 根據語意化版本更新版本號
- [ ] **發布說明準備**: 準備詳細的發布說明和遷移指南
- [ ] **向後相容性說明**: 明確標記破壞性變更和遷移路徑

### 最終檢查清單

#### 程式碼檢查
```bash
# 執行完整的程式碼檢查
#!/bin/bash
set -e

echo "=== 最終程式碼檢查 ==="

# 1. 清理並重新建置
cargo clean
cargo build --release

# 2. 程式碼品質
cargo fmt -- --check
cargo clippy -- -D warnings

# 3. 測試套件
cargo test --workspace

# 4. 覆蓋率報告
cargo llvm-cov --all-features --workspace --html

# 5. 文件產生
cargo doc --no-deps --workspace

echo "=== 所有檢查通過，準備交付 ==="
```

#### 交付確認
- [ ] **所有開發任務完成**: 參考實作計畫中的所有任務
- [ ] **所有測試通過**: 單元測試、整合測試、回歸測試
- [ ] **所有文件更新**: API 文件、使用者文件、技術文件
- [ ] **程式碼審查完成**: 至少一位其他開發者審查通過
- [ ] **效能驗證通過**: 符合所有效能基準要求

---

**完成標準**: 當且僅當上述所有項目都勾選完成時，此 Backlog 才算正式完成交付。

**交付物清單**:
1. ✅ 重構後的配置系統程式碼
2. ✅ 完整的測試套件 (單元 + 整合)
3. ✅ 更新的文件 (API + 使用者 + 技術)
4. ✅ 效能基準驗證報告
5. ✅ 向後相容性驗證報告
6. ✅ Git 提交和發布準備

---

## 總結

這個實作計畫將 SubX 的配置系統從雙軌並存升級為統一的 `ConfigManager` 架構，提供以下核心價值：

### 🎯 主要成果
1. **統一配置管理**: 移除重複的配置載入邏輯，建立單一配置管理架構
2. **多來源支援**: 支援檔案、環境變數、CLI 參數的分層配置載入
3. **動態監控**: 配置檔案變更時自動重新載入 (為未來功能做準備)
4. **效能最佳化**: 配置快取機制減少重複載入時間
5. **錯誤處理**: 統一且使用者友善的配置錯誤訊息

### 📋 實作重點
- **5 天工期** 分為 6 個階段，每階段都有明確的目標和可驗證的交付物
- **詳細程式碼範例** 確保實作者能準確理解每個修改點
- **完整測試策略** 包含單元測試、整合測試和回歸測試
- **嚴格品質控制** 通過 Clippy、格式檢查和覆蓋率要求

### 🔧 技術架構轉換
```
舊架構: Config::load() → TOML 解析 → Config
新架構: ConfigManager → 多來源載入 → PartialConfig → Config
```

### ✅ 交付保證
- **零破壞性**: 現有配置檔案格式完全相容
- **高品質**: 80% 測試覆蓋率 + 靜態分析零警告
- **完整文件**: API 文件 + 使用者指南 + 遷移指南

這個計畫為 SubX 的配置系統奠定了穩固的技術基礎，支援未來的擴展需求如配置熱重載、配置驗證增強、以及更複雜的配置來源整合。

**負責人**: 核心開發團隊  
**預計工期**: 5 個工作天  
**依賴項目**: Backlog #13 (統一配置管理系統)  
**里程碑**: 配置系統統一化完成


