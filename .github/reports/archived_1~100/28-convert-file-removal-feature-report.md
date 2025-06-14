---
title: "Job Report: Bug Fix #05 - Convert 命令檔案移除功能"
date: "2025-06-08T07:59:19Z"
---

## Bug Fix #05: Convert 命令檔案移除功能

**日期**：2025-06-08T07:59:19Z  
**任務**：實作 convert 命令在未使用 `--keep-original` 時，自動移除原始檔案的功能。

## 一、核心變更

1. **新增檔案管理器**  
   - 建立 `src/core/file_manager.rs`，定義 `FileManager` 與 `FileOperation`，提供檔案建立與移除的安全操作及回滾機制。  
   - 在 `src/core/mod.rs` 加入 `pub mod file_manager`。

2. **擴充錯誤類型**  
   - 在 `src/error.rs` 新增 `FileAlreadyExists`、`FileNotFound`、`InvalidFileName`、`FileOperationFailed`，以便明確描述檔案操作錯誤。

3. **整合至 Convert 命令流程**  
   - 在 `src/commands/convert_command.rs` 中引用 `FileManager`，於單檔案轉換成功後（`result.success == true`）記錄建立的輸出檔案，並在 `--keep-original` 未指定時移除原始檔案；若移除或回滾失敗，則僅輸出警告並不影響轉換結果。

## 二、測試驗證

1. **單元測試**  
   - `src/core/file_manager.rs` 新增檔案移除與回滾測試，驗證 `remove_file`、`record_creation` 及 `rollback` 行為。

2. **整合測試 (手動檢驗)**  
   - 可透過 CLI 端到端測試確認預設與 `--keep-original` 行為：  
     ```bash
     subx-cli convert input.srt --format vtt
     subx-cli convert input.srt --format vtt --keep-original
     ```

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 三、結論

本次修正依據需求完整實作了 Convert 命令的檔案移除機制，並補強相關錯誤處理與測試，確保在不同情境下皆符合預期行為，所有測試通過且無新增警告。
