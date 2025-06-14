---
title: "Job Report: Bug Fix #04 - 語言編碼檔名處理"
date: "2025-06-07T23:18:10Z"
---

# Bug Fix #04: 語言編碼檔名處理

**日期**：2025-06-07T23:18:10Z  
**任務**：在檔案匹配與重命名流程中，正確偵測路徑或檔名內的語言編碼（如 tc、sc、en），並將語言標記附加到新的字幕檔名上。

## 一、核心變更

### 1. 新增語言偵測模組
- 建立 `src/core/language.rs`，定義 `LanguageDetector`、`LanguageInfo`、`LanguageSource`。
- 支援從目錄名稱與檔名模式解析語言編碼，並可排序去重。
- 提供 `detect_from_path`、`detect_all_languages`、`get_primary_language` 等 API。

### 2. 整合至檔案資訊結構
- 在 `src/core/matcher/mod.rs` 的 `FileInfo` 中新增 `language: Option<LanguageInfo>` 欄位。
- 建構 `FileInfo::new` 時，自動偵測並儲存語言資訊。

### 3. 更新重命名邏輯
- 在 `src/core/matcher/engine.rs` 的 `generate_subtitle_name` 中，透過 `LanguageDetector` 決定主要語言編碼。
- 若偵測到語言，則輸出格式為 `<video>.<lang>.<ext>`；否則沿用 `<video>.<ext>`。

## 二、單元測試

- 在 `src/core/language.rs` 中新增語言偵測測試，涵蓋目錄、檔名與無語言三種情境。
- 在 `src/core/matcher/engine.rs` 中新增字幕檔名生成測試，驗證多種語言標記場景。

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 三、結論

此次修正完整支援路徑及檔名中的語言編碼處理，確保字幕重命名時保留正確語言標記，並透過單元測試驗證核心行為。整體變更符合既有架構，且所有測試皆已通過。
