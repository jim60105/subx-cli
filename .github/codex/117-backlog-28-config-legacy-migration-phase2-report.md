---
title: "Job Report: Backlog #28 - Config 系統 Legacy 完全移除 Phase 3 CLI 命令更新"
date: "2025-06-12T18:49:08Z"
---

# Backlog #28 - Config 系統 Legacy 完全移除 Phase 3 CLI 命令更新 工作報告

**日期**：2025-06-12T18:49:08Z  
**任務**：更新 `config_command.rs`，移除舊版執行邏輯並改用 `ConfigService` 介面  
**類型**：Backlog  
**狀態**：已完成

## 一、任務概述

針對 Backlog #28 的里程碑 3，需調整 CLI 的 `config` 子命令，移除原先直寫邏輯，改為依賴注入 `ConfigService`，統一使用服務介面。

## 二、實作內容

### 2.1 更新 `config_command.rs` 命令實作
- 修改匯入模組，使用 `crate::cli::{ConfigAction, ConfigArgs}`、`crate::config::ConfigService` 及 `crate::error::{SubXError, SubXResult}`【F:src/commands/config_command.rs†L109-L111】
- 重構 `execute` 函式內部實作，統一透過 `ConfigService` 操作：
  - `Set` 統一回傳不支援錯誤
  - `Get` 使用 `get_config_value` 取得值並輸出
  - `List` 使用 `get_config` 及 `get_config_file_path` 輸出完整設定
  - `Reset` 調用 `reset_to_defaults` 並提示使用者
  【F:src/commands/config_command.rs†L240-L278】
- 同步重構 `execute_with_config` 以相同邏輯取代舊版行為【F:src/commands/config_command.rs†L293-L327】

## 三、技術細節

### 3.1 CLI 執行流程變更
- 原先 `config` 子命令內部自行讀寫設定，現改為呼叫注入的 `ConfigService` 介面，提升測試性與一致性。

## 四、程式碼品質檢查

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
```

## 五、後續事項

### 5.1 待完成項目
- 實作里程碑 4：新增整合測試驗證 Detalis
- 實作里程碑 5：文件更新與最終驗證

## 六、檔案異動清單

| 檔案路徑                        | 異動類型 | 描述                                                      |
|-------------------------------|--------|---------------------------------------------------------|
| `src/commands/config_command.rs` | 修改     | 更新匯入及重構 `execute`、`execute_with_config` 內部實作  |
