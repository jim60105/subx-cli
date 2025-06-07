# Product Backlog #15: OpenAI Base URL 設定功能

## 領域範圍
擴展 AI 服務設定，支援自訂 OpenAI API Base URL，以相容於 Azure OpenAI、私有部署服務和其他 OpenAI 相容 API

## 完成項目

### 1. 配置檔案結構擴展
- [ ] 在 `AIConfig` 結構中新增 `base_url` 欄位
- [ ] 設定合理的預設值（保持向後相容性）
- [ ] 更新配置檔案序列化/反序列化邏輯
- [ ] 擴展配置驗證規則

### 2. 環境變數支援
- [ ] 新增 `OPENAI_BASE_URL` 環境變數支援
- [ ] 更新 `apply_env_vars` 函式
- [ ] 維持環境變數優先權順序
- [ ] 新增環境變數文件說明

### 3. OpenAI 客戶端重構
- [ ] 修改 `OpenAIClient` 建構子接受可配置 base URL
- [ ] 移除硬編碼的 API 端點
- [ ] 更新 HTTP 請求邏輯
- [ ] 保持現有 API 相容性

### 4. Config 命令擴展
- [ ] 支援 `subx config set ai.base_url <url>` 指令
- [ ] 支援 `subx config get ai.base_url` 指令
- [ ] 在 `subx config list` 中顯示 base URL 設定
- [ ] 新增設定重置功能

### 5. 驗證與錯誤處理
- [ ] URL 格式驗證（schema、host 檢查）
- [ ] 連線測試功能
- [ ] 明確的錯誤訊息
- [ ] 降級處理機制

### 6. 測試覆蓋
- [ ] 單元測試：配置載入與儲存
- [ ] 單元測試：環境變數覆蓋
- [ ] 單元測試：URL 驗證邏輯
- [ ] 整合測試：完整工作流程
- [ ] Mock 測試：多種 API 端點

## 技術設計

### 配置結構擴展
```rust
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

### 環境變數支援
```rust
// src/config.rs
impl Config {
    fn apply_env_vars(&mut self) {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.ai.api_key = Some(key);
        }
        if let Ok(model) = std::env::var("SUBX_AI_MODEL") {
            self.ai.model = model;
        }
        // 新增 base URL 環境變數支援
        if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
            self.ai.base_url = base_url;
        }
    }
}
```

### OpenAI 客戶端重構
```rust
// src/services/ai/openai.rs
impl OpenAIClient {
    /// 建立新的 OpenAIClient，支援自訂 base URL
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("建立 HTTP 客戶端失敗");
        Self {
            client,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(), // 移除尾隨斜線
        }
    }

    /// 從配置建立客戶端
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        let api_key = config.api_key.as_ref()
            .ok_or_else(|| crate::error::SubXError::config("缺少 OpenAI API Key"))?;
        
        // 驗證 base URL 格式
        Self::validate_base_url(&config.base_url)?;
        
        Ok(Self::new(
            api_key.clone(),
            config.model.clone(),
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

### Config 命令擴展
```rust
// src/config.rs
impl Config {
    /// 擴展 get_value 方法支援 base_url
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
                "base_url" => Ok(self.ai.base_url.clone()), // 新增支援
                _ => Err(SubXError::config(format!("無效的 AI 配置鍵: {}", key))),
            },
            // ...其他配置區段
            _ => Err(SubXError::config(format!("無效的配置區段: {}", parts[0]))),
        }
    }

    /// 擴展 set_value 方法支援 base_url
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() != 2 {
            return Err(SubXError::config(format!("無效的配置鍵格式: {}", key)));
        }
        match parts[0] {
            "ai" => match parts[1] {
                "provider" => self.ai.provider = value.to_string(),
                "model" => self.ai.model = value.to_string(),
                "base_url" => {
                    // 驗證 URL 格式
                    crate::services::ai::openai::OpenAIClient::validate_base_url(value)?;
                    self.ai.base_url = value.to_string();
                },
                _ => return Err(SubXError::config(format!("無效的 AI 配置鍵: {}", key))),
            },
            // ...其他配置區段
            _ => return Err(SubXError::config(format!("無效的配置區段: {}", parts[0]))),
        }
        Ok(())
    }
}
```

## 實作步驟

### 階段 1: 配置結構更新（估計：2-3 小時）
1. **更新 AIConfig 結構**
   - 在 `src/config.rs` 中的 `AIConfig` 新增 `base_url: String` 欄位
   - 更新 `Default` 實作，設定預設值為 `"https://api.openai.com/v1"`
   - 確保所有相關的 `derive` 巨集正確處理新欄位

2. **更新配置驗證**
   - 在 `validate()` 方法中新增 base URL 格式檢查
   - 新增 URL 解析驗證邏輯
   - 確保錯誤訊息清楚明確

3. **執行測試驗證**
   - 執行現有測試確保無回歸
   - 驗證配置序列化/反序列化正常工作

### 階段 2: 環境變數支援（估計：1-2 小時）
1. **擴展 apply_env_vars 方法**
   - 新增 `OPENAI_BASE_URL` 環境變數讀取
   - 維持現有環境變數優先權邏輯
   - 新增環境變數覆蓋測試

2. **更新單元測試**
   - 擴展 `test_env_var_override` 測試
   - 新增專門的 base URL 環境變數測試
   - 確保測試隔離性（避免環境變數污染）

### 階段 3: OpenAI 客戶端重構（估計：3-4 小時）
1. **重構建構子**
   - 修改 `OpenAIClient::new()` 接受 `base_url` 參數
   - 新增 `from_config()` 靜態方法
   - 實作 URL 格式驗證函式

2. **更新 HTTP 請求邏輯**
   - 移除硬編碼的 API 端點
   - 使用可配置的 base URL 建構請求 URL
   - 確保 URL 路徑正確拼接

3. **新增驗證功能**
   - 實作 `validate_base_url()` 函式
   - 支援 http/https 協定檢查
   - 驗證主機名稱格式

### 階段 4: Config 命令整合（估計：2-3 小時）
1. **擴展配置存取方法**
   - 在 `get_value()` 中新增 `ai.base_url` 支援
   - 在 `set_value()` 中新增 `ai.base_url` 支援並驗證
   - 確保配置更新後的持久化

2. **測試命令功能**
   - 測試 `subx config set ai.base_url <url>` 指令
   - 測試 `subx config get ai.base_url` 指令
   - 驗證錯誤處理邏輯

### 階段 5: 整合與測試（估計：3-4 小時）
1. **建立整合測試**
   - 測試配置檔案載入與儲存
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
- [ ] **配置檔案相容性**
  - 現有配置檔案可正常載入（向後相容）
  - 新配置檔案包含 base_url 設定
  - 配置檔案格式正確且可讀

- [ ] **環境變數功能**
  - `OPENAI_BASE_URL` 正確覆蓋配置檔案設定
  - 環境變數優先權正確執行
  - 無效 URL 產生適當錯誤訊息

- [ ] **客戶端功能**
  - OpenAI 客戶端正確使用自訂 base URL
  - API 請求路由至正確端點
  - 錯誤處理機制正常運作

- [ ] **命令介面**
  - Config 指令正確讀寫 base_url 設定
  - 無效 URL 設定被正確拒絕
  - 配置變更正確持久化

### 測試案例範例
```bash
# 測試 1: 配置檔案設定
echo 'base_url = "https://api.azure.com/openai"' >> ~/.config/subx/config.toml
subx config get ai.base_url  # 應回傳 Azure URL

# 測試 2: 環境變數覆蓋
export OPENAI_BASE_URL="https://custom-api.example.com/v1"
subx config get ai.base_url  # 應回傳自訂 URL

# 測試 3: 命令設定
subx config set ai.base_url "https://api.openai.com/v1"
subx config get ai.base_url  # 應回傳設定的 URL

# 測試 4: 錯誤處理
subx config set ai.base_url "invalid-url"  # 應產生錯誤
subx config set ai.base_url "ftp://invalid.com"  # 應產生錯誤
```

## 效能考量

### 配置載入效能
- URL 驗證邏輯應保持輕量
- 避免在配置載入時進行網路請求
- 快取驗證結果以提升重複使用效能

### 記憶體使用
- base_url 字串應適當重用
- 避免不必要的字串複製
- 考慮使用 `Arc<str>` 進行共享參考

## 風險與挑戰

### 技術風險
1. **向後相容性**
   - **風險**: 現有使用者的配置檔案可能無法載入
   - **緩解**: 確保新欄位具有合理預設值，向後相容測試

2. **URL 驗證複雜性**
   - **風險**: URL 驗證邏輯過於嚴格或寬鬆
   - **緩解**: 使用成熟的 URL 解析函式庫，廣泛測試各種 URL 格式

3. **環境變數衝突**
   - **風險**: 不同系統間環境變數命名衝突
   - **緩解**: 使用明確的前綴，提供清楚的文件說明

### 業務風險
1. **使用者體驗**
   - **風險**: 新增複雜度可能困惑使用者
   - **緩解**: 提供清楚的預設值，詳細的錯誤訊息和文件

2. **支援負擔**
   - **風險**: 增加對不同 API 端點的支援需求
   - **緩解**: 明確文件說明支援範圍，提供故障排除指南

## 相依性

### 前置需求
- Product Backlog #03: 配置管理系統（已完成）
- Product Backlog #05: AI 服務整合（已完成）

### 後續影響
- 為未來支援其他 AI 提供商奠定基礎
- 支援企業級部署和私有雲環境
- 改善多環境開發工作流程

### 外部相依
- `url` crate：用於 URL 解析和驗證
- `serde` crate：配置序列化（現有相依）
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
