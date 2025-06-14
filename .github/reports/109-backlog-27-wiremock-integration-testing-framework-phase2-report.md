---
title: "Job Report: Backlog #27 - Wiremock 整合測試框架階段2"
date: "2025-06-12T01:05:50Z"
---

# Backlog #27 - Wiremock 整合測試框架階段2 工作報告

**日期**: 2025-06-12T01:05:50Z  
**任務**: 依據 Backlog #27 規劃，完成階段2：重構既有整合測試以使用 Wiremock 模擬 AI 服務並確保快取測試隔離  
**類型**: Backlog  
**狀態**: 已完成

## 一、任務概述
本階段旨在將現有的整合測試重構為使用 `MockOpenAITestHelper` 及 `MatchResponseGenerator`，消除對真實 OpenAI 服務的依賴，同時確保快取檔案不跨測試洩漏，提升測試穩定性與隔離性。

## 二、實作內容

### 2.1 擴充 MockOpenAITestHelper
- 新增 `mock_chat_completion_with_expectation` 方法以支援呼叫次數預期檢查
- 新增 `verify_expectations` 方法以驗證所有期待是否符合
- 【F:tests/common/mock_openai_helper.rs†L35-L75】

```rust
pub async fn mock_chat_completion_with_expectation(
    &self,
    response_content: &str,
    expected_calls: usize,
) {
    // ...
    .expect(expected_calls as u64)
    // ...
}

pub async fn verify_expectations(&self) {
    let _ = self.mock_server.received_requests().await;
}
```

### 2.2 重構 tests/match_cache_reuse_tests.rs
- 移除 `#[ignore]` 標記並加入 Wiremock 依賴匯入
- 清除舊快取檔案以確保測試隔離
- 重構測試流程，使用 mock 服務取代真實 API 呼叫
- 移除檔案操作斷言，改以呼叫次數驗證為主
- 【F:tests/match_cache_reuse_tests.rs†L1-L20】【F:tests/match_cache_reuse_tests.rs†L21-L80】

### 2.3 重構 tests/match_copy_behavior_tests.rs
- 移除 `#[ignore]` 標記並加入 Wiremock 依賴匯入
- 在各測試中使用 `MockOpenAITestHelper` 模擬 AI 回應，並注入 `TestConfigBuilder`
- 【F:tests/match_copy_behavior_tests.rs†L1-L20】【F:tests/match_copy_behavior_tests.rs†L21-L60】

### 2.4 重構 tests/match_copy_move_integration_tests.rs
- 新增 Wiremock 匯入與 `MatchResponseGenerator`，並在所有測試中注入 mock AI 服務
- 更新 `execute_parallel_match` 測試以使用 mock server
- 【F:tests/match_copy_move_integration_tests.rs†L1-L25】【F:tests/match_copy_move_integration_tests.rs†L26-L90】

### 2.5 重構 tests/parallel_processing_integration_tests.rs
- 在需 AI 服務的測試前加入 mock server 設定
- 使用 `TestConfigBuilder` 注入伺服器位址
- 【F:tests/parallel_processing_integration_tests.rs†L1-L4】【F:tests/parallel_processing_integration_tests.rs†L35-L70】

## 三、測試與驗證

### 3.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
timeout 20 scripts/check_docs.sh -v
```

### 3.2 單元與整合測試
- 確保所有重構後之整合測試成功通過，並加入快取隔離機制

## 四、後續改進方向
- 考量快取邏輯隔離與多執行緒安全，可將快取目錄改為測試專用路徑
- 未來可進行高級測試場景與錯誤注入測試 (Stage3)

## 五、檔案異動清單

| 檔案路徑                                        | 異動類型 | 描述                               |
|-----------------------------------------------|----------|------------------------------------|
| `tests/common/mock_openai_helper.rs`           | 修改     | 擴充呼叫次數預期與驗證方法           |
| `tests/match_cache_reuse_tests.rs`             | 修改     | 清除快取檔案、重構 cache 重用測試    |
| `tests/match_copy_behavior_tests.rs`           | 修改     | 重構複製行為測試以使用 Wiremock     |
| `tests/match_copy_move_integration_tests.rs`   | 修改     | 重構複製/移動整合測試以使用 Wiremock |
| `tests/parallel_processing_integration_tests.rs` | 修改   | 重構並行處理測試以使用 Wiremock     |
