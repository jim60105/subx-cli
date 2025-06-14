---
title: "Bug Fix Report: #16 Match Command Cache 重用與 Copy 模式錯誤修復"
date: "2025-06-11T23:13:13Z"
---

# Bug Fix Report: #16 Match Command Cache 重用與 Copy 模式錯誤修復

**修復日期**：2025-06-11T23:13:13Z  
**問題編號**：Bug #16  
**問題描述**：在 `match` 指令的 cache 重用機制和 copy 模式中，cache 重用時忽略 copy/move 參數，並且 copy 模式下原始檔案被重新命名【F:.github/plans/bugs/16-match-command-cache-copy-mode-bugs.md†L1-L21】

**修復狀態**：✅ 已完成並驗證

## 一、問題根因分析

### 1.1 Cache 重用忽略重定位參數
原始實作在 `check_cache` 中將 `relocation_target_path` 設為 `None`、`requires_relocation` 設為 `false`，導致忽略使用者參數【F:src/core/matcher/engine.rs†L1072-L1094】

### 1.2 Copy 模式下重新命名邏輯錯誤
`execute_relocation_operation` 判斷 `source_path` 使用已重新命名路徑，導致 copy 實際上移動檔案【F:src/core/matcher/engine.rs†L786-L814】

## 二、實作內容

### 2.1 增強 CacheData 結構
- 新增 `original_relocation_mode`、`original_backup_enabled` 欄位，以便儲存當前配置參數  
- 檔案變更：【F:src/core/matcher/cache.rs†L69-L75】

### 2.2 修正 check_cache 重新計算 relocation
- 根據當前 `config.relocation_mode` 決定 `requires_relocation` 與 `relocation_target_path`  
- 檔案變更：【F:src/core/matcher/engine.rs†L1072-L1094】

### 2.3 修正 save_cache
- 儲存 `original_relocation_mode`、`original_backup_enabled` 欄位  
- 檔案變更：【F:src/core/matcher/engine.rs†L1188-L1196】

### 2.4 分離 Copy 與 Move 執行流程
- 在 `execute_operations` 中依 `relocation_mode` 分支執行 `execute_copy_then_rename` 或 `execute_relocation_operation`  
- 新增 `execute_copy_then_rename` 方法  
- 檔案變更：【F:src/core/matcher/engine.rs†L784-L814】【F:src/core/matcher/engine.rs†L852-L894】

### 2.5 新增整合測試
- 測試 cache 重用時保留 copy/move 行為  
- 測試 copy 模式下原始檔案保留與 rename 行為  
- 檔案變更：【F:tests/match_cache_reuse_tests.rs†L1-L41】【F:tests/match_copy_behavior_tests.rs†L1-L33】

## 三、測試與驗證

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```
**結果**：所有測試通過 & Clippy 無警告

## 四、後續事項

- 無

## 五、檔案異動清單

| 檔案路徑                              | 異動類型 | 描述                         |
|--------------------------------------|----------|------------------------------|
| `src/core/matcher/cache.rs`          | 修改     | 新增 CacheData 欄位          |
| `src/core/matcher/engine.rs`         | 修改     | 修正 cache 重用與 copy 邏輯  |
| `tests/match_cache_reuse_tests.rs`   | 新增     | 增加 cache 重用測試          |
| `tests/match_copy_behavior_tests.rs` | 新增     | 增加 copy 模式行為測試       |
