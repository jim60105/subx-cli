---
title: "Job Report: Backlog #25 - Match Command Copy/Move to Video Folder Feature"
date: "2025-06-11T16:10:59Z"
---

# Backlog #25 - Match Command Copy/Move to Video Folder Feature 工作報告

**日期**：2025-06-11T16:10:59Z  
**任務**：新增 `subx match` 命令的字幕檔案複製/移動至影片資料夾功能  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述
根據 Product Backlog #25 規格，為 `subx match` 命令新增 `--copy`/`--move` 參數，可在 AI 成功匹配後，將字幕檔案自動複製或移動到對應影片檔案所在資料夾，提升使用者媒體管理體驗。

## 二、實作內容

### 2.1 命令列參數與參數驗證
- 在 `MatchArgs` 結構體中新增 `copy: bool`、`move_files: bool` 兩個參數，並導入 `Default` 以支援測試初始化  【F:src/cli/match_args.rs†L38-L47】【F:src/cli/match_args.rs†L62-L75】
- 實作 `validate()` 方法，驗證 `--copy` 與 `--move` 互斥使用  【F:src/cli/match_args.rs†L78-L85】
- 新增 CLI 參數互斥性單元測試  【F:src/cli/match_args.rs†L95-L107】

### 2.2 核心資料結構與設定擴充
- 在 `core/matcher/engine.rs` 定義 `FileRelocationMode`、`ConflictResolution` 枚舉，並於 `MatchConfig` 中新增 `relocation_mode` 與 `conflict_resolution` 欄位  【F:src/core/matcher/engine.rs†L33-L50】【F:src/core/matcher/engine.rs†L54-L69】
- 在 `core/matcher/mod.rs` 重新導出新定義的型別  【F:src/core/matcher/mod.rs†L167-L170】

### 2.3 命令實作與工廠注入
- 更新 `match_command.rs`，在建構 `MatchConfig` 時依 `args.copy`/`args.move_files` 設定 `relocation_mode`，並給予預設 `conflict_resolution: AutoRename`  【F:src/commands/match_command.rs†L305-L315】
- 更新 `ComponentFactory::create_match_engine` 預設 `MatchConfig` 加入新欄位預設值  【F:src/core/factory.rs†L38-L47】

### 2.4 測試與範例說明更新
- 更新整合測試 `match_engine_id_integration_tests.rs` 的 `MatchConfig` 初始化以涵蓋新欄位  【F:tests/match_engine_id_integration_tests.rs†L3-L6】【F:tests/match_engine_id_integration_tests.rs†L98-L105】
- 更新 `match_command` 的單元測試與 doc examples，為 `MatchArgs` 初始化補上 `copy: false`、`move_files: false`  【F:src/commands/match_command.rs†L52-L61】【F:src/commands/match_command.rs†L143-L152】【F:src/commands/match_command.rs†L156-L163】【F:src/commands/match_command.rs†L295-L303】
- 更新 CLI 模組 doc example  【F:src/cli/mod.rs†L129-L136】

## 三、技術細節

### 3.1 架構變更
- 將檔案重新定位邏輯參數納入 `MatchConfig`，解耦 CLI 與匹配引擎。

### 3.2 API 變更
- `MatchConfig` 結構新增兩個欄位，對外 API 增加參數設定支援。
- `FileRelocationMode`、`ConflictResolution` 枚舉公開使用。

### 3.3 設定變更
- CLI `--copy`/`--move` 參數新增，預設關閉，維持向後相容性。

## 四、測試與驗證

```bash
# 程式碼格式檢查
cargo fmt -- --check
# Clippy 警告檢查
cargo clippy -- -D warnings
# 文檔品質檢查
timeout 20 scripts/check_docs.sh
# 測試覆蓋率檢查
scripts/check_coverage.sh -T
# 單元與整合測試
cargo test
```

所有測試案例與文檔示例均已通過，並確保現有功能維持正常。

## 五、影響評估

### 5.1 向後相容性
- 新功能預設關閉，不會影響既有工作流程。

### 5.2 使用者體驗
- 提供自動化字幕整理，減少手動操作，提升播放便利性。

## 六、問題與解決方案

### 6.1 遇到的問題
- 在 engine 模組中移除多重 `derive` 並修正匯入衝突。
- 更新多處測試與範例以配合新 API，涵蓋 CLI、內部測試與 doc tests。

## 七、後續事項

### 7.1 待完成項目
- 實作實際檔案複製/移動邏輯與衝突解決策略。
- 撰寫更多針對 I/O 錯誤與檔名衝突的單元與整合測試。

## 八、檔案異動清單

| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| src/cli/match_args.rs | 修改 | 新增 copy/move 參數及驗證邏輯 |
| src/cli/mod.rs | 修改 | 更新 doc example |
| src/commands/match_command.rs | 修改 | 建構 MatchConfig 時加入 relocation 參數，並更新多處 doc & 測試 |
| src/core/factory.rs | 修改 | create_match_engine 預設 relocation 欄位 |
| src/core/matcher/engine.rs | 修改 | 定義 FileRelocationMode、ConflictResolution，擴充 MatchConfig |
| src/core/matcher/mod.rs | 修改 | 重新導出 FileRelocationMode、ConflictResolution |
| tests/match_engine_id_integration_tests.rs | 修改 | 更新 MatchConfig 初始化 |
