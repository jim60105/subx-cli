---
title: "Job Report: Backlog #26 - 輸入路徑參數 (-i) 功能實作"
date: "2025-06-15T20:40:04Z"
---

# Backlog #26 - 輸入路徑參數 (-i) 功能實作 工作報告

**日期**：2025-06-15T20:40:04Z  
**任務**：實作通用 `-i/--input` 參數與遞迴掃描功能，為 CLI 命令引入多重輸入路徑處理  
**類型**：Backlog  
**狀態**：部分完成

> [!TIP]
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述

Backlog #26 要求在 `match`、`convert`、`sync`、`detect-encoding` 等命令中新增通用的 `-i/--input` 參數，支援多次指定檔案或目錄；並在所有檔案目錄掃描中新增 `--recursive` 參數，預設只掃描一層子目錄。

本次實作主要完成以下前兩階段內容：
- 階段 1：建立 `InputPathHandler` 共用模組，實作路徑驗證、目錄平掃與遞迴掃描、檔案副檔名過濾
- 階段 2：更新 CLI 參數定義，於 `match_args`、`detect_encoding_args` 中新增 `-i`、`--recursive` 並提供 `get_input_handler` / `get_file_paths` 介面

## 二、實作內容

### 2.1 基礎設施建立：InputPathHandler 模組
- 新增 `src/cli/input_handler.rs`，定義 `InputPathHandler` 結構及方法
- 支援:
  - 路徑存在性驗證  【F:src/cli/input_handler.rs†L14-L26】【F:src/cli/input_handler.rs†L29-L37】
  - 檔案副檔名過濾  【F:src/cli/input_handler.rs†L47-L55】
  - 非遞迴與遞迴目錄掃描  【F:src/cli/input_handler.rs†L57-L78】【F:src/cli/input_handler.rs†L80-L100】

```rust
let handler = InputPathHandler::from_args(&[path1, path2], recursive)?
    .with_extensions(&["srt", "ass"]);
let files = handler.collect_files()?;
```

### 2.2 CLI 參數更新：Match & DetectEncoding
- 於 `src/cli/match_args.rs` 新增 `input_paths: Vec<PathBuf>` 和 `recursive: bool` 參數
- 調整 `path` 為 `Option<PathBuf>` 並實作 `get_input_handler()` 取得共用處理器
- 更新測試，保持原有預設值行為並驗證新參數預設狀態
【F:src/cli/match_args.rs†L21-L30】【F:src/cli/match_args.rs†L78-L100】【F:src/cli/match_args.rs†L158-L175】

### 2.3 CLI 參數更新：DetectEncoding
- 於 `src/cli/detect_encoding_args.rs` 新增 `input_paths: Vec<PathBuf>` 和 `recursive: bool` 參數，與 `file_paths` 互斥
- 實作 `get_file_paths()` 回傳完整待處理路徑列表
【F:src/cli/detect_encoding_args.rs†L4-L10】【F:src/cli/detect_encoding_args.rs†L45-L75】

## 三、技術細節

### 3.1 架構變更
- 新增 `cli::input_handler` 模組，將共用路徑解析邏輯集中管理，提升可維護性與重用性

### 3.2 CLI API 變更
- `MatchArgs.path: PathBuf` → `Option<PathBuf>`；新增 `MatchArgs.input_paths`、`recursive`
- `DetectEncodingArgs.file_paths: Vec<String>` 保留；新增 `input_paths`、`recursive`，並互斥

### 3.3 錯誤類型擴充
- 擴充 `SubXError`，新增路徑相關錯誤與輸入未指定錯誤
【F:src/error.rs†L67-L95】

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
# cargo clippy -- -D warnings  # (略)
cargo build
cargo test -- --nocapture tests/cli/input_handler_tests.rs tests/cli/detect_encoding_args_tests.rs src/cli/match_args.rs
```

### 4.2 功能測試
- 單元測試 `InputPathHandler` 各種檔案與目錄掃描場景
  【F:tests/cli/input_handler_tests.rs†L1-L28】
- CLI 參數解析測試：`match`、`detect-encoding` 子命令參數行為
  【F:src/cli/match_args.rs†L158-L192】【F:tests/cli/detect_encoding_args_tests.rs†L1-L20】

## 五、影響評估

### 5.1 向後相容性
- 預設行為不變：未使用 `-i` 時自動沿用舊有參數
- CLI 參數位置調整後，原有使用者腳本需將 `path` 位置改為第一參數或使用 `-i`

### 5.2 使用者體驗
- 提供統一輸入路徑介面，易於批量處理多個資料夾或檔案
- 增加 `--recursive` 參數，操作更直覺一致

## 六、問題與解決方案

### 6.1 遇到的問題
- 欄位順序調整導致部分解析測試需更新
  **解決**：同步更新相應測試案例

### 6.2 技術債務
- 尚未整合命令實作中的路徑處理，須於 Stage 3 完成整合

## 七、後續事項

### 7.1 待完成項目
- Stage 3: 更新 `match_command`、`convert_command`、`sync_command`、`detect_encoding_command` 以採用 `InputPathHandler`
- Stage 4~5: 整合實作、文件更新與更詳盡整合測試

### 7.2 相關任務
- Backlog #26 整合命令實作與文件更新 (後續)

## 八、檔案異動清單

| 檔案路徑                                  | 異動類型 | 描述                         |
|------------------------------------------|---------|------------------------------|
| `src/cli/input_handler.rs`               | 新增     | 共用輸入路徑處理模組         |
| `src/cli/match_args.rs`                  | 修改     | `-i`、`--recursive` 參數與 API |
| `src/cli/detect_encoding_args.rs`        | 修改     | `-i`、`--recursive` 參數與 API |
| `src/cli/mod.rs`                         | 修改     | 匯出 `InputPathHandler`     |
| `src/error.rs`                           | 修改     | 擴充路徑相關錯誤類型         |
| `tests/cli/input_handler_tests.rs`       | 新增     | `InputPathHandler` 單元測試  |
| ...                                     | ...     | 後續 Stage2 文件與範例更新   |
