---
title: "Job Report: Bug Fix #170 - Sync 指令參數彈性與直覺性問題"
date: "2025-06-19T01:56:45Z"
---

# Bug Fix #170 - Sync 指令參數彈性與直覺性問題 工作報告

**日期**：2025-06-19T01:56:45Z  
**任務**：修正 `subx-cli sync` 指令參數解析與自動配對行為，支援位置參數與單檔/目錄自動推論  
**類型**：Bug Fix  
**狀態**：已完成

## 一、任務概述

依照 Backlog #24 中定義，`subx-cli sync` 指令在處理參數時過於嚴格，無法接受單一檔案或目錄的直覺用法，且自動推論與批次模式行為與文件不符。本次任務目標為：
- 支援以位置參數直接指定檔案或目錄
- 強化單一檔案模式的自動配對邏輯
- 修正 Batch 模式僅以 `-b` 啟用
- 移除過時的 `range`/`threshold` 參數
- 同步更新測試與文件範例

## 二、實作內容

### 2.1 CLI 參數彈性調整與刪除過時欄位
- 新增 `positional_paths: Vec<PathBuf>` 欄位，收集無需 `-i` 的位置參數【F:src/cli/sync_args.rs†L46-L61】
- 移除 `range`、`threshold` 參數與相關 `#[deprecated]` 註解【F:src/cli/sync_args.rs†L112-L125】

```rust
#[arg(value_name = "PATH", num_args = 0..)]
pub positional_paths: Vec<PathBuf>,
// 移除過時欄位 range, threshold
```

### 2.2 參數驗證與自動配對邏輯強化
- 更新 `validate`、`validate_compat`，允許位置參數輸入與單檔模式檢驗【F:src/cli/sync_args.rs†L152-L172】【F:src/cli/sync_args.rs†L200-L215】
- 調整 `get_input_handler`，自動將位置參數併入處理清單【F:src/cli/sync_args.rs†L82-L94】
- 重構 `get_sync_mode`；批次模式僅依 `--batch`，並在單檔模式中依據副檔名自動推論配對檔案【F:src/cli/sync_args.rs†L180-L240】

### 2.3 測試同步與修正
- 移除所有測試中對 `range`/`threshold` 的引用，加入 `positional_paths` 預設值【F:tests/sync_cli_integration_tests.rs†L22-L60】【F:tests/sync_command_comprehensive_tests.rs†L3-L20】【F:src/commands/sync_command.rs†L224-L260】

### 2.4 文件範例更新
- 在 `README.md`、`README.zh-TW.md` 中新增位置參數使用說明與範例【F:README.md†L136-L157】【F:README.md†L370-L380】【F:README.zh-TW.md†L130-L151】【F:README.zh-TW.md†L360-L370】

### 2.5 移除 App API 過時範例
- 修正 `App::sync_files` 與 `sync_files_with_offset` 中的過時參數範例【F:src/lib.rs†L433-L460】【F:src/lib.rs†L498-L540】

## 三、測試與驗證

### 3.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
```

### 3.2 單元與整合測試
```bash
cargo nextest run --profile ci
```

## 四、影響評估

### 向後相容性
- 刪除過時參數僅影響內部實作，CLI 使用者透過新位置參數介面獲得相同或更佳體驗。

### 使用者體驗
- 可用直覺位置參數指定檔案/資料夾，自動推論檔案對應，提升使用便捷性。

## 五、後續事項

- 無，所有既定驗收標準已達成。

## 六、檔案異動清單

| 檔案路徑                             | 異動類型 | 描述                         |
|-------------------------------------|----------|------------------------------|
| `src/cli/sync_args.rs`              | 修改     | 新增位置參數、強化驗證與邏輯  |
| `src/commands/sync_command.rs`      | 修改     | 測試移除過時欄位              |
| `src/lib.rs`                        | 修改     | 調整 App 範例、移除過時參數    |
| `tests/sync_cli_integration_tests.rs` | 修改   | 同步測試結構                  |
| `tests/sync_command_comprehensive_tests.rs` | 修改 | 同步測試結構              |
| `README.md`                         | 修改     | 新增參數說明與範例            |
| `README.zh-TW.md`                   | 修改     | 新增參數說明與範例            |
