---
title: "Job Report: Bug Fix #22 - CIFS 檔案系統 std::fs::copy 權限錯誤修復"
date: "2025-06-17T10:53:10Z"
---

# Bug Fix #22 - CIFS 檔案系統 std::fs::copy 權限錯誤修復 工作報告

**日期**：2025-06-17T10:53:10Z  
**任務**：修復在 CIFS (SMB) 檔案系統上執行 `std::fs::copy` 時因複製 POSIX 權限導致的「Operation not permitted」錯誤  
**類型**：Bug Fix  
**狀態**：已完成

## 一、任務概述

在 CIFS 掛載點上呼叫 `std::fs::copy` 會因底層不支援 POSIX 權限操作而失敗，導致 SubX 在複製檔案 (如 match 指令的 copy 模式或備份流程) 時拋出「Operation not permitted (os error 1)」錯誤。此問題影響所有執行跨目錄複製的功能。

## 二、實作內容

### 2.1 增加 CIFS 相容的檔案複製工具函式
- 實作 `copy_file_cifs_safe`，只複製檔案內容而不嘗試複製 metadata，避免權限設定失敗  
- 定義於 `src/core/fs_util.rs`【F:src/core/fs_util.rs†L19-L23】

### 2.2 註冊 fs_util 模組
- 在核心模組中註冊新的檔案運算工具模組  
- 修改 `src/core/mod.rs`【F:src/core/mod.rs†L22】

### 2.3 取代所有直接呼叫 `std::fs::copy` 的程式碼
- 在 `engine.rs` 中，以 `copy_file_cifs_safe` 取代檔案備份與複製流程  
- 相關行數【F:src/core/matcher/engine.rs†L872-L877】【F:src/core/matcher/engine.rs†L915-L927】【F:src/core/matcher/engine.rs†L971-L991】【F:src/core/matcher/engine.rs†L1006-L1026】【F:src/core/matcher/engine.rs†L1091-L1100】
- 在 `task.rs` 中，以 `copy_file_cifs_safe` 取代平行處理的檔案複製操作  
- 相關行數【F:src/core/parallel/task.rs†L306-L308】

## 三、技術細節

### 3.1 架構變更
- 新增 `fs_util` 模組集中管理檔案複製邏輯，以避免重複實作

### 3.2 API 變更
- 無對外公開 API 變更，僅內部實現優化

### 3.3 配置變更
- 無新增或修改設定檔

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
cargo test -- --ignored --nocapture
```

### 4.2 單元測試
- 新增對 `copy_file_cifs_safe` 的單元測試，驗證檔案內容正確複製  
- 測試定義於 `src/core/fs_util.rs`【F:src/core/fs_util.rs†L25-L43】

### 4.3 功能測試
- 手動於 CIFS 掛載點測試 copy 模式與備份流程均不再因權限失敗

## 五、影響評估

### 5.1 向後相容性
- 原有檔案內容複製行為不變，僅不再複製 POSIX metadata

### 5.2 使用者體驗
- 在網路檔案系統上消除權限錯誤，提升穩定性

## 六、問題與解決方案

### 6.1 遇到的問題
- `scripts/quality_check.sh` 執行時間超出 CI 環境限制，改以手動 `cargo test` 驗證

### 6.2 技術債務
- 尚未同步檔案時間戳與其他 metadata，如有需求可後續再補

## 七、後續事項

### 7.1 待完成項目
- [ ] 額外測試其他 network FS 兼容性

### 7.2 相關任務
- [.github/plans/bugs/22-cifs-filesystem-compatibility-std-fs-copy-permission-error.md](../../plans/bugs/22-cifs-filesystem-compatibility-std-fs-copy-permission-error.md)

### 7.3 建議的下一步
- 進行更多不同檔案系統的回歸測試

## 八、檔案異動清單

| 檔案路徑                         | 異動類型 | 描述                               |
|----------------------------------|----------|------------------------------------|
| `src/core/fs_util.rs`            | 新增     | 增加 CIFS 相容的檔案複製工具函式    |
| `src/core/mod.rs`                | 修改     | 註冊 fs_util 模組                  |
| `src/core/matcher/engine.rs`    | 修改     | 以 copy_file_cifs_safe 取代 std::fs::copy |
| `src/core/parallel/task.rs`      | 修改     | 以 copy_file_cifs_safe 取代 std::fs::copy |
