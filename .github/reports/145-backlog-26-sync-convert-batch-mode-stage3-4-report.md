---
title: "Job Report: Backlog #26 - Sync 與 Convert 批次模式命令實作及測試"
date: "2025-06-15T22:44:08Z"
---

# Backlog #26 - Sync 與 Convert 批次模式命令實作及測試 工作報告

**日期**: 2025-06-15T22:44:08Z  
**任務**: 完成階段3 (Sync 與 Convert 命令批次模式實作) 及階段4 (對應測試)  
**類型**: Backlog  
**狀態**: 已完成

## 一、任務概述

本次工作針對 backlog #26，專注於 Sync 與 Convert 命令的批次模式 (Batch Mode) 實作與測試：
- **階段3**：在 `sync_command.rs` 與 `convert_command.rs` 中完成批次處理流程邏輯
- **階段4**：新增批次模式相關的單元與整合測試，驗證行為正確性

## 二、實作內容

### 2.1 Sync 命令批次模式實作
- 引入 `run_single` 私有非公開函式，集中處理單檔同步邏輯，並在 `execute` 中展開批次迴圈
- 修改檔案路徑：
  - `src/commands/sync_command.rs`【F:src/commands/sync_command.rs†L16-L87】【F:src/commands/sync_command.rs†L115-L173】

```rust
async fn run_single(
    args: &SyncArgs,
    config: &Config,
    sync_engine: &SyncEngine,
    format_manager: &FormatManager,
) -> Result<()> { /* 單檔同步邏輯 */ }

pub async fn execute(args: SyncArgs, config_service: &dyn ConfigService) -> Result<()> {
    // 批次模式偵測與展開
    if let Ok(SyncMode::Batch(handler)) = args.get_sync_mode() {
        for video in handler.collect_files()? { /* 自動配對並呼叫 run_single */ }
        return Ok(());
    }
    // 單檔模式
    run_single(&args, &config, &sync_engine, &format_manager).await?;
    Ok(())
}
```

### 2.2 Convert 命令批次模式實作
- 依據 `ConvertArgs::get_input_handler()` 與 `handler.collect_files()`，採批次 `for input_path in files` 處理多檔轉檔
- 修改檔案路徑：
  - `src/commands/convert_command.rs`【F:src/commands/convert_command.rs†L242-L261】

```rust
for input_path in files {
    let output_path = if let Some(o) = &args.output { /* 目錄模式命名 */ } else { /* 單檔副檔名 */ };
    converter.convert_file(&input_path, &output_path, &fmt).await?;
}
```

### 2.3 Sync 批次模式測試
- 新增 `test_sync_batch_processing`，在臨時目錄建立多對影片與字幕檔後呼叫 `execute`，驗證同步檔案輸出
- 測試檔案：
  - `src/commands/sync_command.rs`【F:src/commands/sync_command.rs†L183-L203】

### 2.4 Convert 批次模式測試
- 使用 `test_convert_batch_processing` 驗證轉檔命令可成功執行非空結果
- 測試檔案：
  - `src/commands/convert_command.rs`【F:src/commands/convert_command.rs†L348-L368】

## 三、技術細節

### 3.1 架構變更
- Sync 命令將單檔邏輯抽離至 `run_single`，並以 `SyncMode` 判別批次/單模式，避免重複程式碼

### 3.2 API 變更
- `execute` 函式簽章維持相容，新增非公開 `run_single` 作為內部呼叫

## 四、測試與驗證

```bash
# 單元與整合測試
cargo test test_sync_batch_processing test_convert_batch_processing

# 指令解析測試
cargo test test_sync_args_batch_input test_convert_args_multiple_input_recursive_and_keep_original
```

## 五、後續事項

- 建議後續可增強 Sync 批次模式的自動影片與字幕最佳配對策略
- 平行化批次處理以提升大量檔案效能

