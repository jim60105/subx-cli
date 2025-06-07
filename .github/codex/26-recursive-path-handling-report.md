---
title: "Job Report: Bug Fix #03 - 遞迴模式下的路徑處理優化"
date: "2025-06-07"
---

# Bug Fix #03: 遞迴模式下的路徑處理優化

**日期**：2025-06-07  
**任務**：於遞迴匹配模式中，增強 LLM 提示的路徑上下文，並新增 FileInfo 結構以擷取相對路徑與目錄資訊。

## 一、新增 FileInfo 結構
- 在 `src/core/matcher/mod.rs` 新增 `FileInfo`，記錄 `name`、`relative_path`、`full_path`、`directory`、`depth`。
- 提供 `FileInfo::new` 方法，計算相對路徑與目錄深度。

## 二、改進 AI 提示生成
- 修改 `src/core/matcher/engine.rs` 中 `match_files`，在 `AnalysisRequest` 的 `video_files` 及 `subtitle_files` 欄位，將檔案名稱替換為 `"檔名 (路徑: 相對路徑, 目錄: 目錄名稱)"` 形式，提升 LLM 對多層目錄結構的辨識能力。

## 三、單元測試
- 新增 `FileInfo` 的單元測試，驗證相對路徑、目錄名稱與深度計算。
- 確認原有測試均通過。

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test -- --nocapture
```

## 四、結論
本次修正已在遞迴匹配時提供更完整的路徑上下文，有助於增強 LLM 檔案匹配準確度；同時保留原有行為與測試通過，具備向下相容性。
