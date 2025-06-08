# Product Backlog #15: OpenAI Base URL 設定功能

## 領域範圍
擴展 AI 服務設定，支援自訂 OpenAI API Base URL，以相容於 Azure OpenAI、私有部署服務和其他 OpenAI 相容 API。此實作將基於已建立的統一配置管理系統 (Backlog #14)。

## 前置需求
- ✅ Product Backlog #03: 配置管理系統（已完成）
- ✅ Product Backlog #05: AI 服務整合（已完成）
- ✅ Product Backlog #14: 統一配置管理系統整合（已完成）

## 完成項目

### 1. 配置結構擴展（基於統一配置系統）
- [ ] 在 `AIConfig` 和 `PartialAIConfig` 結構中新增 `base_url` 欄位
- [ ] 設定合理的預設值（保持向後相容性）
- [ ] 更新配置合併邏輯 (`PartialConfig::merge`)
- [ ] 更新配置轉換邏輯 (`PartialConfig::to_complete_config`)

### 2. 環境變數支援（統一配置來源）
- [ ] 在 `EnvSource` 中新增 `OPENAI_BASE_URL` 環境變數支援
- [ ] 維持統一配置系統的優先權順序
- [ ] 更新環境變數文件說明

### 3. OpenAI 客戶端重構
- [ ] 修改 `OpenAIClient::new` 建構子接受 `base_url` 參數
- [ ] 新增 `OpenAIClient::from_config` 靜態方法
- [ ] 更新 HTTP 請求邏輯移除硬編碼端點
- [ ] 保持現有 API 相容性

### 4. Config 命令整合（統一配置管理）
- [ ] 支援 `subx config set ai.base_url <url>` 指令
- [ ] 支援 `subx config get ai.base_url` 指令
- [ ] 在 `subx config list` 中顯示 base URL 設定
- [ ] 配置變更自動生效

### 5. 驗證與錯誤處理
- [ ] 在 `AIConfigValidator` 中新增 URL 格式驗證
- [ ] 連線測試功能
- [ ] 明確的錯誤訊息
- [ ] 降級處理機制

### 6. 測試覆蓋（統一配置架構）
- [ ] 單元測試：`PartialConfig` 合併邏輯
- [ ] 單元測試：環境變數來源載入
- [ ] 單元測試：URL 驗證器
- [ ] 整合測試：完整配置管理流程
- [ ] Mock 測試：多種 API 端點

## 技術設計

### 統一配置系統架構
基於 Backlog #14 實作的統一配置管理系統，`base_url` 配置將透過以下架構進行管理：

```
ConfigManager -> ConfigSource[] -> PartialConfig -> Config
    ↓              ↓                   ↓           ↓
  管理器        多來源載入          部分配置     完整配置
  (快取)       (檔案/環境/CLI)      (可選欄位)   (必填欄位)
```

### 配置結構擴展
```rust
// src/config/partial.rs
/// Partial AI configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialAIConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,  // 新增欄位
    pub max_sample_length: Option<usize>,
    pub temperature: Option<f32>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

// src/config.rs
/// AI 相關配置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: String,  // 新增欄位
    pub max_sample_length: usize,
    pub temperature: f32,
    pub retry_attempts: u32,
    pub retry_delay_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ai: AIConfig {
                provider: "openai".to_string(),
                api_key: None,
                model: "gpt-4o-mini".to_string(),
                base_url: "https://api.openai.com/v1".to_string(), // 預設值
                max_sample_length: 2000,
                temperature: 0.3,
                retry_attempts: 3,
                retry_delay_ms: 1000,
            },
            // ...其他配置
        }
    }
}
```

### 環境變數支援（統一配置來源）
```rust
// src/config/source.rs
impl ConfigSource for EnvSource {
    fn load(&self) -> Result<PartialConfig, ConfigError> {
        let mut config = PartialConfig::default();
        
        // 現有環境變數
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.ai.api_key = Some(api_key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            config.ai.model = Some(model);
        }
        if let Ok(provider) = std::env::var("SUBX_AI_PROVIDER") {
            config.ai.provider = Some(provider);
        }
        
        // 新增 base URL 環境變數支援
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            config.ai.base_url = Some(base_url);
        }
        
        // ...其他環境變數
        Ok(config)
    }
}
```

### PartialConfig 合併邏輯擴展
```rust
// src/config/partial.rs
impl PartialConfig {
    pub fn merge(&mut self, other: PartialConfig) -> Result<(), ConfigError> {
        // 現有合併邏輯
        if let Some(v) = other.ai.provider { self.ai.provider = Some(v); }
        if let Some(v) = other.ai.api_key { self.ai.api_key = Some(v); }
        if let Some(v) = other.ai.model { self.ai.model = Some(v); }
        
        // 新增 base_url 合併
        if let Some(v) = other.ai.base_url { self.ai.base_url = Some(v); }
        
        // ...其他欄位合併
        Ok(())
    }

    pub fn to_complete_config(&self) -> Result<Config, ConfigError> {
        let default = Config::default();
        
        let ai = AIConfig {
            provider: self.ai.provider.clone().unwrap_or(default.ai.provider),
            api_key: self.ai.api_key.clone().or(default.ai.api_key),
            model: self.ai.model.clone().unwrap_or(default.ai.model),
            base_url: self.ai.base_url.clone().unwrap_or(default.ai.base_url), // 新增轉換
            max_sample_length: self.ai.max_sample_length.unwrap_or(default.ai.max_sample_length),
            temperature: self.ai.temperature.unwrap_or(default.ai.temperature),
            retry_attempts: self.ai.retry_attempts.unwrap_or(default.ai.retry_attempts),
            retry_delay_ms: self.ai.retry_delay_ms.unwrap_or(default.ai.retry_delay_ms),
        };
        
        Ok(Config {
            ai,
            // ...其他配置區段
        })
    }
}
```

### OpenAI 客戶端重構
```rust
// src/services/ai/openai.rs
impl OpenAIClient {
    /// 現有建構子保持相容性
    pub fn new(
        api_key: String,
        model: String,
        temperature: f32,
        retry_attempts: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self::new_with_base_url(
            api_key,
            model,
            temperature,
            retry_attempts,
            retry_delay_ms,
            "https://api.openai.com/v1".to_string(),
        )
    }

    /// 新的建構子，支援自訂 base URL
    pub fn new_with_base_url(
        api_key: String,
        model: String,
        temperature: f32,
        retry_attempts: u32,
        retry_delay_ms: u64,
        base_url: String,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("建立 HTTP 客戶端失敗");
        Self {
            client,
            api_key,
            model,
            temperature,
            retry_attempts,
            retry_delay_ms,
            base_url: base_url.trim_end_matches('/').to_string(), // 移除尾隨斜線
        }
    }

    /// 從統一配置建立客戶端
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| crate::error::SubXError::config("缺少 OpenAI API Key"))?;
        
        // 驗證 base URL 格式
        Self::validate_base_url(&config.base_url)?;
        
        Ok(Self::new_with_base_url(
            api_key.clone(),
            config.model.clone(),
            config.temperature,
            config.retry_attempts,
            config.retry_delay_ms,
            config.base_url.clone(),
        ))
    }

    /// 驗證 base URL 格式
    fn validate_base_url(url: &str) -> crate::Result<()> {
        use url::Url;
        let parsed = Url::parse(url)
            .map_err(|e| crate::error::SubXError::config(format!("無效的 base URL: {}", e)))?;
        
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(crate::error::SubXError::config(
                "base URL 必須使用 http 或 https 協定".to_string()
            ));
        }
        
        if parsed.host().is_none() {
            return Err(crate::error::SubXError::config(
                "base URL 必須包含有效的主機名稱".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 配置驗證器擴展
```rust
// src/config/validator.rs
impl ConfigValidator for AIConfigValidator {
    fn validate(&self, config: &Config) -> Result<(), ConfigError> {
        // 現有驗證邏輯
        if let Some(ref api_key) = config.ai.api_key {
            if !api_key.starts_with("sk-") {
                return Err(ConfigError::InvalidValue(
                    "ai.api_key".to_string(),
                    "OpenAI API 金鑰必須以 'sk-' 開頭".to_string(),
                ));
            }
        }
        
        // 新增 base URL 驗證
        if let Err(e) = validate_base_url(&config.ai.base_url) {
            return Err(ConfigError::InvalidValue(
                "ai.base_url".to_string(),
                e.to_string(),
            ));
        }
        
        // ...其他驗證
        Ok(())
    }
}

fn validate_base_url(url: &str) -> Result<(), String> {
    use url::Url;
    let parsed = Url::parse(url)
        .map_err(|e| format!("無效的 URL 格式: {}", e))?;
    
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("base URL 必須使用 http 或 https 協定".to_string());
    }
    
    if parsed.host().is_none() {
        return Err("base URL 必須包含有效的主機名稱".to_string());
    }
    
    Ok(())
}
```

## 實作步驟

### 階段 1: 配置結構更新（估計：2-3 小時）
1. **更新 PartialAIConfig 結構**
   - 在 `src/config/partial.rs` 中的 `PartialAIConfig` 新增 `base_url: Option<String>` 欄位
   - 更新 `merge` 方法處理 `base_url` 欄位合併
   - 更新 `to_complete_config` 方法處理 `base_url` 轉換

2. **更新 AIConfig 結構**
   - 在 `src/config.rs` 中的 `AIConfig` 新增 `base_url: String` 欄位
   - 更新 `Default` 實作，設定預設值為 `"https://api.openai.com/v1"`
   - 確保所有相關的 `derive` 巨集正確處理新欄位

3. **執行測試驗證**
   - 執行現有測試確保無回歸
   - 驗證配置序列化/反序列化正常工作

### 階段 2: 環境變數支援（估計：1-2 小時）
1. **擴展 EnvSource 配置來源**
   - 在 `src/config/source.rs` 的 `EnvSource::load` 方法中新增 `OPENAI_BASE_URL` 環境變數讀取
   - 維持統一配置系統的優先權邏輯
   - 新增環境變數覆蓋測試

2. **更新單元測試**
   - 擴展 `tests/config_integration_tests.rs` 測試
   - 新增專門的 base URL 環境變數測試
   - 確保測試隔離性（避免環境變數污染）

### 階段 3: OpenAI 客戶端重構（估計：3-4 小時）
1. **重構建構子方法**
   - 保持現有 `OpenAIClient::new()` 向後相容
   - 新增 `new_with_base_url()` 方法接受 `base_url` 參數
   - 新增 `from_config()` 靜態方法使用統一配置

2. **更新 HTTP 請求邏輯**
   - 確保所有 API 請求使用可配置的 base URL
   - 移除硬編碼的 API 端點
   - 確保 URL 路徑正確拼接

3. **新增驗證功能**
   - 實作 `validate_base_url()` 函式
   - 支援 http/https 協定檢查
   - 驗證主機名稱格式

### 階段 4: 配置驗證器整合（估計：1-2 小時）
1. **擴展 AIConfigValidator**
   - 在 `src/config/validator.rs` 的 `AIConfigValidator::validate()` 中新增 base URL 驗證
   - 新增獨立的 `validate_base_url()` 函式
   - 確保驗證錯誤訊息清楚明確

2. **測試驗證器功能**
   - 測試有效和無效 URL 格式
   - 驗證錯誤處理邏輯
   - 確保驗證器與統一配置系統整合

### 階段 5: 命令列整合（估計：1-2 小時）
1. **更新配置命令**
   - 由於統一配置系統，`ai.base_url` 設定會自動支援
   - 測試 `subx config set ai.base_url <url>` 指令
   - 測試 `subx config get ai.base_url` 指令
   - 驗證配置更新後的持久化

2. **更新應用程式整合**
   - 更新 `match_command.rs` 使用 `OpenAIClient::from_config()`
   - 確保所有 AI 客戶端建立使用統一配置
   - 測試端到端功能

### 階段 6: 整合與測試（估計：2-3 小時）
1. **建立整合測試**
   - 測試統一配置管理流程
   - 測試環境變數覆蓋行為
   - 測試 OpenAI 客戶端建立流程

2. **Mock API 測試**
   - 建立多個 mock API 端點測試
   - 驗證不同 base URL 的請求路由
   - 測試錯誤情況處理

3. **文件更新**
   - 更新 README.md 環境變數說明
   - 更新配置檔案範例
   - 新增使用案例文件

## 驗證方式

### 功能驗證檢查清單
- [ ] **統一配置系統相容性**
  - 現有配置檔案可正常載入（向後相容）
  - 新配置檔案包含 base_url 設定
  - 統一配置管理器正確合併各來源設定

- [ ] **環境變數功能**
  - `OPENAI_BASE_URL` 正確覆蓋配置檔案設定
  - 環境變數優先權符合統一配置系統規則
  - 無效 URL 產生適當錯誤訊息

- [ ] **客戶端功能**
  - OpenAI 客戶端正確使用自訂 base URL
  - `from_config()` 方法正確從統一配置建立客戶端
  - API 請求路由至正確端點

- [ ] **統一配置命令**
  - Config 指令透過統一配置系統讀寫 base_url 設定
  - 無效 URL 設定被配置驗證器正確拒絕
  - 配置變更透過統一系統正確持久化

- [ ] **配置驗證器**
  - `AIConfigValidator` 正確驗證 base URL 格式
  - 錯誤訊息清楚且可操作
  - 驗證與統一配置系統無縫整合

### 測試案例範例
```bash
# 測試 1: 配置檔案設定（統一配置系統）
echo 'base_url = "https://api.azure.com/openai"' >> ~/.config/subx/config.toml
subx config get ai.base_url  # 應回傳 Azure URL

# 測試 2: 環境變數覆蓋（統一配置優先權）
export OPENAI_BASE_URL="https://custom-api.example.com/v1"
subx config get ai.base_url  # 應回傳自訂 URL

# 測試 3: 命令設定（統一配置管理）
subx config set ai.base_url "https://api.openai.com/v1"
subx config get ai.base_url  # 應回傳設定的 URL

# 測試 4: 錯誤處理（配置驗證器）
subx config set ai.base_url "invalid-url"  # 應產生錯誤
subx config set ai.base_url "ftp://invalid.com"  # 應產生錯誤

# 測試 5: 統一配置優先權驗證
# 設定檔案 base_url -> 環境變數覆蓋 -> 最終生效
```

### 統一配置系統整合測試
```rust
#[test]
fn test_base_url_unified_config_integration() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 建立測試配置檔案
    let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
base_url = "https://api.custom.com/v1"
"#;

    std::fs::write(&config_path, config_content).unwrap();
    env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
    env::set_var("OPENAI_BASE_URL", "https://env-override.com/v1");

    // 測試統一配置系統
    assert!(init_config_manager().is_ok());
    let config = load_config().unwrap();

    // 驗證環境變數覆蓋檔案設定
    assert_eq!(config.ai.base_url, "https://env-override.com/v1");

    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_BASE_URL");
}
```

## 效能考量

### 統一配置系統效能
- 利用現有的配置快取機制
- URL 驗證邏輯保持輕量
- 避免在配置載入時進行網路請求
- 快取驗證結果以提升重複使用效能

### 記憶體使用
- base_url 字串透過統一配置系統適當重用
- 避免不必要的字串複製
- 利用 `Arc<RwLock<PartialConfig>>` 進行配置共享

## 風險與挑戰

### 技術風險
1. **統一配置系統整合複雜性**
   - **風險**: 與現有統一配置系統整合可能產生非預期行為
   - **緩解**: 充分測試配置合併邏輯，確保優先權正確

2. **向後相容性**
   - **風險**: 現有使用者的配置檔案可能無法載入
   - **緩解**: 新欄位具有合理預設值，利用統一配置系統的容錯設計

3. **URL 驗證複雜性**
   - **風險**: URL 驗證邏輯過於嚴格或寬鬆
   - **緩解**: 使用成熟的 URL 解析函式庫，在配置驗證器中統一處理

4. **多來源配置衝突**
   - **風險**: 檔案、環境變數、CLI 參數間可能產生意外覆蓋
   - **緩解**: 遵循統一配置系統的優先權規則，提供清楚的配置來源說明

### 業務風險
1. **使用者體驗**
   - **風險**: 統一配置系統的複雜度可能困惑使用者
   - **緩解**: 提供清楚的預設值，詳細的錯誤訊息和文件

2. **支援負擔**
   - **風險**: 增加對不同 API 端點的支援需求
   - **緩解**: 明確文件說明支援範圍，利用統一配置系統提供一致的故障排除流程

## 相依性

### 前置需求
- ✅ Product Backlog #03: 配置管理系統（已完成）
- ✅ Product Backlog #05: AI 服務整合（已完成）
- ✅ Product Backlog #14: 統一配置管理系統整合（已完成）

### 後續影響
- 為未來支援其他 AI 提供商奠定基礎
- 支援企業級部署和私有雲環境
- 改善多環境開發工作流程
- 展示統一配置系統的擴展性

### 外部相依
- `url` crate：用於 URL 解析和驗證
- `serde` crate：配置序列化（統一配置系統已使用）
- `toml` crate：配置檔案格式（統一配置系統已使用）
- `reqwest` crate：HTTP 客戶端（現有相依）

## 驗收標準

### 必須達成
1. ✅ 現有功能無回歸，所有測試通過
2. ✅ 支援透過配置檔案設定 OpenAI base URL
3. ✅ 支援透過環境變數 `OPENAI_BASE_URL` 覆蓋設定
4. ✅ Config 命令可讀寫 `ai.base_url` 設定
5. ✅ 無效 URL 格式產生明確錯誤訊息
6. ✅ 文件更新包含新功能說明

### 品質要求
1. ✅ 程式碼覆蓋率不低於 80%
2. ✅ 通過 `cargo clippy` 檢查無警告
3. ✅ 通過 `cargo fmt` 格式檢查
4. ✅ 整合測試覆蓋主要使用案例
5. ✅ 效能測試確認無明顯回歸

### 使用者體驗
1. ✅ 設定過程直觀易懂
2. ✅ 錯誤訊息清楚且可操作
3. ✅ 向後相容性完全保持
4. ✅ 文件和範例完整充分

## 預估工作量
- **總計**: 11-16 小時
- **階段 1**: 2-3 小時（配置結構更新）
- **階段 2**: 1-2 小時（環境變數支援）
- **階段 3**: 3-4 小時（客戶端重構）
- **階段 4**: 2-3 小時（命令整合）
- **階段 5**: 3-4 小時（整合測試）

## 後續計劃
完成此 backlog 後，可考慮以下功能擴展：
1. 支援多個 AI 提供商配置
2. 實作連線測試與健康檢查功能
3. 新增 API 用量監控和成本追蹤
4. 支援進階 HTTP 設定（代理、逾時等）
