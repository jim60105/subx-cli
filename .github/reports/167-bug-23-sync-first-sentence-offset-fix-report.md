---
title: "Job Report: Bug Fix #23 - 修正 Sync 首句時間偏移計算錯誤"
date: "2025-06-18T03:24:30Z"
---

# Bug Fix #23 - 修正 Sync 首句時間偏移計算錯誤 工作報告

**日期**：2025-06-18T03:24:30Z  
**任務**：修正字幕同步命令對第一句負偏移處理錯誤，並新增相關測試以驗證正確性

## 一、實作內容

### 1.1 修正負偏移應用邏輯
- 修正 `SyncEngine::apply_manual_offset` 負偏移邏輯，避免誤將負偏移當作正偏移套用【F:src/core/sync/engine.rs†L132-L150】

### 1.2 新增單元測試覆蓋負偏移情境
- 新增 `test_manual_offset_negative_application` 驗證負偏移正確套用【F:src/core/sync/engine.rs†L301-L314】

### 1.3 新增整合測試驗證首句對齊
- 新增 `test_sync_first_sentence_with_assets`，使用 assets 樣本檔案驗證首句字幕同步行為【F:tests/sync_first_sentence_offset_integration_tests.rs†L1-L56】

## 二、驗證
```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```
結果：通過

## 三、後續事項
- 無

---
**檔案異動**：
- src/core/sync/engine.rs
- tests/sync_first_sentence_offset_integration_tests.rs
