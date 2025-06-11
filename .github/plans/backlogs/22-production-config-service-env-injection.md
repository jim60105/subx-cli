# Backlog #22: ProductionConfigService 環境變數源注入重構

## 概述

目前的 `ProductionConfigService` 直接使用 `std::env::var()` 讀取 `OPENAI_API_KEY` 和 `OPENAI_BASE_URL` 環境變數，這使得針對這些特定環境變數載入邏輯的單元測試變得困難或不可能。雖然我們已透過 `TestConfigService` 和依賴注入模式成功測試了配置邏輯，但缺乏對環境變數載入本身的直接測試覆蓋。

本 backlog 旨在重構 `ProductionConfigService`，引入環境變數源的依賴注入機制，使其既能保持現有功能，又能支援完全隔離的單元測試，同時遵循 Backlog #21 建立的架構原則。

## 問題陳述

### 當前限制

1. **測試覆蓋缺口**：
   - 無法直接測試 `OPENAI_API_KEY` 環境變數載入邏輯
   - 無法直接測試 `OPENAI_BASE_URL` 環境變數載入邏輯
   - 無法測試環境變數優先權和回退機制

2. **架構一致性**：
   - 直接的 `std::env::var()` 調用破壞了依賴注入原則
   - 無法在不修改全域狀態的情況下進行單元測試

3. **維護挑戰**：
   - 環境變數處理邏輯與配置服務緊密耦合
   - 難以擴展支援新的環境變數源

### 業務需求

1. **測試要求**：能夠驗證 OPENAI 環境變數是否正確載入和處理
2. **安全要求**：測試必須完全隔離，不能修改全域環境狀態
3. **相容性要求**：重構不能破壞現有功能或 API
4. **擴展性要求**：設計應支援未來新增其他環境變數源

## 解決方案設計

### 核心概念

引入 **環境變數提供者** 抽象層，通過依賴注入將環境變數讀取邏輯與配置服務分離：

```rust
trait EnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String>;
}
```

### 架構設計

1. **環境變數提供者介面**：
   - `EnvironmentProvider` 特徵：抽象環境變數存取
   - `SystemEnvironmentProvider`：生產環境實作，使用 `std::env::var()`
   - `TestEnvironmentProvider`：測試環境實作，使用預設值映射

2. **配置服務重構**：
   - `ProductionConfigService` 接受 `EnvironmentProvider` 依賴
   - 保持現有 API 相容性
   - 新增支援環境變數提供者的構造方法

3. **測試基礎設施**：
   - 為環境變數邏輯建立專門的單元測試
   - 使用 `TestEnvironmentProvider` 注入模擬環境變數
   - 驗證環境變數優先權和回退邏輯

## 技術實作計劃

### 階段 1：環境變數提供者介面設計（1 天）

#### 1.1 設計 EnvironmentProvider 特徵

**目標**：建立環境變數存取的抽象介面

**實作內容**：
```rust
// src/config/environment.rs

/// 環境變數提供者特徵
/// 
/// 此特徵抽象了環境變數的存取，允許在測試中注入模擬實作
pub trait EnvironmentProvider: Send + Sync {
    /// 取得指定環境變數的值
    /// 
    /// # 參數
    /// * `key` - 環境變數名稱
    /// 
    /// # 回傳值
    /// 如果環境變數存在且有效，回傳 `Some(value)`，否則回傳 `None`
    fn get_var(&self, key: &str) -> Option<String>;
    
    /// 檢查環境變數是否存在
    /// 
    /// # 參數
    /// * `key` - 環境變數名稱
    /// 
    /// # 回傳值
    /// 如果環境變數存在，回傳 `true`，否則回傳 `false`
    fn has_var(&self, key: &str) -> bool {
        self.get_var(key).is_some()
    }
}
```

**品質檢查清單**：
- [ ] 特徵介面設計簡潔且易於理解
- [ ] 方法命名遵循 Rust 慣例
- [ ] 包含完整的文件註解
- [ ] Send + Sync 約束正確應用

#### 1.2 實作 SystemEnvironmentProvider

**目標**：建立生產環境的環境變數提供者

**實作內容**：
```rust
/// 系統環境變數提供者
/// 
/// 此實作直接讀取系統環境變數，用於生產環境
#[derive(Debug, Default)]
pub struct SystemEnvironmentProvider;

impl SystemEnvironmentProvider {
    /// 建立新的系統環境變數提供者
    pub fn new() -> Self {
        Self
    }
}

impl EnvironmentProvider for SystemEnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}
```

**品質檢查清單**：
- [ ] 實作正確包裝 `std::env::var()`
- [ ] 錯誤處理適當（將 `Result` 轉換為 `Option`）
- [ ] 結構體實作 `Debug` 以便除錯
- [ ] 提供預設建構方法

#### 1.3 實作 TestEnvironmentProvider

**目標**：建立測試環境的環境變數提供者

**實作內容**：
```rust
use std::collections::HashMap;

/// 測試環境變數提供者
/// 
/// 此實作使用預設的變數映射，用於測試環境的完全隔離
#[derive(Debug)]
pub struct TestEnvironmentProvider {
    variables: HashMap<String, String>,
}

impl TestEnvironmentProvider {
    /// 建立新的測試環境變數提供者
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
    
    /// 建立包含指定變數的測試提供者
    /// 
    /// # 參數
    /// * `variables` - 環境變數映射
    pub fn with_variables(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }
    
    /// 設定環境變數
    /// 
    /// # 參數
    /// * `key` - 環境變數名稱
    /// * `value` - 環境變數值
    pub fn set_var(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }
    
    /// 移除環境變數
    /// 
    /// # 參數
    /// * `key` - 環境變數名稱
    pub fn remove_var(&mut self, key: &str) {
        self.variables.remove(key);
    }
    
    /// 清除所有環境變數
    pub fn clear(&mut self) {
        self.variables.clear();
    }
}

impl EnvironmentProvider for TestEnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        self.variables.get(key).cloned()
    }
}

impl Default for TestEnvironmentProvider {
    fn default() -> Self {
        Self::new()
    }
}
```

**品質檢查清單**：
- [ ] 提供靈活的變數設定介面
- [ ] 支援動態修改環境變數映射
- [ ] 實作 `Default` 以便建立空的提供者
- [ ] 方法命名與系統 API 一致

### 階段 2：ProductionConfigService 重構（1.5 天）

#### 2.1 重構 ProductionConfigService 建構方法

**目標**：為 `ProductionConfigService` 新增環境變數提供者支援

**實作內容**：

1. **更新結構體定義**：
```rust
pub struct ProductionConfigService {
    config_builder: ConfigBuilder<DefaultState>,
    cached_config: Arc<RwLock<Option<Config>>>,
    env_provider: Arc<dyn EnvironmentProvider>,
}
```

2. **新增建構方法**：
```rust
impl ProductionConfigService {
    /// 使用預設環境變數提供者建立配置服務（現有方法保持相容）
    pub fn new() -> Result<Self> {
        Self::with_env_provider(Arc::new(SystemEnvironmentProvider::new()))
    }
    
    /// 使用指定環境變數提供者建立配置服務
    /// 
    /// # 參數
    /// * `env_provider` - 環境變數提供者
    pub fn with_env_provider(env_provider: Arc<dyn EnvironmentProvider>) -> Result<Self> {
        let config_builder = ConfigCrate::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::from(Self::user_config_path()).required(false))
            .add_source(Environment::with_prefix("SUBX").separator("_"));

        Ok(Self {
            config_builder,
            cached_config: Arc::new(RwLock::new(None)),
            env_provider,
        })
    }
}
```

**品質檢查清單**：
- [ ] 保持現有 API 完全向後相容
- [ ] 新方法遵循 Rust 命名慣例
- [ ] 錯誤處理保持一致
- [ ] 文件註解說明用途

#### 2.2 重構環境變數載入邏輯

**目標**：將直接的 `std::env::var()` 調用替換為環境變數提供者

**實作內容**：

**原始程式碼**：
```rust
// Special handling for OPENAI_API_KEY environment variable
if app_config.ai.api_key.is_none() {
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        debug!("ProductionConfigService: Found OPENAI_API_KEY environment variable");
        app_config.ai.api_key = Some(api_key);
    }
}

// Special handling for OPENAI_BASE_URL environment variable
if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
    debug!("ProductionConfigService: Found OPENAI_BASE_URL environment variable");
    app_config.ai.base_url = base_url;
}
```

**重構後的程式碼**：
```rust
// Special handling for OPENAI_API_KEY environment variable
// This provides backward compatibility with direct OPENAI_API_KEY usage
if app_config.ai.api_key.is_none() {
    if let Some(api_key) = self.env_provider.get_var("OPENAI_API_KEY") {
        debug!("ProductionConfigService: Found OPENAI_API_KEY environment variable");
        app_config.ai.api_key = Some(api_key);
    }
}

// Special handling for OPENAI_BASE_URL environment variable
if let Some(base_url) = self.env_provider.get_var("OPENAI_BASE_URL") {
    debug!("ProductionConfigService: Found OPENAI_BASE_URL environment variable");
    app_config.ai.base_url = base_url;
}
```

**品質檢查清單**：
- [ ] 所有 `std::env::var()` 調用已替換
- [ ] 邏輯流程保持不變
- [ ] 除錯訊息保持一致
- [ ] 錯誤處理正確（從 `Result` 改為 `Option`）

### 階段 3：測試基礎設施建立（1 天）

#### 3.1 環境變數提供者單元測試

**目標**：為環境變數提供者建立基礎單元測試

**實作內容**：
```rust
// src/config/environment.rs 的測試模組

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_environment_provider_existing_var() {
        // 測試系統環境變數提供者能正確讀取存在的環境變數
        let provider = SystemEnvironmentProvider::new();
        
        // 使用一個通常存在的環境變數進行測試
        let path = provider.get_var("PATH");
        assert!(path.is_some());
        assert!(!path.unwrap().is_empty());
    }

    #[test]
    fn test_system_environment_provider_non_existing_var() {
        // 測試系統環境變數提供者對不存在變數回傳 None
        let provider = SystemEnvironmentProvider::new();
        let result = provider.get_var("NON_EXISTING_VAR_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_test_environment_provider_empty() {
        // 測試空的測試環境變數提供者
        let provider = TestEnvironmentProvider::new();
        assert!(provider.get_var("ANY_VAR").is_none());
        assert!(!provider.has_var("ANY_VAR"));
    }

    #[test]
    fn test_test_environment_provider_with_variables() {
        // 測試預設變數的測試環境變數提供者
        let mut variables = HashMap::new();
        variables.insert("TEST_VAR".to_string(), "test_value".to_string());
        
        let provider = TestEnvironmentProvider::with_variables(variables);
        
        assert_eq!(provider.get_var("TEST_VAR"), Some("test_value".to_string()));
        assert!(provider.has_var("TEST_VAR"));
        assert!(provider.get_var("OTHER_VAR").is_none());
    }

    #[test]
    fn test_test_environment_provider_set_and_remove() {
        // 測試動態設定和移除變數
        let mut provider = TestEnvironmentProvider::new();
        
        // 設定變數
        provider.set_var("DYNAMIC_VAR", "dynamic_value");
        assert_eq!(provider.get_var("DYNAMIC_VAR"), Some("dynamic_value".to_string()));
        
        // 移除變數
        provider.remove_var("DYNAMIC_VAR");
        assert!(provider.get_var("DYNAMIC_VAR").is_none());
    }

    #[test]
    fn test_test_environment_provider_clear() {
        // 測試清除所有變數
        let mut provider = TestEnvironmentProvider::new();
        provider.set_var("VAR1", "value1");
        provider.set_var("VAR2", "value2");
        
        provider.clear();
        
        assert!(provider.get_var("VAR1").is_none());
        assert!(provider.get_var("VAR2").is_none());
    }
}
```

**品質檢查清單**：
- [ ] 測試覆蓋所有公開方法
- [ ] 測試正常情況和邊界情況
- [ ] 測試命名清晰且具描述性
- [ ] 測試完全隔離，無外部依賴

#### 3.2 ProductionConfigService 環境變數測試

**目標**：為 OPENAI 環境變數載入邏輯建立專門測試

**實作內容**：
```rust
// src/config/service.rs 測試模組中新增

#[test]
fn test_production_config_service_openai_api_key_loading() {
    // 測試 OPENAI_API_KEY 環境變數載入
    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("OPENAI_API_KEY", "sk-test-openai-key-env");
    
    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    
    let config = service.get_config().expect("Failed to get config");
    
    assert_eq!(config.ai.api_key, Some("sk-test-openai-key-env".to_string()));
}

#[test]
fn test_production_config_service_openai_base_url_loading() {
    // 測試 OPENAI_BASE_URL 環境變數載入
    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("OPENAI_BASE_URL", "https://test.openai.com/v1");
    
    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    
    let config = service.get_config().expect("Failed to get config");
    
    assert_eq!(config.ai.base_url, "https://test.openai.com/v1");
}

#[test]
fn test_production_config_service_both_openai_env_vars() {
    // 測試同時設定兩個 OPENAI 環境變數
    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("OPENAI_API_KEY", "sk-test-key-both");
    env_provider.set_var("OPENAI_BASE_URL", "https://both.openai.com/v1");
    
    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    
    let config = service.get_config().expect("Failed to get config");
    
    assert_eq!(config.ai.api_key, Some("sk-test-key-both".to_string()));
    assert_eq!(config.ai.base_url, "https://both.openai.com/v1");
}

#[test]
fn test_production_config_service_no_openai_env_vars() {
    // 測試沒有 OPENAI 環境變數的情況
    let env_provider = TestEnvironmentProvider::new(); // 空的提供者
    
    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    
    let config = service.get_config().expect("Failed to get config");
    
    // 應該使用預設值
    assert_eq!(config.ai.api_key, None);
    assert_eq!(config.ai.base_url, "https://api.openai.com/v1"); // 預設值
}

#[test]
fn test_production_config_service_api_key_priority() {
    // 測試 API key 優先權：如果已有 API key，不應覆蓋
    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("OPENAI_API_KEY", "sk-env-key");
    // 模擬從其他來源（如配置檔案）載入的 API key
    env_provider.set_var("SUBX_AI_APIKEY", "sk-config-key");
    
    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    
    let config = service.get_config().expect("Failed to get config");
    
    // SUBX_AI_APIKEY 應該有更高優先權（因為它先處理）
    // 這個測試可能需要根據實際優先權邏輯調整
    assert!(config.ai.api_key.is_some());
}
```

**品質檢查清單**：
- [ ] 測試覆蓋所有環境變數載入情境
- [ ] 測試優先權和回退邏輯
- [ ] 測試完全隔離，無全域狀態修改
- [ ] 斷言明確且有意義

### 階段 4：整合與驗證（0.5 天）

#### 4.1 模組整合

**目標**：確保新的環境變數模組正確整合到配置系統

**實作內容**：

1. **更新 mod.rs**：
```rust
// src/config/mod.rs

pub mod environment;
// ...現有模組...

pub use environment::{EnvironmentProvider, SystemEnvironmentProvider, TestEnvironmentProvider};
```

2. **更新 lib.rs**：
```rust
// src/lib.rs 中的 pub use 語句

pub use config::{
    // ...現有匯出...
    EnvironmentProvider, SystemEnvironmentProvider, TestEnvironmentProvider,
};
```

**品質檢查清單**：
- [ ] 所有公開介面正確匯出
- [ ] 模組結構清晰且一致
- [ ] 不破壞現有的匯入路徑

#### 4.2 向後相容性驗證

**目標**：確保重構不破壞現有功能

**驗證內容**：

1. **執行現有測試**：
```bash
cargo test config --lib
```

2. **執行整合測試**：
```bash
cargo test --test config_integration_tests
```

3. **檢查編譯相容性**：
```bash
cargo build --release
```

**品質檢查清單**：
- [ ] 所有現有測試通過
- [ ] 現有 API 調用方式仍然有效
- [ ] 編譯無警告
- [ ] 功能行為保持一致

#### 4.3 效能影響評估

**目標**：確保重構不引入顯著的效能回退

**評估內容**：

1. **配置載入效能**：比較重構前後的配置載入時間
2. **記憶體使用**：檢查新增依賴是否顯著增加記憶體佔用
3. **測試執行時間**：確保新測試不顯著延長測試套件執行時間

**品質檢查清單**：
- [ ] 配置載入時間變化 < 5%
- [ ] 記憶體使用增加 < 1MB
- [ ] 測試執行時間增加 < 10%

## 驗收標準

### 功能性需求

1. **環境變數載入**：
   - ✅ `OPENAI_API_KEY` 環境變數正確載入
   - ✅ `OPENAI_BASE_URL` 環境變數正確載入
   - ✅ 環境變數優先權邏輯正確運作

2. **測試覆蓋**：
   - ✅ 環境變數載入邏輯有專門的單元測試
   - ✅ 所有測試完全隔離，無全域狀態修改
   - ✅ 測試覆蓋率保持或提升

3. **API 相容性**：
   - ✅ 現有 `ProductionConfigService::new()` 方法繼續運作
   - ✅ 所有現有測試繼續通過
   - ✅ 公開 API 沒有破壞性變更

### 非功能性需求

1. **程式碼品質**：
   - ✅ 所有新程式碼通過 `cargo fmt` 檢查
   - ✅ 所有新程式碼通過 `cargo clippy -- -D warnings` 檢查
   - ✅ 測試覆蓋率 ≥ 85%

2. **文件品質**：
   - ✅ 所有公開介面有完整文件註解
   - ✅ 文件通過 `cargo doc` 檢查
   - ✅ 包含使用範例

3. **架構一致性**：
   - ✅ 遵循 Backlog #21 建立的依賴注入原則
   - ✅ 不使用 `unsafe` 程式碼
   - ✅ 不修改全域狀態

## 風險管理

### 技術風險

1. **複雜度增加**：
   - **風險**：新的抽象層可能增加系統複雜度
   - **緩解**：保持介面簡單，提供清晰的文件和範例

2. **效能影響**：
   - **風險**：額外的間接層可能影響效能
   - **緩解**：使用 `Arc` 避免不必要的複製，進行效能測試

3. **向後相容性**：
   - **風險**：重構可能破壞現有功能
   - **緩解**：保持現有 API，逐步驗證功能一致性

### 實作風險

1. **測試設計**：
   - **風險**：新測試可能無法正確覆蓋所有情境
   - **緩解**：詳細分析現有邏輯，設計全面的測試案例

2. **整合複雜性**：
   - **風險**：新模組與現有系統整合可能出現問題
   - **緩解**：分階段實作，每階段進行充分測試

## 後續任務

### 短期（1 週內）

1. **測試增強**：
   - 新增更多邊界情況測試
   - 驗證錯誤處理邏輯
   - 添加效能基準測試

2. **文件完善**：
   - 更新使用者指南
   - 新增架構設計文件
   - 提供遷移指南

### 中期（1 個月內）

1. **擴展支援**：
   - 考慮支援其他環境變數
   - 評估是否需要環境變數驗證功能
   - 探索配置熱重載支援

2. **生態系統整合**：
   - 檢視是否有其他元件可受益於類似模式
   - 考慮建立通用的環境變數管理庫
   - 評估與其他配置來源的整合

### 長期（3 個月內）

1. **架構演進**：
   - 評估是否需要更複雜的配置來源管理
   - 考慮支援動態配置變更
   - 探索配置版本控制需求

2. **效能最佳化**：
   - 分析配置載入效能瓶頸
   - 最佳化快取策略
   - 考慮非同步配置載入

## 成功指標

### 量化指標

1. **測試覆蓋率**：
   - 配置模組測試覆蓋率 ≥ 90%
   - 環境變數相關程式碼覆蓋率 = 100%

2. **效能指標**：
   - 配置載入時間增加 ≤ 5%
   - 記憶體使用增加 ≤ 1MB
   - 測試執行時間增加 ≤ 10%

3. **程式碼品質**：
   - Clippy 警告數 = 0
   - 文件覆蓋率 = 100%（公開 API）

### 質化指標

1. **開發體驗**：
   - 新環境變數測試易於編寫和理解
   - 錯誤訊息清晰且有幫助
   - API 使用直觀

2. **維護性**：
   - 程式碼結構清晰，職責分離明確
   - 新增環境變數支援簡單
   - 除錯和問題定位容易

3. **架構一致性**：
   - 與現有依賴注入模式一致
   - 符合 Rust 最佳實務
   - 遵循專案編碼標準

---

**預估總時間：4 天**
**優先權：中**
**複雜度：中等**
**前置需求：Backlog #21 完成**
**相關議題：#90（OPENAI 環境變數配置測試）**
