---
title: "Job Report: Backlog #18 - 測試覆蓋率提升計畫"
date: "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
---

# Backlog #18 - 測試覆蓋率提升計畫 工作報告

**日期**：$(date -u +"%Y-%m-%dT%H:%M:%SZ")  
**任務**：第一階段 CLI 層級測試補強，包括主程式與參數模組測試

## 一、實作內容

### 1. CLI 主程式整合測試
- 新增 `tests/cli_integration_tests.rs`，驗證 `--version`、`--help`、錯誤指令及 `config show` 子命令行為

### 2. CLI 參數模組單元測試
- `src/cli/match_args.rs`：新增信心度參數值範圍 (0~100) 驗證，並補充預設值、解析與範圍錯誤測試【F:src/cli/match_args.rs†L11-L15】【F:src/cli/match_args.rs†L17-L26】【F:src/cli/match_args.rs†L28-L58】
- `src/cli/convert_args.rs`：對 `OutputSubtitleFormat` 派生 `PartialEq, Eq`，並補充預設值與解析測試【F:src/cli/convert_args.rs†L28-L32】【F:src/cli/convert_args.rs†L58-L85】

## 二、驗證

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

所有測試通過，無警告

## 三、後續事項

- 擴充其餘 CLI 參數模組及 UI 模組測試
- 開始指令層級測試提升 (Backlog #18 第二階段)

---
**檔案異動**：
```
tests/cli_integration_tests.rs
src/cli/match_args.rs
src/cli/convert_args.rs
```
