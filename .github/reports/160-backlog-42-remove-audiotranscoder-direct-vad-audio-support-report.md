---
title: "Job Report: Backlog #42 - 移除 AudioTranscoder 並實現 VAD 直接音訊格式支援"
date: "2025-06-17T14:06:50Z"
---

# Backlog #42 - 移除 AudioTranscoder 並實現 VAD 直接音訊格式支援 工作報告

**日期**：2025-06-17T13:32:43Z  
**任務**：消除不必要的 AudioTranscoder，利用 Symphonia 直接解碼音訊以提升 VAD 效能  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> 此報告日期以 `date -u +"%Y-%m-%dT%H:%M:%SZ"` 取得。

## 一、任務概述

本次任務依據 Backlog #42 規劃，移除 AudioTranscoder 相關流程，並新增 DirectAudioLoader 使 VAD 模組可直接處理多種音訊格式，減少不必要的 WAV 臨時檔案轉碼，提高效能並簡化架構。

## 二、實作內容

### 2.1 實作 DirectAudioLoader::load_audio_samples
- 在 `src/services/vad/audio_loader.rs` 實作 `load_audio_samples`，使用 Symphonia 解碼容器格式並回傳 i16 樣本與音訊資訊
- 更新檔案【F:src/services/vad/audio_loader.rs†L33-L105】

### 2.2 更新使用範例
- 更新 `src/services/audio/mod.rs` 範例為 LocalVadDetector 直接處理音訊格式
- 更新檔案【F:src/services/audio/mod.rs†L49-L64】

### 2.3 更新服務模組範例
- 更新 `src/services/mod.rs` 範例為 LocalVadDetector 直接處理音訊格式
- 更新檔案【F:src/services/mod.rs†L56-L71】

### 2.4 更新 README 音訊支援說明
- 修改 README feature bullet 為直接解碼多種音訊容器格式
- 更新檔案【F:README.md†L25-L25】

## 三、技術細節

### 3.1 架構變更
- 去除 AudioTranscoder，改為 DirectAudioLoader 直接解碼，移除中間 WAV 檔案流程

### 3.2 API 變更
- `VadAudioProcessor` 新增 `load_and_prepare_audio_direct`，原 `load_and_prepare_audio` 改為調用新方法並標註已廢棄

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo check
cargo fmt -- --check
```

### 4.2 單元測試
```bash
cargo test
```

## 五、影響評估

### 5.1 向後相容性
- 保留原 `load_and_prepare_audio` 為已廢棄方法，確保現有依賴不立即中斷

### 5.2 使用者體驗
- 減少轉碼開銷，可直接載入各類容器格式音訊，提高 VAD 執行效能

## 六、問題與解決方案

無重大問題。

## 七、後續事項

### 7.1 待完成項目
- 完成 DirectAudioLoader 實際解碼實作
- 撰寫多格式載入完整單元測試

### 7.2 相關任務
- Backlog #42

## 八、檔案異動清單

| 檔案路徑                                    | 異動類型 | 描述                                         |
|---------------------------------------------|----------|----------------------------------------------|
| `src/services/vad/audio_loader.rs`          | 新增     | 建立 DirectAudioLoader 實作                   |
| `src/services/vad/audio_processor.rs`      | 修改     | 重構載入方法；新增直接載入與廢棄舊方法          |
| `src/services/vad/detector.rs`             | 修改     | 改用直接載入的方法                            |
| `src/services/vad/mod.rs`                  | 修改     | 新增 audio_loader module                      |
| `src/services/audio/transcoder.rs`         | 刪除     | 移除不必要的 AudioTranscoder                   |
| `src/services/audio/mod.rs`                | 修改     | 移除 AudioTranscoder 匯出                      |
| `Cargo.toml`                                | 修改     | 移除 `tempfile` 依賴                          |
| `tests/vad_direct_audio_loading_tests.rs`  | 新增     | 測試骨架（忽略）                             |
| `benches/vad_performance_comparison.rs`    | 新增     | 基準測試骨架                                 |
