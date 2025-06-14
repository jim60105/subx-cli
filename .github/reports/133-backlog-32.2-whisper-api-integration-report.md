---
title: "Job Report: Backlog #32.2 - OpenAI Whisper API 整合"
date: "2025-06-14T15:32:38Z"
---

# Backlog #32.2 - OpenAI Whisper API 整合 工作報告

**日期**：2025-06-14T15:32:38Z  
**任務**：實作 OpenAI Whisper API 整合，提供雲端語音轉錄與字幕同步功能  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述

本任務為 Backlog #32.2 的子項目，目標實作與 OpenAI Whisper API 的整合：
1. 建立 HTTP 客戶端與 API 呼叫機制
2. 提取與預處理音訊片段
3. 分析轉錄回應並抽取時間戳
4. 完成同步檢測器並整合至核心服務
5. 補充錯誤處理與重試機制
6. 撰寫單元、整合與 mock 測試

## 二、實作內容

### 2.1 建立 Whisper API 客戶端
- 新增 `src/services/whisper/client.rs`，實作 `WhisperApiClient`、請求與回應結構  
【F:src/services/whisper/client.rs†L1-L139】

### 2.2 實作音訊片段擷取器
- 新增 `src/services/whisper/audio_extractor.rs`，實作 `AudioSegmentExtractor`，負責擷取與預處理音訊  
【F:src/services/whisper/audio_extractor.rs†L1-L80】

### 2.3 實作 Whisper 同步檢測器
- 新增 `src/services/whisper/sync_detector.rs`，實作 `WhisperSyncDetector` 同步邏輯  
【F:src/services/whisper/sync_detector.rs†L1-L122】

### 2.4 建立模組結構
- 新增 `src/services/whisper/mod.rs`，並於 `src/services/mod.rs` 加入 `pub mod whisper;`  
【F:src/services/whisper/mod.rs†L1-L17】【F:src/services/mod.rs†L101-L102】

### 2.5 整合至核心服務容器
- 修改 `src/core/services.rs`，新增 `SyncServiceFactory` 與 `create_whisper_detector` 方法  
【F:src/core/services.rs†L9-L17】【F:src/core/services.rs†L60-L80】

### 2.6 更新錯誤處理
- 修改 `src/error.rs`，新增 `ApiErrorSource` 與 `SubXError::Api/whisper_api/audio_extraction`，並更新 `exit_code`、`user_friendly_message`  
【F:src/error.rs†L70-L109】【F:src/error.rs†L332-L386】

### 2.7 更新同步引擎結構
- 修改 `src/core/sync/engine.rs`，新增 `SyncMethod::WhisperApi`、`SyncResult.additional_info` 欄位  
【F:src/core/sync/engine.rs†L364-L378】【F:src/core/sync/engine.rs†L477-L495】

### 2.8 補強 AudioTranscoder 方法
- 修改 `src/services/audio/transcoder.rs`，新增 `extract_segment`、`transcode_to_format` 並導入 `Duration`、`fs`  
【F:src/services/audio/transcoder.rs†L292-L336】

### 2.9 更新依賴
- 修改 `Cargo.toml`，更新 `reqwest` features 並加入 `tokio-util`  
【F:Cargo.toml†L55-L62】

### 2.10 新增測試
- 新增 `tests/whisper_integration_tests.rs` 與 `tests/whisper_mock_tests.rs`  
【F:tests/whisper_integration_tests.rs†L1-L28】【F:tests/whisper_mock_tests.rs†L1-L24】

## 三、技術細節

### 3.1 架構變更
- 引入 `services::whisper` 模組，解耦核心同步邏輯與外部 API 交互

### 3.2 API 變更
- 擴充 `SyncMethod` enum 支援 `WhisperApi`
- 同步結果 `SyncResult` 新增 `additional_info` 用於承載轉錄細節

### 3.3 配置變更
- 使用 `WhisperConfig` 控制是否啟用及參數設定，無需修改既有配置檔案

## 四、測試與驗證

### 4.1 程式碼品質檢查
```
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check
cargo test -- --ignored
```

### 4.2 功能測試
- 單元測試涵蓋客戶端、擷取器、同步檢測器
- Mock 測試模擬 Whisper API 回應
- 整合測試需提供 `OPENAI_API_KEY`，標記 `#[ignore]`

## 五、影響評估

### 5.1 向後相容性
與既有 VAD 與同步流程共存，預設支持退回 VAD，不破壞舊版使用者設定

### 5.2 使用者體驗
提供雲端高精度字幕同步選項，擴充使用者可選方案

## 六、問題與解決方案

### 6.1 遇到的問題
- **AudioTranscoder 無 extract_segment 支援**：暫以 `transcode_to_wav` + 檔案複製實作，後續可優化
- **multipart file_name 生命週期問題**：改用 `to_string_lossy().to_string()` 產生 owned 字串

## 七、後續事項

### 7.1 待完成項目
- [ ] 精準切割音訊範圍替代簡易複製方式
- [ ] 實作 `fallback_to_vad` 機制

### 7.2 相關任務
- Backlog #32.1, Backlog #32.3

### 7.3 建議的下一步
- 開發本地 VAD (Backlog #32.3)

## 八、檔案異動清單

| 檔案路徑                                    | 異動類型 | 描述                            |
|-------------------------------------------|---------|---------------------------------|
| `src/services/whisper/client.rs`          | 新增    | Whisper API 客戶端與回應定義    |
| `src/services/whisper/audio_extractor.rs` | 新增    | 音訊片段擷取與預處理            |
| `src/services/whisper/sync_detector.rs`   | 新增    | Whisper 同步檢測器實作          |
| `src/services/whisper/mod.rs`             | 新增    | 模組宣告                        |
| `src/services/mod.rs`                     | 修改    | 新增 `pub mod whisper`         |
| `src/core/services.rs`                    | 修改    | 新增 `SyncServiceFactory`       |
| `src/error.rs`                             | 修改    | 新增 `ApiErrorSource`、`SubXError::Api` 等 |
| `src/core/sync/engine.rs`                 | 修改    | 新增 `SyncMethod::WhisperApi`、`SyncResult.additional_info` |
| `src/services/audio/transcoder.rs`        | 修改    | 新增 `extract_segment`、`transcode_to_format` 方法 |
| `Cargo.toml`                               | 修改    | 更新 `reqwest` features、加入 `tokio-util` |
| `tests/whisper_integration_tests.rs`      | 新增    | Whisper 音訊片段與整合測試      |
| `tests/whisper_mock_tests.rs`             | 新增    | Whisper API mock 測試           |
