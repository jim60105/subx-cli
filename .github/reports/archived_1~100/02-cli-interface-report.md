---
title: "Job Report: Backlog #02 - CLI 介面框架"
date: "2025-06-04T22:35:28Z"
---

# Backlog #02 - CLI 介面框架 工作報告

**日期**：2025-06-04T22:35:28Z  
**任務**：實作命令列介面架構，參數解析與子命令路由

## 一、相依套件更新

- 新增互動式提示套件 `dialoguer` 以支援未來互動式提示需求
  【F:Cargo.toml†L37-L39】

## 二、CLI 參數結構檔案新增

- `src/cli/match_args.rs`：定義 Match 子命令參數結構
  【F:src/cli/match_args.rs†L1-L26】
- `src/cli/convert_args.rs`：定義 Convert 子命令參數結構與 `OutputSubtitleFormat` 枚舉
  【F:src/cli/convert_args.rs†L1-L35】
- `src/cli/sync_args.rs`：定義 Sync 子命令參數結構與 `SyncMethod` 枚舉
  【F:src/cli/sync_args.rs†L1-L23】
- `src/cli/config_args.rs`：定義 Config 子命令與子指令 `Set`/`Get`/`List`/`Reset`
  【F:src/cli/config_args.rs†L1-L20】
- `src/cli/generate_completion_args.rs`：定義 GenerateCompletion 子命令參數
  【F:src/cli/generate_completion_args.rs†L1-L11】
- `src/cli/ui.rs`：整合 `colored`、`indicatif` 提供彩色輸出與進度條工具函式
  【F:src/cli/ui.rs†L1-L22】

## 三、CLI 主模組實作

- 更新 `src/cli/mod.rs`：
  - 匯入並公開各子模組與參數結構
  - 定義 `Cli` 主結構與 `Commands` 枚舉
  - 實作 `run()`，路由至對應子命令，並實作 `generate-completion` 腳本產生
  【F:src/cli/mod.rs†L1-L68】

## 四、主程式呼叫 CLI

- 更新 `src/main.rs`：
  - 取消原有暫時輸出，改以 `subx::cli::run()` 執行 CLI 主邏輯並處理錯誤
  【F:src/main.rs†L13-L19】

## 五、README 更新

- 在「命令參考」中新增 `config` 與 `generate-completion` 支援示例
  【F:README.md†L169-L182】

## 六、後續事項

- 確認 CLI 參數解析與幫助信息正確顯示 (驗收標準)
- Backlog #03：配置管理系統實作
- Backlog #09：與核心邏輯整合命令執行
