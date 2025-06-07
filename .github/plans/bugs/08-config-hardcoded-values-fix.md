# Bug Report #08: 修復硬編碼配置值問題

## 問題描述

在配置檔案使用情況分析中發現，多個配置項目在程式碼中被硬編碼，導致使用者無法透過配置檔案自訂這些值。這違反了配置系統的設計原則，降低了應用程式的靈活性。

## 影響範圍

- AI 服務配置無法正確自訂
- 格式轉換行為無法透過配置控制
- 同步引擎參數被硬編碼覆蓋
- 使用者體驗不一致

## 受影響的配置項目

### 1. AI 配置硬編碼問題

#### `ai.temperature` 硬編碼
- **檔案位置**: `src/services/ai/openai.rs:138`
- **問題**: 硬編碼為 0.3，忽略配置檔案中的 temperature 設定
- **影響**: 使用者無法調整 AI 回應的隨機性

#### `ai.retry` 相關配置未整合
- **檔案位置**: AI 客戶端未使用 retry_attempts 和 retry_delay_ms
- **問題**: 重試功能模組存在但未整合到 OpenAI 客戶端
- **影響**: API 調用失敗時無法按配置重試

### 2. 格式配置硬編碼問題

#### `formats.preserve_styling` 硬編碼
- **檔案位置**: `src/commands/convert_command.rs:7`
- **問題**: 硬編碼為 true，忽略配置檔案設定
- **影響**: 使用者無法控制格式轉換時是否保留樣式

### 3. 同步配置硬編碼問題

#### `sync.max_offset_seconds` 被忽略
- **檔案位置**: `src/commands/sync_command.rs:13`
- **問題**: 使用命令列參數 args.range 而非配置值
- **影響**: 配置的預設偏移範圍無效

#### `sync.correlation_threshold` 不一致
- **檔案位置**: `src/commands/sync_command.rs:14`
- **問題**: 硬編碼為 0.3，配置預設值為 0.7
- **影響**: 實際行為與配置說明不符

## 修復計劃

### 階段 1: AI 配置修復 (預估工時: 4 小時)

#### 1.1 修復 temperature 硬編碼
```rust
// 修改 src/services/ai/openai.rs
// 目前程式碼 (第 138 行):
"temperature": 0.3,

// 修復後:
"temperature": self.temperature,

// 在 OpenAIClient struct 中加入 temperature 欄位
pub struct OpenAIClient {
    api_key: String,
    model: String,
    temperature: f32, // 新增
    client: reqwest::Client,
}

// 修改建構子接受 temperature 參數
impl OpenAIClient {
    pub fn new(api_key: String, model: String, temperature: f32) -> Self {
        // ...
    }
}
```

#### 1.2 整合重試配置
```rust
// 修改 src/commands/match_command.rs
let ai_client = OpenAIClient::new(
    api_key,
    config.ai.model.clone(),
    config.ai.temperature, // 新增
    config.ai.retry_attempts, // 新增
    config.ai.retry_delay_ms, // 新增
)?;
```

#### 1.3 實作重試邏輯
```rust
// 在 src/services/ai/openai.rs 中實作重試機制
impl OpenAIClient {
    async fn make_request_with_retry(&self, request: RequestBuilder) -> Result<Response> {
        let mut attempts = 0;
        loop {
            match request.try_clone().unwrap().send().await {
                Ok(response) => return Ok(response),
                Err(e) if attempts < self.retry_attempts => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(self.retry_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
```

### 階段 2: 格式配置修復 (預估工時: 2 小時)

#### 2.1 修復 preserve_styling 硬編碼
```rust
// 修改 src/commands/convert_command.rs
// 目前程式碼:
let preserve_styling = true; // 硬編碼

// 修復後:
let preserve_styling = config.formats.preserve_styling;
```

#### 2.2 整合 default_output 配置
```rust
// 在 convert_command.rs 中使用配置的預設輸出格式
let output_format = args.format.unwrap_or_else(|| {
    config.formats.default_output.clone()
});
```

### 階段 3: 同步配置修復 (預估工時: 3 小時)

#### 3.1 修復 max_offset_seconds 忽略問題
```rust
// 修改 src/commands/sync_command.rs
// 使用配置作為預設值，命令列參數為覆蓋值
let max_offset = args.range.unwrap_or(config.sync.max_offset_seconds);
```

#### 3.2 修復 correlation_threshold 不一致
```rust
// 修改 src/commands/sync_command.rs
// 目前程式碼:
let threshold = 0.3; // 硬編碼

// 修復後:
let threshold = args.threshold.unwrap_or(config.sync.correlation_threshold);
```

#### 3.3 更新 CLI 參數定義
```rust
// 在 src/cli/sync_args.rs 中加入可選參數
#[derive(Parser)]
pub struct SyncArgs {
    // ...existing fields...
    
    #[arg(long, help = "覆蓋配置檔案中的最大偏移秒數")]
    pub range: Option<f32>,
    
    #[arg(long, help = "覆蓋配置檔案中的相關性閾值")]
    pub threshold: Option<f32>,
}
```

## 測試計劃

### 單元測試 (預估工時: 3 小時)

#### 1. AI 配置測試
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_from_config() {
        let config = Config {
            ai: AIConfig {
                temperature: 0.8,
                // ...其他欄位...
            },
            // ...其他欄位...
        };
        
        let client = OpenAIClient::new(
            "test-key".to_string(),
            "gpt-4".to_string(),
            config.ai.temperature,
            config.ai.retry_attempts,
            config.ai.retry_delay_ms,
        );
        
        assert_eq!(client.temperature, 0.8);
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        // 測試重試機制是否按配置執行
    }
}
```

#### 2. 格式配置測試
```rust
#[test]
fn test_preserve_styling_from_config() {
    let config = Config {
        formats: FormatsConfig {
            preserve_styling: false,
            // ...其他欄位...
        },
        // ...其他欄位...
    };
    
    // 測試轉換命令是否使用配置值
}
```

#### 3. 同步配置測試
```rust
#[test]
fn test_sync_parameters_from_config() {
    let config = Config {
        sync: SyncConfig {
            max_offset_seconds: 45.0,
            correlation_threshold: 0.8,
            // ...其他欄位...
        },
        // ...其他欄位...
    };
    
    // 測試同步命令是否使用配置值
}
```

### 整合測試 (預估工時: 2 小時)

#### 1. 端到端配置測試
```bash
# 建立測試配置檔案
cat > test_config.toml << EOF
[ai]
temperature = 0.9
retry_attempts = 5

[formats]
preserve_styling = false

[sync]
max_offset_seconds = 60.0
correlation_threshold = 0.9
EOF

# 測試各命令是否正確讀取配置
subx --config test_config.toml match --dry-run test.srt test.mp4
subx --config test_config.toml convert --format vtt test.srt
subx --config test_config.toml sync test.srt test.wav
```

## 驗收標準

### 功能驗收
- [ ] AI 客戶端使用配置檔案中的 temperature 值
- [ ] AI 請求失敗時按配置進行重試
- [ ] 格式轉換時按配置決定是否保留樣式
- [ ] 同步命令使用配置中的預設參數
- [ ] 命令列參數能正確覆蓋配置值

### 程式碼品質驗收
- [ ] 所有硬編碼值已移除並替換為配置讀取
- [ ] 新增的程式碼通過 `cargo clippy` 檢查
- [ ] 程式碼格式化符合 `cargo fmt` 要求
- [ ] 單元測試覆蓋率達到 80% 以上

### 文件更新
- [ ] 更新配置檔案說明文件
- [ ] 更新命令列說明 (help 文字)
- [ ] 在 CHANGELOG.md 中記錄修復內容

## 風險評估

### 技術風險
- **中等風險**: 修改 OpenAI 客戶端可能影響現有功能
- **緩解措施**: 詳細的單元測試和整合測試

### 相容性風險
- **低風險**: 修復為內部實作變更，不影響 API 介面
- **緩解措施**: 保持命令列介面不變

### 回歸風險
- **中等風險**: 配置系統變更可能影響其他功能
- **緩解措施**: 完整的回歸測試套件

## 實作順序建議

1. **第一階段**: 修復 AI 配置 (最高優先級，影響核心功能)
2. **第二階段**: 修復格式配置 (中等優先級，影響使用者體驗)
3. **第三階段**: 修復同步配置 (較低優先級，功能性影響)

## 後續改進

此修復完成後，建議進行以下改進：
- 實作配置檔案熱重載功能
- 加入配置檔案驗證和錯誤提示
- 建立配置遷移機制以支援未來版本升級
