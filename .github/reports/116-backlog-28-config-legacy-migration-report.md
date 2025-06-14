---
title: "Job Report: Backlog #28 - Config 系統 Legacy 完全移除"
date: "2025-06-12T18:30:41Z"
---

# Backlog #28 - Config 系統 Legacy 完全移除 工作報告

**日期**：2025-06-12T18:30:41Z  
**任務**：將舊版 `config_legacy.rs` 完全移除，並遷移配置結構與工具函式至新的依賴注入式 `ConfigService` 系統  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述
本次任務旨在淘汰 legacy 配置檔 `config_legacy.rs`，統一使用依賴注入式的 `ConfigService` 系統，增強可測試性與模組化。

## 二、實作內容

### 2.1 里程碑 1：遷移配置結構並移除 `config_legacy.rs`
- 移除舊版 legacy 檔案 `src/config/config_legacy.rs` 【F:src/config/config_legacy.rs†D】
- 更新 `src/config/mod.rs`，刪除 `mod config_legacy` 並內嵌所有配置類型定義【F:src/config/mod.rs†L1-L15】【F:src/config/mod.rs†L25-L33】

### 2.2 里程碑 2：擴充 `ConfigService` Trait 及實作
- 在 `src/config/service.rs` 中擴增 `ConfigService` trait 方法簽章（save_config, save_config_to_file, get_config_file_path, get_config_value, reset_to_defaults）【F:src/config/service.rs†L27-L71】
- 完成 `ProductionConfigService` 的 I/O 實作 stub【F:src/config/service.rs†L187-L272】
- 在 `src/config/test_service.rs` 實作對應方法並維持測試隔離【F:src/config/test_service.rs†L39-L80】

## 三、技術細節

### 3.1 架構變更
- 移除 `config_legacy.rs`，所有配置資料結構統一於 `src/config/mod.rs` 定義
- `ConfigService` 作為唯一對外 API 進行配置存取與操作

### 3.2 API 變更
- 新增 `save_config`, `save_config_to_file`, `get_config_file_path`, `get_config_value`, `reset_to_defaults` 方法於 `ConfigService` trait

### 3.3 測試與驗證更新
- 調整 `validator.rs` 中 Option unwrap 為 `as_deref` 以消除類型推斷問題【F:src/config/validator.rs†L36-L43】

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo check
```

## 五、後續事項

### 5.1 待完成項目
- 里程碑 3~5: 更新 CLI 命令、撰寫整合測試、最終文件檢查與報告

### 5.2 相關任務
- Backlog #28: Config 系統 Legacy 完全移除計劃

### 5.3 建議後續
- 立即實作里程碑 3：更新 `config_command.rs`，全面切換至 `ConfigService` API

## 六、檔案異動清單
| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| `.github/plans/backlogs/28-config-legacy-migration-completion.md` | 修改 | 新增里程碑規劃【F:.github/plans/backlogs/28-config-legacy-migration-completion.md†L3-L10】 |
| `src/config/config_legacy.rs` | 刪除 | 移除 legacy 檔案 |
| `src/config/mod.rs` | 修改 | 刪除 legacy 模組並遷移類型定義【F:src/config/mod.rs†L1-L15】【F:src/config/mod.rs†L25-L33】 |
| `src/config/service.rs` | 修改 | 擴充 `ConfigService` trait 及實作 stub【F:src/config/service.rs†L27-L71】【F:src/config/service.rs†L187-L272】 |
| `src/config/test_service.rs` | 修改 | 實作新的 `ConfigService` 方法於 TestConfigService【F:src/config/test_service.rs†L39-L80】 |
| `src/config/validator.rs` | 修改 | Option unwrap 改 `as_deref`【F:src/config/validator.rs†L36-L43】 |
