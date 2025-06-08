---
title: "Job Report: Backlog #16.4 - 檔案編碼自動檢測實作"
date: "2025-06-08T19:58:27Z"
---

# Backlog #16.4 - 檔案編碼自動檢測實作 工作報告

**日期**：2025-06-08T19:58:27Z  
**任務**：根據 Backlog #16.4 規範，新增檔案編碼自動檢測、轉換與 CLI 命令整合  
**類型**：Backlog  
**狀態**：已完成

> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"`.

## 一、任務概述

為提升多語言字幕檔案處理的可靠性，SubX 需自動檢測並轉換不同編碼格式，以免因非 UTF-8 檔案造成亂碼問題。本次依 Backlog #16.4 規範，實作編碼檢測器、轉換器、統計分析器，並整合到格式引擎與 CLI 指令。

## 二、實作內容

### 2.1 擴充配置系統
- 新增 `formats.encoding_detection_confidence` 設定欄位，定義檢測信心度閾值【F:src/config.rs†L209-L214】【F:src/config.rs†L268-L273】。
- 驗證配置不為空並在 0.0-1.0 範圍內【F:src/config/validator.rs†L148-L155】。
- 在 partial merge 流程中加入此欄位【F:src/config/partial.rs†L63-L70】【F:src/config/partial.rs†L193-L200】。

### 2.2 新增編碼模組
- 刪除舊版 `src/core/formats/encoding.rs`，改以模組化目錄結構實作【F:src/core/formats/encoding.rs†-L1】。
- 建立 `encoding/{mod,charsei,detector,converter,analyzer}` 子模組，實作自動編碼檢測、轉換與統計分析【F:src/core/formats/encoding/mod.rs†L1-L8】【F:src/core/formats/encoding/charset.rs†L1-L15】【F:src/core/formats/encoding/detector.rs†L1-L20】【F:src/core/formats/encoding/converter.rs†L1-L18】【F:src/core/formats/encoding/analyzer.rs†L1-L20】。

### 2.3 格式引擎整合
- 新增 `FormatManager::read_subtitle_with_encoding_detection` 及 `get_encoding_info` 方法，統合檔案編碼檢測與 UTF-8 轉換【F:src/core/formats/manager.rs†L65-L85】。

### 2.4 CLI 指令新增
- 新增 `detect-encoding` 子命令參數定義【F:src/cli/detect_encoding_args.rs†L1-L10】。
- 更新 `src/cli/mod.rs`，將命令列加入 DetectEncoding 並整合對應執行邏輯【F:src/cli/mod.rs†L15-L25】【F:src/cli/mod.rs†L75-L85】。
- 實作 `detect_encoding_command`，列印檔案編碼結果與樣本文字輸出【F:src/commands/detect_encoding_command.rs†L1-L40】。

## 三、測試與驗證

### 3.1 單元測試
在 `src/core/formats/encoding/tests.rs` 中新增編碼檢測、轉換與分析器測試，並初始化配置管理器【F:src/core/formats/encoding/tests.rs†L1-L30】。

### 3.2 整合測試
新增 `tests/encoding_integration_tests.rs`，涵蓋檔案編碼檢測與端對端轉換流程【F:tests/encoding_integration_tests.rs†L1-L25】。

### 3.3 程式碼品質
```bash
cargo fmt -- --check
cargo clippy --all-features -- -D warnings
cargo test
```
結果：所有測試通過，無警告。

## 四、檔案異動清單
| 檔案路徑                                            | 異動類型 | 描述                       |
|-----------------------------------------------------|---------|----------------------------|
| `src/config.rs`                                     | 修改    | 新增 encoding_detection_confidence 預設值 |
| `src/config/validator.rs`                           | 修改    | 驗證信心度範圍             |
| `src/config/partial.rs`                             | 修改    | 合併 partial 流程           |
| `src/core/formats/encoding.rs`                      | 刪除    | 移除舊版檔案               |
| `src/core/formats/encoding/`*                       | 新增    | 新增編碼檢測相關模組       |
| `src/core/formats/manager.rs`                       | 修改    | 整合編碼檢測函式           |
| `src/cli/detect_encoding_args.rs`                   | 新增    | 定義 CLI 參數              |
| `src/cli/mod.rs`                                    | 修改    | 將命令列註冊到 CLI         |
| `src/commands/detect_encoding_command.rs`           | 新增    | 實作檔案編碼檢測命令       |
| `tests/encoding_integration_tests.rs`               | 新增    | 編碼整合測試               |
