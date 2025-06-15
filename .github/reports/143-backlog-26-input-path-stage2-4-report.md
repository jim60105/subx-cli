---
title: "Job Report: Backlog #26 - 輸入路徑參數功能實作 Stage 2~4"
date: "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
---

# Backlog #26 - 輸入路徑參數功能實作 Stage 2~4 工作報告

**日期**：$(date -u +"%Y-%m-%dT%H:%M:%SZ")  
**任務**：為 CLI 命令新增通用 `-i/--input` 及 `--recursive` 參數，並補充對應單元及參數解析測試  
**類型**：Backlog  
**狀態**：部分完成

> [!TIP]
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.

## 一、任務概述

Backlog #26 要求在 `match`、`convert`、`sync`、`detect-encoding` 等命令中新增通用 `-i/--input` 參數以支援多輸入來源，並為尚未支援的命令新增 `--recursive` 參數以控制目錄掃描深度；同時需補足對應的單元測試與 CLI 參數解析測試。

本報告涵蓋 Stage 2~4：
- Stage 2：更新各命令 CLI 參數並加入對應測試
- Stage 3：重構 `convert` 命令以整合共用輸入處理邏輯
- Stage 4：新增 `InputPathHandler` 單元測試

## 二、實作內容

### 2.1 更新 CLI 參數結構（Stage 2）
- 在 `convert_args.rs`、`detect_encoding_args.rs`、`sync_args.rs`、`match_args.rs` 中新增 `input_paths: Vec<PathBuf>` 與 `recursive: bool` 參數
- 調整原有必填輸入參數為 `Option<PathBuf>`（以保留相容性）
- 補充 CLI 解析測試模組
  【F:src/cli/convert_args.rs†L25-L40】【F:src/cli/detect_encoding_args.rs†L20-L35】

```bash
# 參數解析測試示例
subx-cli convert -i dir1 -i file.srt --recursive --format vtt --keep-original
```

### 2.2 共用輸入處理模組與測試（Stage 4）
- 新增 `InputPathHandler`，提供路徑驗證、平掃/遞迴掃描與副檔名過濾邏輯
- 為 `InputPathHandler` 撰寫單元測試涵蓋檔案、目錄、遞迴與錯誤情境
  【F:src/cli/input_handler.rs†L1-L80】【F:tests/cli/input_handler_tests.rs†L1-L50】

```rust
let handler = InputPathHandler::from_args(&[path1, path2], recursive)?
    .with_extensions(&["srt", "ass"]);
let files = handler.collect_files()?;
```

### 2.3 重構 Convert 命令實作（Stage 3）
- 以 `get_input_handler()` 收集多檔案輸入
- 支援混合目錄與單檔批次處理
  【F:src/commands/convert_command.rs†L214-L282】

```rust
for input_path in handler.collect_files()? {
    converter.convert_file(&input_path, &output_path, &format_str).await?;
}
```

## 三、技術細節

### 3.1 架構變更
- 提取共用輸入邏輯到 `cli/input_handler.rs` 模組，提高可維護性與一致性

### 3.2 CLI API 變更
- `MatchArgs.path: PathBuf` → `Option<PathBuf>`；新增 `input_paths`、`recursive`
- `DetectEncodingArgs.file_paths` 保留；新增 `input_paths`、`recursive`，並互斥

## 四、測試與驗證

```bash
# 單元測試
cargo test tests/cli/input_handler_tests.rs

# CLI 解析測試
cargo test -- --nocapture test_convert_args_default_and_format test_detect_encoding_args_input_paths

# 其他測試
cargo fmt -- --check
cargo clippy -- -D warnings
```

## 五、影響評估

### 5.1 向後相容性
- 未使用 `-i` 時保持原有行為；CLI 位置參數可繼續使用舊參數模式

### 5.2 使用者體驗
- 統一多輸入來源 API，提升批次處理靈活性；新增 `--recursive` 控制深度

## 六、後續事項

### 6.1 待完成項目 (Stage 3~4 完整落實)
- 更新 `match_command`、`sync_command`、`detect_encoding_command` 實作以使用 `InputPathHandler`
- 調整相關整合測試以符合新參數結構

### 6.2 建議的下一步
- 分階段更新整合測試並持續驗證 CI 綠燈

## 八、檔案異動清單

| 檔案路徑                             | 異動類型 | 描述                      |
|-------------------------------------|---------|---------------------------|
| `src/cli/input_handler.rs`          | 新增     | 共用輸入路徑處理模組       |
| `src/cli/convert_args.rs`           | 修改     | 新增 `-i` / `--recursive` |
| `src/cli/detect_encoding_args.rs`   | 修改     | 新增 `-i` / `--recursive` |
| `src/cli/sync_args.rs`              | 修改     | 新增 `-i` / `--recursive` |
| `src/cli/match_args.rs`             | 修改     | 新增 `-i` / `--recursive` |
| `tests/cli/input_handler_tests.rs`  | 新增     | 單元測試                  |
| `src/commands/convert_command.rs`   | 修改     | 批次轉換整合 `get_input_handler()` |
