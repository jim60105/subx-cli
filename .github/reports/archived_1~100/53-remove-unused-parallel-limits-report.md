---
title: "Job Report: Bug Fix #12 - 移除未使用的並行限制配置項目"
date: "2025-06-09T11:08:59Z"
---

# Bug Fix #12 - 移除未使用的並行限制配置項目 工作報告

**日期**：2025-06-09T11:08:59Z  
**任務**：根據 Bug #12 建議，完全移除 `cpu_intensive_limit` 和 `io_intensive_limit` 兩個死代碼配置項目，以簡化配置並避免混淆。

> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、實作內容

### 1.1 移除應用配置中 ParallelConfig 及預設值
- 刪除 `src/config.rs` 中 `ParallelConfig` 結構 `cpu_intensive_limit` 與 `io_intensive_limit` 欄位及對應 `Default` 實作  
- 檔案變更：【F:src/config.rs†L305-306】【F:src/config.rs†L317-318】

### 1.2 更新部分配置結構和合併邏輯
- 從 `PartialParallelConfig` 中移除對應欄位及 `merge` 與 `to_complete_config` 的處理  
- 檔案變更：【F:src/config/partial.rs†L96-97】【F:src/config/partial.rs†L183-187】【F:src/config/partial.rs†L318-331】

### 1.3 清理 core/parallel/config.rs
- 刪除 `cpu_intensive_limit` 與 `io_intensive_limit` 相關定義、驗證與初始化，並更新單元測試  
- 檔案變更：【F:src/core/parallel/config.rs†L7-13】【F:src/core/parallel/config.rs†L28-54】【F:src/core/parallel/config.rs†L56-60】【F:src/core/parallel/config.rs†L80-102】

### 1.4 移除文件與文檔
- 更新 README 示例，刪除 `cpu_intensive_limit` 和 `io_intensive_limit` 行  
- 檔案變更：【F:README.md†L184-185】
- 移除 `command.instructions.md` 中對應配置條目及說明  
- 檔案變更：【F:.github/instructions/command.instructions.md†L56-59】【F:.github/instructions/command.instructions.md†L84】

## 二、驗證

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```
結果：通過

## 三、後續事項

- 確認是否需對 `enable_smart_resampling` 功能進行後續實作（在並行及同步配置整合後）。

---
**檔案異動**：
- src/config.rs
- src/config/partial.rs
- src/core/parallel/config.rs
- README.md
- .github/instructions/command.instructions.md
