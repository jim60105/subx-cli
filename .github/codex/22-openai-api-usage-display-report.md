---
title: "Job Report: Bug #01 - 顯示 OpenAI API 呼叫細節"
date: "2025-06-11"
---

# Bug Fix #01 - OpenAI API 呼叫細節顯示

**日期**：2025-06-11  
**任務**：於 chat_completion 中解析並顯示 OpenAI API 呼叫的模型名稱與 token 使用統計

## 一、擴充 AI 服務結構
- 新增 `AiUsageStats` 與 `AiResponse` 結構以代表 API 使用統計與回應內容【F:src/services/ai/mod.rs†L68-L86】

## 二、實作 UI 顯示函式
- 在 `src/cli/ui.rs` 中新增 `display_ai_usage` 函式，統一呈現模型與 token 統計【F:src/cli/ui.rs†L34-L43】

## 三、於 chat_completion 解析並顯示使用統計
- 修改 `OpenAIClient::chat_completion`，於解析回應 JSON 後擷取 `usage` 欄位並呼叫 `display_ai_usage` 顯示詳情【F:src/services/ai/openai.rs†L137-L180】
- 同步更新 `src/cli/mod.rs`，將 `display_ai_usage` 重新匯出至 CLI 根空間【F:src/cli/mod.rs†L18】

## 四、驗證
```bash
cargo fmt && cargo clippy -- -D warnings && cargo test -- --nocapture
```
```text
🤖 AI API 呼叫詳情:
   模型: gpt-3.5-turbo
   Prompt tokens: 123
   Completion tokens: 45
   Total tokens: 168
test services::ai::openai::tests::test_chat_completion_success ... ok
```

## 五、結論
成功於每次 chat_completion 完成後顯示 OpenAI API 使用成本資訊，提高使用者透明度。
