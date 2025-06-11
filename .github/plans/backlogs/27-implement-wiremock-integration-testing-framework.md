# 實作 Wiremock 整合測試模擬框架

## 問題描述

目前 SubX 專案中調用 `match_command::execute()` 的整合測試依賴實際的 OpenAI 服務進行測試，這種設計存在以下問題：

1. **測試穩定性問題**：依賴外部 OpenAI API 服務，網路問題或服務中斷會導致測試失敗
2. **測試成本問題**：每次測試都會產生 OpenAI API 費用
3. **測試隔離問題**：無法完全控制 AI 回應內容，測試結果不可預測
4. **測試速度問題**：網路請求延遲影響測試執行速度
5. **違反測試原則**：測試應該是自包含的，不應依賴外部服務

## 解決方案概述

使用 wiremock crate 建立模擬 OpenAI API 服務的整合測試框架，確保：

- **完全測試隔離**：每個測試都有獨立的 mock server 實例
- **可預測的回應**：完全控制 AI 服務的回應內容
- **快速執行**：消除網路延遲，大幅改善測試速度
- **成本控制**：不產生任何 OpenAI API 費用
- **測試穩定性**：消除外部服務依賴帶來的不穩定性

## 技術設計

### 架構概覽

```
┌─────────────────────────────────────────────────────────┐
│                整合測試層                                 │
├─────────────────────────────────────────────────────────┤
│  MockOpenAITestHelper  │  WiremockAIClient              │
├─────────────────────────────────────────────────────────┤
│              Wiremock MockServer                        │
├─────────────────────────────────────────────────────────┤
│         TestConfigService (注入 Mock URL)               │
├─────────────────────────────────────────────────────────┤
│              match_command::execute()                   │
└─────────────────────────────────────────────────────────┘
```

### 核心元件設計

#### 1. MockOpenAITestHelper

負責管理 wiremock 伺服器生命週期和提供測試工具函數：

```rust
pub struct MockOpenAITestHelper {
    mock_server: MockServer,
    base_url: String,
}

impl MockOpenAITestHelper {
    pub async fn new() -> Self { /* ... */ }
    pub fn base_url(&self) -> &str { /* ... */ }
    
    // 模擬標準的 OpenAI 聊天完成回應
    pub async fn mock_chat_completion(&self, request_matcher: impl Match, response: MockChatCompletionResponse) { /* ... */ }
    
    // 為匹配命令設定典型的成功回應
    pub async fn setup_successful_match_response(&self, matches: Vec<MockFileMatch>) { /* ... */ }
    
    // 設定 API 錯誤回應（如 401, 429, 500 等）
    pub async fn setup_error_response(&self, status: u16, error_message: &str) { /* ... */ }
    
    // 設定延遲回應來測試重試邏輯
    pub async fn setup_delayed_response(&self, delay_ms: u64, response: MockChatCompletionResponse) { /* ... */ }
}
```

#### 2. Mock 資料結構

為了符合 OpenAI API 格式的標準化模擬資料：

```rust
#[derive(Debug, Clone)]
pub struct MockChatCompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<MockUsageStats>,
}

#[derive(Debug, Clone)]
pub struct MockUsageStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct MockFileMatch {
    pub video_file_id: String,
    pub subtitle_file_id: String,
    pub confidence: f32,
    pub match_factors: Vec<String>,
}
```

#### 3. 測試配置建構器擴展

擴展現有的 `TestConfigBuilder` 來支援 mock URL 注入：

```rust
impl TestConfigBuilder {
    /// 設定 AI 基礎 URL 為 mock server
    pub fn with_mock_ai_server(mut self, mock_url: &str) -> Self {
        self.config.ai.base_url = mock_url.to_string();
        self.config.ai.api_key = Some("mock-api-key".to_string());
        self
    }
    
    /// 為整合測試建立帶有 mock server 的配置服務
    pub async fn build_with_mock_server(self) -> (TestConfigService, MockOpenAITestHelper) {
        let mock_helper = MockOpenAITestHelper::new().await;
        let config_service = self
            .with_mock_ai_server(mock_helper.base_url())
            .build_service();
        (config_service, mock_helper)
    }
}
```

### 實作步驟

#### 階段 1：建立 Mock 框架基礎設施（預估 3-4 小時）

**1.1 建立 MockOpenAITestHelper**

檔案：`tests/common/mock_openai_helper.rs`

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header, body_json_schema};
use serde_json::{json, Value};

pub struct MockOpenAITestHelper {
    mock_server: MockServer,
}

impl MockOpenAITestHelper {
    pub async fn new() -> Self {
        let mock_server = MockServer::start().await;
        Self { mock_server }
    }
    
    pub fn base_url(&self) -> String {
        self.mock_server.uri()
    }
    
    pub async fn mock_chat_completion_success(&self, response_content: &str) {
        let response_body = json!({
            "choices": [
                {
                    "message": {
                        "content": response_content
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "model": "gpt-4o-mini"
        });
        
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }
}
```

**1.2 建立測試工具模組**

檔案：`tests/common/mod.rs`

```rust
pub mod mock_openai_helper;
pub mod test_data_generators;
pub mod integration_test_macros;

pub use mock_openai_helper::MockOpenAITestHelper;
```

**1.3 實作測試資料產生器**

檔案：`tests/common/test_data_generators.rs`

```rust
use serde_json::json;

pub struct MatchResponseGenerator;

impl MatchResponseGenerator {
    pub fn successful_single_match() -> String {
        json!({
            "matches": [
                {
                    "video_file_id": "file_video123",
                    "subtitle_file_id": "file_subtitle123", 
                    "confidence": 0.95,
                    "match_factors": ["filename_similarity", "content_correlation"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "High confidence match based on filename similarity and content analysis."
        }).to_string()
    }
    
    pub fn no_matches_found() -> String {
        json!({
            "matches": [],
            "confidence": 0.1,
            "reasoning": "No suitable matches found between video and subtitle files."
        }).to_string()
    }
    
    pub fn multiple_matches() -> String {
        json!({
            "matches": [
                {
                    "video_file_id": "file_video1",
                    "subtitle_file_id": "file_subtitle1",
                    "confidence": 0.92,
                    "match_factors": ["filename_similarity"]
                },
                {
                    "video_file_id": "file_video2", 
                    "subtitle_file_id": "file_subtitle2",
                    "confidence": 0.87,
                    "match_factors": ["content_correlation", "language_match"]
                }
            ],
            "confidence": 0.89,
            "reasoning": "Multiple high-confidence matches identified."
        }).to_string()
    }
}
```

#### 階段 2：整合測試重構（預估 4-5 小時）

**2.1 重構 match_copy_move_integration_tests.rs**

將現有測試改為使用 mock server：

```rust
use crate::common::MockOpenAITestHelper;
use crate::common::test_data_generators::MatchResponseGenerator;
use subx_cli::config::TestConfigBuilder;

#[tokio::test]
async fn test_match_copy_operation_with_mock_ai() {
    // 建立測試環境
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // 建立 mock AI 服務
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper.mock_chat_completion_success(
        &MatchResponseGenerator::successful_single_match()
    ).await;

    // 建立測試檔案
    create_test_files(&root);

    // 使用 mock server 的配置
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .build_service();

    // 執行匹配命令
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 50,
        recursive: true,
        backup: true,
        copy: true,
        move_files: false,
    };

    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok());

    // 驗證結果
    verify_copy_operation_results(&root);
}

fn create_test_files(root: &Path) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie1.mp4"), "fake video content").unwrap();
    fs::write(
        subtitle_dir.join("subtitle1.srt"),
        "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n",
    ).unwrap();
}

fn verify_copy_operation_results(root: &Path) {
    let video_dir = root.join("videos");
    let expected_copy = video_dir.join("movie1.srt");
    assert!(expected_copy.exists(), "Copy operation should create the expected file");
    
    let original_file = root.join("subtitles").join("subtitle1.srt");
    assert!(original_file.exists(), "Original file should still exist after copy");
}
```

**2.2 建立錯誤處理測試**

```rust
#[tokio::test]
async fn test_match_with_ai_service_error() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // 設定 mock server 回傳錯誤
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper.setup_error_response(401, "Invalid API key").await;
    
    create_test_files(&root);
    
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .build_service();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 50,
        recursive: true,
        backup: true,
        copy: false,
        move_files: false,
    };
    
    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_err());
    
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("401") || error_message.contains("Invalid API key"));
}
```

**2.3 建立重試邏輯測試**

```rust
#[tokio::test]
async fn test_match_with_retry_logic() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path(); 
    
    let mock_helper = MockOpenAITestHelper::new().await;
    
    // 前兩次請求失敗，第三次成功
    mock_helper.setup_retry_scenario(vec![
        (500, "Internal Server Error"),
        (502, "Bad Gateway"),
        (200, &MatchResponseGenerator::successful_single_match()),
    ]).await;
    
    create_test_files(&root);
    
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .with_ai_retry_settings(3, 100) // 3 次重試，100ms 延遲
        .build_service();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 50,
        recursive: true,
        backup: true,
        copy: false,
        move_files: false,
    };
    
    let start_time = std::time::Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    // 驗證重試邏輯有正確執行（應該花費一些時間）
    assert!(elapsed > std::time::Duration::from_millis(200));
}
```

#### 階段 3：高級測試場景（預估 3-4 小時）

**3.1 並行處理測試**

```rust
#[tokio::test]
async fn test_parallel_match_operations_with_mock() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // 建立多個檔案
    create_multiple_test_files(&root, 5); // 5 對影片+字幕檔案
    
    let mock_helper = MockOpenAITestHelper::new().await;
    mock_helper.mock_chat_completion_success(
        &MatchResponseGenerator::multiple_matches()
    ).await;
    
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .with_parallel_settings(4, 100) // 4 個並行工作者
        .build_service();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 50,
        recursive: true,
        backup: true,
        copy: true,
        move_files: false,
    };
    
    let start_time = std::time::Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    
    // 驗證並行處理效率（並行處理應該比序列處理快）
    assert!(elapsed < std::time::Duration::from_secs(10));
    
    // 驗證所有檔案都被正確處理
    verify_parallel_processing_results(&root);
}

fn create_multiple_test_files(root: &Path, count: usize) {
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();
    
    for i in 1..=count {
        fs::write(
            video_dir.join(format!("movie{}.mp4", i)),
            format!("fake video content {}", i)
        ).unwrap();
        
        fs::write(
            subtitle_dir.join(format!("subtitle{}.srt", i)),
            format!("1\n00:00:01,000 --> 00:00:02,000\nTest subtitle {}\n", i)
        ).unwrap();
    }
}
```

**3.2 信心度閾值測試**

```rust
#[tokio::test] 
async fn test_confidence_threshold_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    create_test_files(&root);
    
    let mock_helper = MockOpenAITestHelper::new().await;
    
    // 模擬低信心度回應
    let low_confidence_response = json!({
        "matches": [
            {
                "video_file_id": "file_video123",
                "subtitle_file_id": "file_subtitle123",
                "confidence": 0.45, // 低於閾值
                "match_factors": ["weak_filename_similarity"]
            }
        ],
        "confidence": 0.45,
        "reasoning": "Weak match with low confidence."
    }).to_string();
    
    mock_helper.mock_chat_completion_success(&low_confidence_response).await;
    
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .build_service();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        confidence: 80, // 高閾值，應該拒絕低信心度匹配
        recursive: true,
        backup: true,
        copy: true,
        move_files: false,
    };
    
    let result = match_command::execute(args, &config_service).await;
    assert!(result.is_ok());
    
    // 驗證沒有檔案被複製（因為信心度太低）
    let video_dir = root.join("videos");
    let expected_copy = video_dir.join("movie1.srt");
    assert!(!expected_copy.exists(), "Low confidence matches should be rejected");
}
```

#### 階段 4：效能和穩定性測試（預估 2-3 小時）

**4.1 載荷測試**

```rust
#[tokio::test]
async fn test_high_load_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    
    // 建立大量測試檔案
    create_multiple_test_files(&root, 50); // 50 對檔案
    
    let mock_helper = MockOpenAITestHelper::new().await;
    
    // 設定具有真實延遲的回應來模擬網路條件
    mock_helper.setup_delayed_response(
        100, // 100ms 延遲
        MockChatCompletionResponse {
            content: MatchResponseGenerator::multiple_matches(),
            model: "gpt-4o-mini".to_string(),
            usage: Some(MockUsageStats {
                prompt_tokens: 500,
                completion_tokens: 200,
                total_tokens: 700,
            }),
        }
    ).await;
    
    let config_service = TestConfigBuilder::new()
        .with_mock_ai_server(mock_helper.base_url())
        .with_parallel_settings(8, 200) // 高並行度設定
        .build_service();
    
    let args = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true, // 乾執行模式以避免實際檔案操作
        confidence: 50,
        recursive: true,
        backup: true,
        copy: true,
        move_files: false,
    };
    
    let start_time = std::time::Instant::now();
    let result = match_command::execute(args, &config_service).await;
    let elapsed = start_time.elapsed();
    
    assert!(result.is_ok());
    println!("高載荷測試完成時間: {:?}", elapsed);
    
    // 驗證在合理時間內完成（應小於 30 秒）
    assert!(elapsed < std::time::Duration::from_secs(30));
}
```

**4.2 記憶體洩漏測試**

```rust
#[tokio::test]
async fn test_memory_stability() {
    for iteration in 1..=10 {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        
        create_test_files(&root);
        
        let mock_helper = MockOpenAITestHelper::new().await;
        mock_helper.mock_chat_completion_success(
            &MatchResponseGenerator::successful_single_match()
        ).await;
        
        let config_service = TestConfigBuilder::new()
            .with_mock_ai_server(mock_helper.base_url())
            .build_service();
        
        let args = MatchArgs {
            path: root.to_path_buf(),
            dry_run: true,
            confidence: 50,
            recursive: true,
            backup: true,
            copy: false,
            move_files: false,
        };
        
        let result = match_command::execute(args, &config_service).await;
        assert!(result.is_ok(), "迭代 {} 失敗", iteration);
        
        // MockServer 會在 drop 時自動清理資源
        drop(mock_helper);
        drop(config_service);
        drop(temp_dir);
    }
}
```

#### 階段 5：測試工具和巨集（預估 2 小時）

**5.1 建立測試巨集**

檔案：`tests/common/integration_test_macros.rs`

```rust
/// 用於整合測試的便利巨集
#[macro_export]
macro_rules! test_with_mock_ai {
    ($test_name:ident, $response:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let root = temp_dir.path();
            
            let mock_helper = $crate::common::MockOpenAITestHelper::new().await;
            mock_helper.mock_chat_completion_success($response).await;
            
            let config_service = subx_cli::config::TestConfigBuilder::new()
                .with_mock_ai_server(mock_helper.base_url())
                .build_service();
            
            $test_body(root, config_service, mock_helper).await;
        }
    };
}

/// 建立帶有錯誤回應的測試
#[macro_export]
macro_rules! test_with_mock_ai_error {
    ($test_name:ident, $status:expr, $error_msg:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let root = temp_dir.path();
            
            let mock_helper = $crate::common::MockOpenAITestHelper::new().await;
            mock_helper.setup_error_response($status, $error_msg).await;
            
            let config_service = subx_cli::config::TestConfigBuilder::new()
                .with_mock_ai_server(mock_helper.base_url())
                .build_service();
            
            $test_body(root, config_service, mock_helper).await;
        }
    };
}
```

**5.2 使用巨集重寫測試**

```rust
use crate::test_with_mock_ai;
use crate::common::test_data_generators::MatchResponseGenerator;

test_with_mock_ai!(
    test_successful_copy_operation,
    &MatchResponseGenerator::successful_single_match(),
    |root, config_service, _mock_helper| async move {
        // 建立測試檔案
        create_test_files(&root);
        
        // 執行測試
        let args = MatchArgs {
            path: root.to_path_buf(),
            dry_run: false,
            confidence: 50,
            recursive: true,
            backup: true,
            copy: true,
            move_files: false,
        };
        
        let result = match_command::execute(args, &config_service).await;
        assert!(result.is_ok());
        
        // 驗證結果
        verify_copy_operation_results(&root);
    }
);
```

### 實作檢查清單

#### 必須完成項目

- [ ] **建立 MockOpenAITestHelper 基礎類別**
  - [ ] 實作 `new()` 方法建立 wiremock server
  - [ ] 實作 `base_url()` 方法取得 mock server URL
  - [ ] 實作 `mock_chat_completion_success()` 方法
  - [ ] 實作 `setup_error_response()` 方法
  - [ ] 實作 `setup_delayed_response()` 方法

- [ ] **建立測試資料產生器**
  - [ ] 實作 `MatchResponseGenerator::successful_single_match()`
  - [ ] 實作 `MatchResponseGenerator::no_matches_found()`
  - [ ] 實作 `MatchResponseGenerator::multiple_matches()`
  - [ ] 實作錯誤回應產生器

- [ ] **擴展 TestConfigBuilder**
  - [ ] 新增 `with_mock_ai_server()` 方法
  - [ ] 新增 `with_ai_retry_settings()` 方法
  - [ ] 新增 `build_with_mock_server()` 方法

- [ ] **重構現有整合測試**
  - [ ] 重構 `test_match_copy_operation()` 使用 mock
  - [ ] 重構 `test_match_move_operation()` 使用 mock
  - [ ] 重構 `test_match_copy_dry_run()` 使用 mock

- [ ] **新增錯誤處理測試**
  - [ ] API 認證錯誤測試 (401)
  - [ ] 服務限流測試 (429)
  - [ ] 伺服器錯誤測試 (500)
  - [ ] 網路逾時測試

- [ ] **新增高級測試場景**
  - [ ] 並行處理測試
  - [ ] 信心度閾值測試
  - [ ] 載荷測試
  - [ ] 記憶體穩定性測試

#### 選擇性改進項目

- [ ] **測試效能監控**
  - [ ] 新增測試執行時間監控
  - [ ] 新增記憶體使用量監控
  - [ ] 新增 mock server 回應時間設定

- [ ] **測試巨集系統**
  - [ ] 實作 `test_with_mock_ai!` 巨集
  - [ ] 實作 `test_with_mock_ai_error!` 巨集
  - [ ] 實作測試模板巨集

- [ ] **進階 Mock 功能**
  - [ ] 實作請求驗證功能
  - [ ] 實作請求計數功能
  - [ ] 實作回應序列功能（多次不同回應）

### 測試隔離確保措施

根據 wiremock 的 "Test isolation" 指導原則，我們必須確保：

#### 1. 每個測試獨立的 MockServer

```rust
// ✅ 正確做法：每個測試都有自己的 MockServer
#[tokio::test]
async fn test_individual_mock_server() {
    let mock_helper = MockOpenAITestHelper::new().await; // 獨立的 mock server
    // 測試邏輯...
    // MockServer 在函數結束時自動清理
}
```

#### 2. 避免共享 MockServer 實例

```rust
// ❌ 錯誤做法：在測試間共享 MockServer
static SHARED_MOCK_SERVER: Lazy<MockServer> = Lazy::new(|| {
    // 這會違反測試隔離原則
});

// ✅ 正確做法：每個測試建立自己的實例
#[tokio::test]
async fn test_isolated_mock() {
    let mock_helper = MockOpenAITestHelper::new().await;
    // 使用獨立的 mock server...
}
```

#### 3. 確保資源清理

```rust
impl Drop for MockOpenAITestHelper {
    fn drop(&mut self) {
        // wiremock::MockServer 會自動清理資源
        // 但如果需要額外清理邏輯，可以在這裡實作
    }
}
```

### 向後相容性保證

此改進完全向後相容：

1. **不修改現有 API**：所有現有的 `match_command::execute()` 介面保持不變
2. **不影響生產程式碼**：Mock 框架僅用於測試環境
3. **保持配置系統**：使用現有的 `TestConfigService` 依賴注入架構
4. **可選擇使用**：開發者可以選擇使用 mock 或真實 API 進行測試

### 預期效益

#### 量化效益

- **測試執行速度**：從平均 5-10 秒/測試 → 100-500ms/測試（90%+ 改善）
- **測試成本**：從每次 CI 執行 $0.1-1.0 → $0（完全消除）
- **測試穩定性**：從 80-90% 成功率 → 99%+ 成功率
- **網路依賴**：從 100% 依賴外部服務 → 0% 依賴

#### 定性效益

- **開發體驗改善**：開發者可在離線環境進行測試
- **CI/CD 可靠性**：消除因外部服務問題導致的構建失敗
- **測試覆蓋率提升**：可以測試各種錯誤場景和邊界條件
- **debug 便利性**：完全控制 AI 回應，便於除錯

### 風險評估和緩解策略

#### 低風險項目

- **Mock 不精確**
  - *緩解*：基於真實 OpenAI API 文檔建立 mock，定期更新
- **測試覆蓋不足**
  - *緩解*：保留部分端到端測試使用真實 API（標記為 `#[ignore]`）

#### 中風險項目

- **實作複雜度**
  - *緩解*：分階段實作，先覆蓋核心場景
- **維護成本**
  - *緩解*：建立完善的測試工具和文檔

### 完成標準

- [ ] 所有現有的 `match_command` 整合測試都改用 mock
- [ ] 新增至少 10 個不同場景的測試案例
- [ ] 測試執行時間減少 80% 以上
- [ ] 所有測試在離線環境下都能正常執行
- [ ] CI/CD 構建時間減少且更穩定
- [ ] 程式碼覆蓋率維持或提升
- [ ] 完整的文檔和使用範例

### 後續改進方向

1. **擴展到其他 AI 提供者**：為 Anthropic、Google 等提供者建立類似的 mock 框架
2. **效能基準測試**：建立自動化的效能回歸測試
3. **錯誤注入測試**：建立更複雜的錯誤場景模擬
4. **負載測試自動化**：定期執行載荷測試確保系統穩定性

## 總結

這個改進計劃將顯著提升 SubX 專案的測試品質和開發效率，通過使用 wiremock 建立完整的 AI 服務模擬框架，我們可以：

- 消除對外部服務的依賴
- 大幅提升測試執行速度
- 增強測試的可預測性和穩定性
- 降低測試成本和維護複雜度
- 遵循測試隔離的最佳實踐

實作完成後，開發者將擁有一個快速、可靠、經濟的整合測試環境，有助於持續提升程式碼品質和開發效率。
