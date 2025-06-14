---
title: "Job Report: Bug Fix #17 - Sync 指令 --offset 參數使用時完全忽略視頻檔案需求"
date: "2025-06-14T14:38:52Z"
---

# Bug Fix #17 - Sync 指令 --offset 參數使用時完全忽略視頻檔案需求 工作報告

**日期**: 2025-06-14T14:38:52Z  
**任務**: 修正當使用 `--offset` 時應完全跳過視頻檔案需求  
**類型**: Bug Fix  
**狀態**: 已完成

## 一、任務概述

使用者在執行手動偏移模式時應僅針對字幕檔案應用時間偏移，不應再要求提供視頻檔案；目前行為仍然強制要求視頻檔案，與預期不符。

## 二、實作內容

### 2.1 CLI 參數結構調整
- 將 `video` 欄位從 `PathBuf` 調整為 `Option<PathBuf>`，並新增 `required_unless_present = "offset"` 屬性，使得在手動偏移模式下可選。  
【F:src/cli/sync_args.rs†L111-L118】
- 新增 `validate` 方法以檢查參數組合的合理性，並在自動模式缺少視頻時回報友好錯誤訊息。  
【F:src/cli/sync_args.rs†L374-L391】
- 新增 `requires_video` 方法（供未來擴充使用）。  
【F:src/cli/sync_args.rs†L393-L397】

```rust
/// 自動或手動模式參數驗證
pub fn validate(&self) -> SubXResult<()> {
    match (self.offset.is_some(), self.video.is_some()) {
        (true, _) => Ok(()),
        (false, true) => Ok(()),
        (false, false) => Err(SubXError::CommandExecution(
            "視頻檔案在自動同步模式下是必填的...".to_string()
        )),
    }
}
```

### 2.2 同步邏輯重構
- 重構 `execute_sync_logic` 方法，拆分手動與自動同步流程邏輯。  
【F:src/commands/sync_command.rs†L330-L342】
- 實作 `execute_manual_sync`、`execute_automatic_sync`、`execute_batch_sync`、`execute_single_sync` 區分不同同步場景處理。  
【F:src/commands/sync_command.rs†L344-L430】

## 三、文檔與範例更新

- 更新 README.md 中 `sync` 指令使用範例，展示手動同步可跳過視頻檔案且保留舊格式向後相容。  
【F:README.md†L108-L119】
- 同步更新 README.zh-TW.md 中文範例與說明。  
【F:README.zh-TW.md†L108-L119】【F:README.zh-TW.md†L342-L345】

## 四、測試與驗證

### 4.1 單元測試
- 新增 `tests/commands/sync_command_manual_offset_tests.rs` 測試手動偏移跳過視頻檔案行為。  
【F:tests/commands/sync_command_manual_offset_tests.rs†L1-L38】

### 4.2 CLI 整合測試
- 新增 `tests/cli/sync_manual_offset_integration_tests.rs` 驗證 CLI 介面輸出錯誤與成功訊息。  
【F:tests/cli/sync_manual_offset_integration_tests.rs†L1-L25】

## 五、程式碼品質檢查

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
timeout 30 scripts/quality_check.sh
scripts/check_coverage.sh -T
```

## 六、影響評估

### 6.1 向後相容性
- 保持舊格式 `sync video.mp4 subtitle.srt --offset` 支援，不影響現有腳本。  

### 6.2 使用者體驗
- 簡化手動同步流程，無需冗餘視頻檔案參數，提升操作便利性。

## 七、後續事項

### 7.1 建議項目
- 更新 CLI 幫助文件與 man page 範例。  
- 規劃自動同步效能優化（Backlog #19）。

## 八、檔案異動清單

| 檔案路徑 | 異動類型 | 描述 |
| -------- | -------- | ---- |
| `src/cli/sync_args.rs` | 修改 | 動態必填參數與驗證 |
| `src/commands/sync_command.rs` | 修改 | 重構同步流程邏輯 |
| `README.md` | 修改 | 更新 `sync` 範例 |
| `README.zh-TW.md` | 修改 | 更新中文範例 |
| `tests/commands/sync_command_manual_offset_tests.rs` | 新增 | 手動偏移單元測試 |
| `tests/cli/sync_manual_offset_integration_tests.rs` | 新增 | CLI 整合測試 |
