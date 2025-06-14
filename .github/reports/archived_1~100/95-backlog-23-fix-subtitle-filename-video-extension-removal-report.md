---
title: "Job Report: Backlog #23 - 修正字幕檔案重命名中的影片副檔名移除功能"
date: "2025-06-11T12:09:34Z"
---

# Backlog #23 - 修正字幕檔案重命名中的影片副檔名移除功能 工作報告

**日期**：2025-06-11T12:09:34Z  
**任務**：修正字幕檔案重命名邏輯以移除影片檔案副檔名  
**類型**：Backlog  
**狀態**：已完成

## 一、任務概述

Backlog #23 旨在解決 `match` 命令執行後，字幕檔案重命名包含原影片檔案副檔名的問題，確保最終字幕檔僅保留基礎檔名與字幕副檔名。

## 二、實作內容

### 2.1 更新 `generate_subtitle_name` 函式以移除影片副檔名
- 新增邏輯：先剔除影片檔名中的 `.extension`，再組合字幕檔名
- 檔案變更：【F:src/core/matcher/engine.rs†L452-L466】

```rust
// 從影片檔案名稱中移除副檔名（如果有）
let video_base_name = if !video.extension.is_empty() {
    video.name.strip_suffix(&format!(".{}", video.extension)).unwrap_or(&video.name)
} else {
    &video.name
};
```

### 2.2 新增單元測試覆蓋副檔名移除與邊界情況
- 測試移除 `.mkv` 副檔名、帶語言標籤以及檔名中多點/無副檔名情況
- 檔案變更：【F:src/core/matcher/engine.rs†L163-L240】

## 三、技術細節

### 3.1 相容性
- 僅影響字幕檔案命名邏輯，不改動其他匹配流程或 API，完全向後相容

## 四、測試與驗證

### 4.1 單元測試
```bash
cargo test test_generate_subtitle_name
```
結果：所有測試通過

### 4.2 程式碼品質檢查
```bash
cargo fmt && cargo clippy -- -D warnings
```
結果：無格式或警告

### 4.3 文件品質檢查
```bash
timeout 20 scripts/check_docs.sh
```
結果：文件檢查通過

### 4.4 覆蓋率檢查
```bash
scripts/check_coverage.sh -T
```
結果：覆蓋率符合標準

## 五、後續事項
- 無

---
**檔案異動**：
- src/core/matcher/engine.rs
- docs/tech-architecture.md
- README.zh-TW.md
