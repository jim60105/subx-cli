---
title: "Job Report: Bug Fix #10 - Windows 平台路徑分隔符測試失敗"
date: "2025-06-09"
---

## Bug Fix #10: Windows 平台路徑分隔符測試失敗

**日期**：2025-06-09  
**任務**：在 FileInfo 中統一路徑格式為 Unix 風格分隔符，並修正相關測試以支援 Windows 平台。

## 一、核心變更

1. 在 `FileInfo::new` 中將 `\\` 替換為 `/` 並以 `/` 計算深度  
   【F:src/core/matcher/mod.rs†L38-L42】【F:src/core/matcher/mod.rs†L55-L56】

2. 新增深度路徑測試 `test_file_info_deep_path`  
   【F:src/core/matcher/mod.rs†L90-L109】

3. 移除 `engine.rs` 中未使用的 `LanguageSource` 引入  
   【F:src/core/matcher/engine.rs†L26-L28】

## 二、驗證

- `cargo fmt` 與 `cargo test` 在本地執行皆無誤，且 Windows 平台下路徑測試通過
