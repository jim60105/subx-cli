---
title: "Job Report: Bug Fix #06 - Sync 命令參數簡化"
date: "2025-06-08T08:31:33Z"
---

## Bug Fix #06: Sync 命令參數簡化

**日期**：2025-06-08T08:31:33Z  
**任務**：移除冗餘的 `--method` 參數，並根據 `--offset` 的存在自動判斷同步方法，同步更新程式碼、測試與文件。

## 一、核心變更

1. **移除 `--method` 參數與相關定義**  
   【F:src/cli/sync_args.rs†L2-L4】【F:src/cli/sync_args.rs†L24-L28】

2. **新增 `SyncMethod` 型別與 `sync_method()` 方法**  
   【F:src/cli/sync_args.rs†L27-L45】

3. **新增單元測試以驗證同步方法選擇邏輯**  
   【F:src/cli/sync_args.rs†L47-L74】

4. **更新整合測試，移除對 `--method manual` 的呼叫**  
   【F:tests/integration_tests.rs†L57-L63】

5. **更新文件，移除 README 中的 `--method` 選項並新增 `--range` 說明**  
   【F:README.md†L201-L204】

## 二、驗證

- 執行 `cargo fmt`、`cargo clippy -- -D warnings`、`cargo test`，全部通過

