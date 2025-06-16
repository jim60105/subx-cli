---
title: "Job Report: Bug Fix #20 - 配置 CLI 不一致性 - VAD 配置支援缺失"
date: "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
---

# Bug Fix #20 - 配置 CLI 不一致性 - VAD 配置支援缺失 工作報告

**日期**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")  
**任務**: 修復配置服務 `get_config_value` 與 `set_config_value` 支援項目不一致，新增 VAD 相關 CLI 操作  
**類型**: Bug Fix  
**狀態**: 已完成

## 一、任務概述
在 CLI 層面發現 `ProductionConfigService` 的 `get_config_value` 僅支援 15 項配置，而 `set_config_value` 支援 31 項，且完全缺少 VAD 相關的 7 項配置，使用者無法透過 CLI 操作 VAD 參數，造成核心功能使用障礙與一致性問題。

## 二、實作內容

### 2.1 擴展 `get_config_value` 支援所有缺失項目
- 新增 AI、Formats、Sync（含 VAD、default_method）、General、Parallel 的 get match 分支  
- 移除對未知項目的誤判  
- 相關程式碼位置：【F:src/config/service.rs†L545-L578】

```rust
// 新增 VAD 配置讀取
["sync", "vad", "enabled"] => Ok(config.sync.vad.enabled.to_string()),
["sync", "vad", "sensitivity"] => Ok(config.sync.vad.sensitivity.to_string()),
```

### 2.2 擴展 `validate_and_set_value` 新增 VAD 設定支援
- 新增 `sync.default_method` 及 7 項 VAD 配置的驗證與設置  
- 移除對已棄用 `correlation_threshold`、`dialogue_detection_threshold` 的支援  
- 相關程式碼位置：【F:src/config/service.rs†L339-L380】【F:src/config/service.rs†L381-L430】

```rust
// VAD 設定
["sync", "vad", "enabled"] => { config.sync.vad.enabled = parse_bool(value)?; },
["sync", "vad", "sensitivity"] => { config.sync.vad.sensitivity = validate_float_range(value, 0.0, 1.0)?; },
```

### 2.3 新增單元與集成測試
- 測試 get/set 方法支援項目一致性  
- 測試 VAD 配置的完整 get/set 循環與驗證邏輯  
- 測試程式碼位置：【F:tests/config_cli_vad_support_tests.rs†L1-L50】

### 2.4 更新文件與範例
- 文件 `docs/configuration-guide.md`、CLI help (`src/cli/config_args.rs`)、使用分析 `docs/config-usage-analysis.md` 同步新增 VAD CLI 操作範例與說明  
- 相關程式碼位置：【F:docs/configuration-guide.md†L28-L38】【F:src/cli/config_args.rs†L45-L51】【F:docs/config-usage-analysis.md†L136-L143】

## 三、測試與驗證

### 3.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

### 3.2 測試結果
- 所有新增及現有測試通過，VAD CLI get/set 環節驗證成功。

## 四、影響評估

### 4.1 向後相容性
- 向後相容：CLI 新增支援但不移除舊版配置欄位  

### 4.2 使用者體驗
- CLI 操作一致性改善，使用者可透過 `subx config get/set sync.vad.*` 直接管理 VAD 參數。

## 五、後續事項

### 5.1 建議的下一步
- 可考慮移除更多已棄用配置並清理相關程式碼與文檔。

### 5.2 相關任務
- Bug #20 配置 CLI 不一致性 - VAD 配置支援缺失

## 六、檔案異動清單
| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| `src/config/service.rs` | 修改 | 擴展 get/set 支援 VAD 與其他缺失項目，移除已棄用項目 |
| `tests/config_cli_vad_support_tests.rs` | 新增 | VAD CLI 支援測試與一致性測試 |
| `docs/configuration-guide.md` | 修改 | 新增 VAD CLI 操作範例 |
| `src/cli/config_args.rs` | 修改 | 更新 CLI 幫助範例支持 VAD 配置 |
| `docs/config-usage-analysis.md` | 修改 | 更新同步設定分析與註解 |
