---
title: "Job Report: Backlog #26 - 輸入路徑參數功能實作 (階段3-4)"
date: "2025-06-15T22:17:57Z"
---

# Backlog #26 - 輸入路徑參數功能實作 (階段3-4) 工作報告

**日期**: 2025-06-15T22:17:57Z  
**任務**: 完成階段3 (命令實作更新) 與階段4 (測試實作)  
**類型**: Backlog  
**狀態**: 已完成

## 一、任務概述

本次工作在 SubX CLI 工具中，針對 backlog #26 的輸入路徑參數功能，完成：
- **階段3**：更新命令實作（Match 與 DetectEncoding），整合 InputPathHandler
- **階段4**：新增 InputPathHandler 單元測試與 CLI paring 測試

## 二、實作內容

### 2.1 更新 Match 命令實作
- 以 `MatchArgs::get_input_handler` 集中處理多重輸入路徑與遞迴選項
- 修改 `src/commands/match_command.rs`，整合命令行參數與 InputPathHandler 展開邏輯【F:src/commands/match_command.rs†L300-L325】

### 2.2 更新 DetectEncoding 命令實作
- 重構 `detect_encoding_command`，改為接受 `DetectEncodingArgs` 參數，透過 `args.get_file_paths()` 收集路徑
- 調整 `detect_encoding_command_with_config` 簽章與實作【F:src/commands/detect_encoding_command.rs†L190-L230】【F:src/commands/detect_encoding_command.rs†L250-L260】

### 2.3 新增 InputPathHandler 單元測試
- 建立 `tests/cli/input_handler_tests.rs` 測試檔案、目錄、混合輸入的展開與過濾行為【F:tests/cli/input_handler_tests.rs†L1-L80】

### 2.4 擴充 MatchArgs CLI 解析測試
- 新增 `-i` 參數多重使用與遞迴選項解析測試【F:src/cli/match_args.rs†L150-L170】

## 三、技術細節

### 3.1 架構變更
- 引入通用 `InputPathHandler`，統一處理多路徑、遞迴掃描與副檔名過濾
- 命令執行流程改為多路徑迭代，取代原本單一路徑處理

### 3.2 API 變更
- `detect_encoding_command` 與 `detect_encoding_command_with_config` 由接收 `&[String]` 改為 `&DetectEncodingArgs` 與 `&dyn ConfigService`

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo build
```

### 4.2 單元測試
```bash
cargo test -- --ignored  # 僅執行本次新增的單元測試
```

## 五、影響評估

### 5.1 向後相容性
- 僅在指定 `-i` 時啟用新邏輯，預設行為不受影響

### 5.2 使用者體驗
- 支援多檔案與多目錄批次處理，操作更彈性直觀

## 六、後續事項

### 6.1 待完成項目
- [ ] 完成 Sync 與 Convert 批次模式命令實作及對應測試

### 6.2 建議的下一步
- 在階段5 中完善文件與效能優化，並補齊整合測試

---

**實作狀態**：已完成  
**預估工時**：4 小時  
**優先級**：中等
