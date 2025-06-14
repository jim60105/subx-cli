---
title: "Job Report: Backlog #25 - Match Command Copy/Move Logic and Tests"
date: "2025-06-11T17:14:30Z"
---

> ! This work has been reverted.

# Backlog #25 - Match Command Copy/Move Logic and Tests 工作報告

**日期**：2025-06-11T17:14:30Z  
**任務**：實作字幕檔案實際複製/移動邏輯與衝突解決策略，並撰寫 `--copy` / `--move` 參數相關單元與整合測試  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述

根據先前報告中「後續事項」，補齊以下項目：
- 實作檔案複製/移動的核心邏輯及自動衝突命名策略
- 撰寫並驗證 `--copy` / `--move` 參數相關的單元測試與整合測試

## 二、實作內容

### 2.1 實作檔案衝突自動命名邏輯
- 新增 `resolve_filename_conflict` 函式，於目標已存在同名檔案時，於檔名後加數字後綴至無衝突為止
- 位置：【F:src/core/matcher/engine.rs†L53-L79】

### 2.2 實作並行檔案重新定位任務
- 新增 `ProcessingOperation::CopyToVideoFolder` 與 `MoveToVideoFolder`，並實作 `FileRelocationTask` 的 `execute_copy_operation` / `execute_move_operation`
- 位置：【F:src/core/parallel/mod.rs†L355-L434】

```rust
// Copy 操作示例
match std::fs::copy(source, target) {
    Ok(_) => TaskResult::Success(format!("Copied: {} -> {}", source.display(), target.display())),
    Err(e) => TaskResult::Failure(format!("Copy failed: {}", e)),
}
```

### 2.3 撰寫單元測試：`resolve_filename_conflict` 與 `FileRelocationTask`
- 新增 `tests/match_copy_operation_tests.rs`，包含：
  - `test_resolve_filename_conflict`
  - `test_copy_to_video_folder_basic`
  - `test_move_to_video_folder_basic`
  - `test_backup_with_copy_operation`
  - `test_backup_with_move_operation`
  - `test_permission_denied_handling`
- 位置：【F:tests/match_copy_operation_tests.rs†L1-L100】

## 三、技術細節

### 3.1 架構變更
- 檔案複製/移動邏輯由核心引擎內部移至 `core/parallel` 任務機制，以利並行化與重用

### 3.2 API 變更
- 新增 `ProcessingOperation::CopyToVideoFolder`、`ProcessingOperation::MoveToVideoFolder` 與 `FileRelocationTask`

### 3.3 配置變更
- 無

## 四、測試與驗證

```bash
# 檔案格式與靜態檢查
cargo fmt -- --check
cargo clippy -- -D warnings

# 文件品質檢查
timeout 20 scripts/check_docs.sh

# 單元測試：檔案重新定位邏輯
cargo test --test match_copy_operation_tests

# 整合測試：match --copy/--move 端到端
cargo test --test match_integration_copy_move_tests

# 全量測試
cargo test
```

所有新增測試皆已通過。

## 五、影響評估

### 5.1 向後相容性
- Copy/Move 功能預設關閉，不影響既有使用流程

### 5.2 使用者體驗
- 於使用 `--copy` 或 `--move` 時，自動整理字幕至影片資料夾並解決檔名衝突，減少手動操作

## 六、問題與解決方案

### 6.1 遇到的問題
- 權限不足時檔案操作會失敗，已於測試中模擬並驗證錯誤處理
- 臨時目錄操作需慎用 `tempdir` 確保檔案存在與清理

## 七、後續事項

### 7.1 待完成項目
- [ ] 強化大量檔案處理進度回饋與性能優化
- [ ] 補充 dry-run 模式下的覆蓋測試與文件範例

### 7.2 相關任務
- Backlog #25

### 7.3 建議的下一步
- 實作 I/O 重試機制與用戶提示
- CLI UI 增加 relocate 進度顯示

## 八、檔案異動清單

| 檔案路徑                             | 異動類型 | 描述                                         |
|-------------------------------------|----------|----------------------------------------------|
| `src/core/matcher/engine.rs`        | 修改     | 新增 `resolve_filename_conflict`              |
| `src/core/parallel/mod.rs`          | 修改     | 新增 Copy/Move 任務與執行邏輯                |
| `tests/match_copy_operation_tests.rs` | 新增   | Copy/Move 及衝突命名邏輯單元測試             |
| `tests/match_integration_copy_move_tests.rs` | 新增 | `match --copy/--move` 端到端整合測試         |
