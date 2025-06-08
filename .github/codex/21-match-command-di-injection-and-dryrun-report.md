---
title: "Job Report: Backlog #07 - 依賴反轉 OpenAIClient 並修正 Dry-run 快取邏輯"
date: "2025-06-07T11:29:31Z"
---

# Backlog #07 - 依賴反轉 OpenAIClient 並修正 Dry-run 快取邏輯

**日期**：2025-06-07T11:29:31Z  
**任務**：將 OpenAIClient 由內部建立改為注入，以便測試；並調整 Dry-run 行為為執行內容分析與結果快取

## 一、依賴反轉（Dependency Injection）
- 提取 `execute_with_client` 函式，將 `OpenAIClient::new` 建立邏輯移至 `execute`，允許外部注入 `Box<dyn AIProvider>`【F:src/commands/match_command.rs†L10-L28】

## 二、修正 Dry-run 行為
- 永遠啟用內容分析 (`enable_content_analysis = true`)，確保 Dry-run 時仍執行 `match_files` 分析並快取結果【F:src/commands/match_command.rs†L30-L38】
- Dry-run 分支改為：先執行匹配運算，寫入快取，再跳過實際檔案操作【F:src/commands/match_command.rs†L39-L47】

## 三、更新單元測試
- 新增 `execute_with_client` 測試，使用 `DummyAI` 模擬 `AIProvider`，避免實際呼叫 AI 服務【F:src/commands/match_command.rs†L51-L69】
- 將 Dry-run 單元測試改為呼叫 `execute_with_client` 並驗證快取產生與原檔案保留【F:src/commands/match_command.rs†L70-L93】

## 四、驗證
```bash
cargo fmt && cargo clippy -- -D warnings && cargo test match_command -- --nocapture
```
```text
ℹ 預覽模式 - 未實際執行檔案操作
test commands::match_command::tests::dry_run_creates_cache_and_skips_execute_operations ... ok
```

```bash
cargo test -- --nocapture
# all tests passed
```
