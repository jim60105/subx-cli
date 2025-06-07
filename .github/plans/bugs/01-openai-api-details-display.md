# Bug Fix #01: OpenAI API 呼叫細節顯示

## 問題描述

當執行 `subx-cli match` 命令時，使用者無法看到 OpenAI API 呼叫的詳細資訊，包括：
- 使用的模型名稱
- Token 使用量統計（Prompt tokens、Completion tokens、Total tokens）

這會導致使用者無法了解 API 使用成本和效能情況。

## 問題分析

### 現狀分析
- 目前 `match` 命令會呼叫 AI 服務進行檔案匹配
- AI 服務回應包含 token 使用量資訊，但未向使用者顯示
- 缺乏透明度，使用者無法監控 API 使用情況

### 根本原因
- AI 服務整合層未傳遞詳細的回應資訊
- CLI 介面層未處理和顯示 API 統計資訊

## 技術方案

### 架構設計
1. **擴展 AI 服務回應結構**
   - 修改 `AiResponse` 結構體，增加模型和 token 統計欄位
   - 確保所有 AI 服務實作都返回完整資訊

2. **增強 CLI 輸出**
   - 在 match 命令執行過程中顯示 API 呼叫詳情
   - 使用結構化的輸出格式提升可讀性

### 資料結構設計
```rust
// 在 src/services/ai/mod.rs 中
pub struct AiUsageStats {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub struct AiResponse {
    pub content: String,
    pub usage: Option<AiUsageStats>,
}
```

## 實作步驟

### 第一階段：擴展 AI 服務層
1. **修改 AI 服務介面**
   - 檔案：`src/services/ai/mod.rs`
   - 更新 `AiResponse` 結構體
   - 增加 `AiUsageStats` 結構體

2. **更新 OpenAI 服務實作**
   - 檔案：`src/services/ai/openai.rs`
   - 解析 OpenAI API 回應中的 usage 欄位
   - 填充 `AiUsageStats` 資料

### 第二階段：增強檔案匹配器
1. **修改匹配器介面**
   - 檔案：`src/core/matcher/mod.rs`
   - 更新 `MatchResult` 結構體包含 AI 使用統計
   - 確保統計資訊正確傳遞

2. **更新匹配邏輯**
   - 檔案：`src/core/matcher/ai_matcher.rs`
   - 收集和聚合多次 AI 呼叫的統計資訊

### 第三階段：改善 CLI 輸出
1. **修改 match 命令**
   - 檔案：`src/commands/match_command.rs`
   - 增加 API 統計資訊的顯示邏輯
   - 設計美觀的輸出格式

2. **增強 UI 元件**
   - 檔案：`src/cli/ui.rs`
   - 建立統一的 API 統計顯示函式
   - 支援不同的輸出模式（詳細/簡潔）

## 詳細實作指南

### 步驟 1：修改 AI 服務結構體
```rust
// src/services/ai/mod.rs
#[derive(Debug, Clone)]
pub struct AiUsageStats {
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug)]
pub struct AiResponse {
    pub content: String,
    pub usage: Option<AiUsageStats>,
}
```

### 步驟 2：更新 OpenAI 服務
```rust
// src/services/ai/openai.rs
impl AiService for OpenAiService {
    async fn generate_response(&self, prompt: &str) -> Result<AiResponse> {
        // ... 現有的 API 呼叫邏輯 ...
        
        let usage = response.usage.map(|u| AiUsageStats {
            model: self.model.clone(),
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        
        Ok(AiResponse {
            content: response.choices[0].message.content.clone(),
            usage,
        })
    }
}
```

### 步驟 3：增強 CLI 輸出
```rust
// src/cli/ui.rs
pub fn display_ai_usage(usage: &AiUsageStats) {
    println!("🤖 AI API 呼叫詳情:");
    println!("   模型: {}", usage.model);
    println!("   Prompt tokens: {}", usage.prompt_tokens);
    println!("   Completion tokens: {}", usage.completion_tokens);
    println!("   Total tokens: {}", usage.total_tokens);
    println!();
}
```

## 測試計劃

### 單元測試
1. **AI 服務測試**
   - 測試 OpenAI 服務正確解析 usage 資訊
   - 測試 mock AI 服務返回正確的統計資料

2. **CLI 輸出測試**
   - 測試統計資訊的顯示格式
   - 測試不同場景下的輸出內容

### 整合測試
1. **端到端測試**
   - 執行實際的 match 命令
   - 驗證 API 統計資訊正確顯示

### 測試用例
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_usage_display() {
        let usage = AiUsageStats {
            model: "gpt-3.5-turbo".to_string(),
            prompt_tokens: 150,
            completion_tokens: 50,
            total_tokens: 200,
        };
        
        // 測試顯示函式
        display_ai_usage(&usage);
    }
}
```

## 品質保證

### 程式碼品質檢查
```bash
# 格式化程式碼
cargo fmt

# 靜態分析
cargo clippy -- -D warnings

# 執行測試
cargo test

# 程式碼覆蓋率
cargo llvm-cov --all-features --workspace --html
```

### 效能考量
- 統計資訊收集不應影響主要功能效能
- 輸出格式化應該快速且不阻塞

## 預期成果

### 功能改善
- 使用者可以清楚看到每次 AI API 呼叫的詳細資訊
- 提供模型名稱和 token 使用量統計
- 增加操作透明度和成本意識

### 輸出範例
```
🤖 正在呼叫 AI 服務進行檔案匹配...

🤖 AI API 呼叫詳情:
   模型: gpt-3.5-turbo
   Prompt tokens: 245
   Completion tokens: 78
   Total tokens: 323

✅ 檔案匹配完成
```

## 注意事項

### 相容性
- 確保新的統計功能不影響現有的 dry-run 模式
- 維持向後相容性

### 錯誤處理
- 當 AI 服務不提供統計資訊時，優雅地處理
- 不應因為統計資訊顯示失敗而影響主要功能

### 安全性
- 不記錄敏感的 API 金鑰資訊
- 統計資訊的儲存和顯示要安全

## 驗收標準

- [ ] AI 服務回應包含完整的使用統計資訊
- [ ] match 命令執行時顯示模型名稱和 token 統計
- [ ] 輸出格式美觀且易讀
- [ ] 所有測試通過
- [ ] 程式碼品質檢查無警告
- [ ] 不影響現有功能的正常運作
