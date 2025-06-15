---
title: "Job Report: Backlog #26 - 建置錯誤與回滾建議"
date: "2025-06-15T21:24:33Z"
---

# Backlog #26 - 建置錯誤與回滾建議 工作報告

**日期**：2025-06-15T21:24:33Z  
**任務**：依據 Backlog #26 Design §1–§3 初步實作輸入參數與錯誤擴充後，發現命令實作尚未整合導致建置失敗，建議暫回滾至可編譯狀態  
**類型**：Backlog  
**狀態**：部分完成

> [!TIP]
> 本報告時間請使用 `date -u +"%Y-%m-%dT%H:%M:%SZ"` 以確保 UTC 格式。

## 一、任務概述
依據 .github/plans/backlogs/26-input-path-parameter-implementation.md 的設計，先行完成以下三個階段：

- **Design §1**：實作共用 InputPathHandler 模組。
- **Design §2**：於 match/convert/sync/detect-encoding 等 CLI 命令新增 `-i/--input` 參數與 `--recursive` 參數，並撰寫對應整合方法。
- **Design §3**：擴充 SubXError 增加錯誤類別（NoInputSpecified、InvalidPath、PathNotFound、DirectoryReadError、InvalidSyncConfiguration、UnsupportedFileType）。

在完成上述設計階段後，執行 `cargo build` 時發現命令實作尚未同步更新，產生大量型別與參數不符錯誤。為避免核心功能斷裂，建議先回滾至先前可編譯版本，並依序完成命令實作整合。

## 二、實作內容

### 2.1 共用輸入處理模組
- 新增通用輸入路徑處理 `InputPathHandler`，支援檔案/目錄掃描與副檔名過濾。  
- 刪除未使用的 `std::io` import。  
- 更新檔案：【F:src/cli/input_handler.rs†L1-L100】

### 2.2 CLI 參數設計與整合方法
- MatchArgs、ConvertArgs、SyncArgs、DetectEncodingArgs 新增 `input_paths`、`recursive` 參數與對應處理函式。  
- CLI 模組匯出與主命令 dispatch 更新：DetectEncoding 改為呼叫 `args.get_file_paths()` 取得標準化路徑。  
- 更新檔案：
  - 【F:src/cli/match_args.rs†L54-L88】
  - 【F:src/cli/convert_args.rs†L31-L45】【F:src/cli/convert_args.rs†L135-L150】
  - 【F:src/cli/sync_args.rs†L91-L130】【F:src/cli/sync_args.rs†L231-L280】
  - 【F:src/cli/detect_encoding_args.rs†L94-L110】
  - 【F:src/cli/mod.rs†L270-L280】

### 2.3 錯誤擴充
- 增加 SubXError 變體：NoInputSpecified、InvalidPath、PathNotFound、DirectoryReadError、InvalidSyncConfiguration、UnsupportedFileType。  
- 更新檔案：【F:src/error.rs†L117-L143】

## 三、技術細節

### 3.1 當前問題
- CLI 命令尚未使用新的參數及 InputPathHandler，導致參數與實作不符，建置失敗。  

### 3.2 回滾建議
- 暫先回滾命令執行邏輯至先前可編譯版本，待下一階段（Phase 3）完成命令實作整合後再恢復。

## 四、後續事項

### 4.1 待完成項目
- [ ] 回滾命令實作，先確保 `cargo build` 通過  
- [ ] Phase 3：整合命令實作至 InputPathHandler 與 CLI 參數設計  
- [ ] Phase 4：完成測試並驗證

### 4.2 建議下一步
- 按照 Backlog #26 實作步驟依序完成命令整合，再次執行建置與測試。

## 五、檔案異動清單
| 檔案路徑 | 異動類型 | 描述 |
|---------|----------|------|
| `src/cli/input_handler.rs` | 修改 | 加入 InputPathHandler，移除 std::io |
| `src/cli/match_args.rs` | 修改 | 新增 `-i/--input`、`recursive` |
| `src/cli/convert_args.rs` | 修改 | 新增 `-i/--input`、`recursive` |
| `src/cli/sync_args.rs` | 修改 | 新增 `-i/--input`、`recursive` |
| `src/cli/detect_encoding_args.rs` | 修改 | 新增 `-i/--input`、`recursive` |
| `src/cli/mod.rs` | 修改 | 更新 DetectEncoding 呼叫邏輯 |
| `src/error.rs` | 修改 | 擴充 SubXError 新增錯誤變體 |
| `.github/reports/142-backlog-26-build-errors-and-rollback-proposal-report.md` | 新增 | 本次工作報告 |
