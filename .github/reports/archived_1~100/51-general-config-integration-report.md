---
title: "工作報告: Bug #11.3 - 一般配置整合優化"
date: "2025-06-09T09:41:04Z"
---

# Bug #11.3 - 一般配置整合優化 工作報告

**日期**: 2025-06-09T09:41:04Z  
**任務目標**: 整合 `enable_progress_bar`、`task_timeout_seconds` 與 `worker_idle_timeout_seconds` 配置，優化進度條顯示與任務調度器逾時機制。

> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、實作內容

### 1.1 根據 `enable_progress_bar` 控制進度條顯示
- 在 CLI UI 中 `create_progress_bar` 函式新增配置檢查，若 `enable_progress_bar = false` 則隱藏進度條。  
- 更新 `match` 命令呼叫 `create_progress_bar` 後，依配置決定是否顯示。  
- 【F:src/cli/ui.rs†L25-L30】【F:src/commands/match_command.rs†L81-L89】

### 1.2 在任務調度器中新增逾時與閒置機制
- 擴充 `TaskScheduler` 結構，新增 `task_timeout` 與 `worker_idle_timeout` 欄位，用於控制單一任務逾時與調度器閒置退出。  
- 在 `new` 與 `new_with_defaults` 初始化時，從 `general` 配置讀取逾時秒數。  
- 【F:src/core/parallel/scheduler.rs†L63-L66】【F:src/core/parallel/scheduler.rs†L83-L100】【F:src/core/parallel/scheduler.rs†L117-L133】

### 1.3 實作逾時執行與閒置退出邏輯
- 在背景調度迴圈中追蹤最後活躍時間，若超過 `worker_idle_timeout` 則結束調度器迴圈。  
- 將執行任務包裹在 `tokio::time::timeout(task_timeout, ...)` 中，逾時返回 `TaskResult::Failed("任務執行逾時")`。  
- 【F:src/core/parallel/scheduler.rs†L146-L165】【F:src/core/parallel/scheduler.rs†L198-L204】

### 1.4 更新 `Clone` 實作保持新欄位
- 在 `TaskScheduler::clone` 中加入新的逾時欄位，確保複製後行為一致。  
- 【F:src/core/parallel/scheduler.rs†L441-L449】

## 二、驗證
```bash
cargo fmt -- --check && \
cargo clippy -- -D warnings && \
cargo test
```

結果：所有檢查與測試皆通過。

## 三、後續事項
- 可考慮將進度條整合至其他長時間命令（如 `sync`, `detect-encoding`）。
- 規劃更多逾時策略，例如自動重試或錯誤回饋。

---
**檔案異動**:
- src/cli/ui.rs
- src/commands/match_command.rs
- src/core/parallel/scheduler.rs
