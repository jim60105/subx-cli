---
title: "Job Report: Backlog #27 - Wiremock 整合測試框架階段3"
date: "2025-06-12T11:49:39Z"
---

# Backlog #27 - Wiremock 整合測試框架階段3 工作報告

**日期**：2025-06-12T11:49:39Z  
**任務**：依據 Backlog #27 規劃，完成階段3：新增並行設定方法並補充 match command 進階整合測試  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.  
> (Do not include this tip in the final report)

## 一、任務概述

依據 Backlog #27 階段3需求，需補充並擴充先前整合測試：
- 新增平行處理設定介面於 TestConfigBuilder
- 補充平行匹配與信心閾值過濾的整合測試
- 補充 AI 服務錯誤處理的整合測試

## 二、實作內容

### 2.1 新增並行處理設定方法
- 在 TestConfigBuilder 中新增 `with_parallel_settings` 方法，用以設定最大工作者數與佇列大小，支援平行測試場景  
- 檔案變更：【F:src/config/builder.rs†L318-L323】

```rust
/// 設定並行處理工作者數量及隊列大小，用於整合測試
pub fn with_parallel_settings(mut self, max_workers: usize, queue_size: usize) -> Self {
    self.config.general.max_concurrent_jobs = max_workers;
    self.config.parallel.task_queue_size = queue_size;
    self
}
```

### 2.2 增加 AI 服務錯誤響應模擬方法
- 在 MockOpenAITestHelper 中新增 `setup_error_response` 方法，用於模擬不同錯誤碼與錯誤訊息  
- 檔案變更：【F:tests/common/mock_openai_helper.rs†L74-L85】

```rust
/// Setup an error response with given status code and error message.
pub async fn setup_error_response(&self, status: u16, error_message: &str) {
    let response_body = json!({
        "error": { "message": error_message }
    });
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer mock-api-key"))
        .respond_with(ResponseTemplate::new(status).set_body_json(response_body))
        .mount(&self.mock_server)
        .await;
}
```

### 2.3 新增進階整合測試
- 平行匹配測試與信心閾值過濾：`tests/match_engine_ai_integration_tests.rs`
- AI 錯誤處理測試：`tests/match_engine_error_handling_integration_tests.rs`
- 檔案變更：【F:tests/match_engine_ai_integration_tests.rs†L1-L154】【F:tests/match_engine_error_handling_integration_tests.rs†L1-L119】

## 三、技術細節

### 3.1 架構變更
- 測試架構新增平行設定與錯誤響應模擬，不影響核心邏輯

### 3.2 API 變更
- 無

### 3.3 配置變更
- 新增 TestConfigBuilder 平行設定介面

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
cargo test
```

### 4.2 功能測試
- 已執行平行匹配與信心閾值過濾整合測試，所有情境通過  
- 已執行 AI 錯誤處理整合測試，401/429/500 情境皆回傳錯誤

### 4.3 覆蓋率測試（如適用）
```bash
cargo llvm-cov --all-features --workspace --html
```

## 五、影響評估

### 5.1 向後相容性
- 介面為向後相容新增，不影響既有測試與生產邏輯

### 5.2 使用者體驗
- 無

## 六、問題與解決方案

### 6.1 遇到的問題
- 初次執行整合測試時，缺少平行設定導致測試流程不穩定  
**解決方案**：新增 `with_parallel_settings` 方法，自行配置 max_workers 與 queue_size

### 6.2 技術債務
- 待整合更多 AI 呼叫失敗情境模擬，以涵蓋更全面的錯誤碼

## 七、後續事項

### 7.1 待完成項目
- [ ] 整合 HTTP 連線超時模擬測試
- [ ] 擴充 parallel 設定之預設值測試

### 7.2 相關任務
- Backlog #27 (Wiremock 整合測試框架)

### 7.3 建議的下一步
- 建立更多 match command CLI 參數組合測試

## 八、檔案異動清單

| 檔案路徑                                                   | 異動類型 | 描述                     |
|----------------------------------------------------------|----------|--------------------------|
| `src/config/builder.rs`                                  | 修改     | 新增 `with_parallel_settings` 方法       |
| `tests/common/mock_openai_helper.rs`                     | 修改     | 新增 `setup_error_response` 方法       |
| `tests/match_engine_ai_integration_tests.rs`             | 新增     | 平行匹配與信心閾值整合測試檔案 |
| `tests/match_engine_error_handling_integration_tests.rs` | 新增     | AI 錯誤處理整合測試檔案       |
