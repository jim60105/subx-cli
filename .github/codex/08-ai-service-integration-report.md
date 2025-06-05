---
title: "Job Report: Backlog #05 - AI 服務整合"
date: "2025-06-08"
---

# Backlog #05 - AI 服務整合 工作報告

**日期**：2025-06-08  
**任務**：OpenAI API 整合、AI 分析服務、錯誤處理、重試與快取機制實作

## 一、相依套件更新

- 在 `Cargo.toml` 新增 `async-trait` 相依以支援非同步 trait 宏  
  【F:Cargo.toml†L31-L34】

## 二、AI Provider Trait 與資料結構

- 在 `src/services/ai/mod.rs` 定義 `AIProvider` trait 與 `AnalysisRequest`、`ContentSample`、`VerificationRequest`、`MatchResult`、`FileMatch`、`ConfidenceScore` 等結構，並加入 `Clone` 派生以支援快取機制  
  【F:src/services/ai/mod.rs†L3-L62】

## 三、OpenAI 客戶端實作

- 新增 `src/services/ai/openai.rs`，實作 `OpenAIClient`，包含 HTTP 客戶端初始化、金鑰認證、Chat Completion 呼叫與錯誤包裝  
  【F:src/services/ai/openai.rs†L1-L62】【F:src/services/ai/openai.rs†L63-L85】

## 四、Prompt 模組實作

- 新增 `src/services/ai/prompts.rs`，撰寫 `build_analysis_prompt`、`parse_match_result`、`build_verification_prompt`、`parse_confidence_score` 等方法，統一 JSON 格式輸入輸出  
  【F:src/services/ai/prompts.rs†L1-L6】【F:src/services/ai/prompts.rs†L7-L56】【F:src/services/ai/prompts.rs†L58-L83】

## 五、重試機制實作

- 新增 `src/services/ai/retry.rs`，提供指數退避重試函式 `retry_with_backoff` 及預設參數 `RetryConfig`  
  【F:src/services/ai/retry.rs†L1-L9】【F:src/services/ai/retry.rs†L23-L31】【F:src/services/ai/retry.rs†L34-L53】

## 六、快取機制實作

- 新增 `src/services/ai/cache.rs`，實現基於 Hash 的記憶快取 `AICache`，並支援 TTL 過期策略  
  【F:src/services/ai/cache.rs†L1-L8】【F:src/services/ai/cache.rs†L28-L36】【F:src/services/ai/cache.rs†L52-L59】

## 七、驗證

- `cargo fmt` 無變動
- `cargo clippy -- -D warnings` 無警告
- `cargo build`、`cargo check` 全部通過

## 八、後續事項

- Backlog #06：檔案匹配引擎實作
