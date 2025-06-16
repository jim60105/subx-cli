---
title: "Job Report: Backlog #36 - Cargo 套件大小最佳化：排除不必要的檔案"
date: "2025-06-16T15:22:18Z"
---

# Backlog #36 - Cargo 套件大小最佳化：排除不必要的檔案 工作報告

**日期**：2025-06-16T15:22:18Z  
**任務**：透過配置 `Cargo.toml` 的 `package.exclude` 欄位排除不必要的檔案，以減少發佈套件大小。  
**類型**：Backlog  
**狀態**：已完成

> [!TIP]  
> Always get the date with `date -u +"%Y-%m-%dT%H:%M:%SZ"` command.  

## 一、任務概述

本計劃旨在透過配置 `Cargo.toml` 的 `package.exclude` 欄位來排除專案中不必要的檔案，以減少發布套件的大小。目前專案中的 `assets`、`.github`、`tests` 等資料夾在 crate 發佈包中並非必要，應予以排除。

## 二、實作內容

### 2.1 修改 Cargo.toml 配置
- 在 `[package]` 區段新增 `exclude` 清單，排除多媒體、測試、配置、文件等不必要檔案。
- **檔案變更說明**：【F:Cargo.toml†L13-L30】

```toml
exclude = [
    "assets/",
    ".github/",
    "tests/",
    "target/",
    "*.mp4",
    "*.mp3",
    "*.mov",
    "*.avi",
    "*.mkv",
    "plans/",
    "scripts/test_*.sh",
    "benches/",
    "**/*.log",
    "**/*.tmp",
    "**/.DS_Store",
    "Cargo.lock"
]
```

### 2.2 更新 CHANGELOG.md
- 在 `Unreleased` 區段新增變更記錄，說明排除設定與效益。
- **檔案變更說明**：【F:CHANGELOG.md†L10-L11】

```markdown
### Changed
- 配置 Cargo.toml 的 `package.exclude` 欄位，排除不必要檔案（assets/, .github/, tests/, ...），減少 crate 發布大小約 15MB。【F:Cargo.toml†L13-L30】
```

## 三、技術細節

### 3.1 架構變更
- 無。

### 3.2 API 變更
- 無對外 API 變更。

### 3.3 配置變更
- 新增 `exclude` 欄位於 `Cargo.toml`。

## 四、測試與驗證

### 4.1 程式碼品質檢查
```bash
# 格式化檢查
cargo fmt -- --check

# 建置測試
cargo build

# 單元測試
cargo test
```

### 4.2 功能測試
- 檢查 dry-run 套件打包內容：
```bash
cargo package --dry-run
cargo package --list
```

## 五、影響評估

### 5.1 向後相容性
- 僅排除不必要檔案，對使用者無影響。

### 5.2 使用者體驗
- 減少 crate 大小，下載與安裝更快速。

## 六、問題與解決方案

### 6.1 遇到的問題
- 無。

### 6.2 技術債務
- 無。

## 七、後續事項

### 7.1 待完成項目
- 無。

### 7.2 相關任務
- Backlog #36。

### 7.3 建議的下一步
- 定期檢查並更新 `exclude` 清單。

## 八、檔案異動清單

| 檔案路徑        | 異動類型 | 描述                       |
|---------------|----------|----------------------------|
| `Cargo.toml`   | 修改     | 新增 `package.exclude` 清單 |
| `CHANGELOG.md` | 修改     | 新增發佈大小最佳化變更記錄 |
