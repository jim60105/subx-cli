---
title: "Job Report: Backlog #12.2 - 單元測試與程式碼覆蓋率 核心模組測試"
date: "2025-06-07"
---

# Backlog #12 (續) - 單元測試與程式碼覆蓋率 核心模組工作報告

**日期**：2025-06-07  
**任務**：為錯誤處理、配置管理、SRT 引擎、AI 服務與檔案匹配等核心模組補齊單元測試。

## 一、錯誤處理模組測試
- 新增 `SubXError::ai_service` 建構函式，並改進 `exit_code()` 對 FileMatching 返回碼之對應【F:src/error.rs†L90-L100】【F:src/error.rs†L102-L176】
- 完整測試各錯誤類型之建構、`to_string()`、退出碼、與使用者友善訊息【F:src/error.rs†L1-L76】

## 二、配置管理模組測試
- 驗證預設配置、TOML 序列化/反序列化、環境變數覆蓋（隔離測試環境）、配置驗證、檔案存取與合併邏輯【F:src/config.rs†L135-L230】

## 三、字幕格式解析引擎 (SRT) 測試
- SRT 基本解析與序列化往返、格式偵測、多條目、空/錯誤區塊、邊界時間值、格式名稱與副檔名【F:src/core/formats/srt.rs†L137-L227】

## 四、AI 服務整合模擬測試
- 派生 `PartialEq`/`Eq` 於 `AnalysisRequest`、`ContentSample`、`MatchResult`、`FileMatch`、`ConfidenceScore`、`VerificationRequest`，以支援 mockall【F:src/services/ai/mod.rs†L25-L84】
- 測試 `OpenAIClient` 建構、chat_completion 成功/失敗、分析與驗證 Prompt 建置、JSON 結果解析【F:src/services/ai/openai.rs†L190-L265】【F:src/services/ai/prompts.rs†L51-L90】

## 五、檔案匹配引擎測試
- 非遞迴/遞迴掃描、檔案分類、副檔名支援、空目錄與不存在路徑處理測試【F:src/core/matcher/discovery.rs†L89-L147】

## 六、執行結果
```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 七、後續規劃
- 擴充格式轉換、音訊同步、CLI 等其餘核心模組之單元與整合測試  
- 驗證整體與核心模組覆蓋率達標，並持續優化 CI 覆蓋率門檻

以上完成核心模組單元測試，下一步將綜合驗證覆蓋率並提升測試品質。
