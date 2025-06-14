---
title: "Job Report: Backlog #20 - Rust Source Code Documentation"
date: "2025-06-09T18:16:16Z"
---

# Backlog #20 - Rust Source Code Documentation 工作報告

**日期**：2025-06-09T18:16:16Z  
**任務**：為 SubX 專案建立 rustdoc 撰寫指南並整合文件品質檢查流程  
**類型**：Backlog  
**狀態**：已完成

## 一、實作內容

### 1.1 建立 rustdoc 撰寫指南
- 新增文件 `docs/rustdoc-guidelines.md`，定義 rustdoc 格式與範例  
- 檔案變更：【F:docs/rustdoc-guidelines.md†L1-L34】

### 1.2 更新 Cargo.toml 文件設定
- 新增 `[package.metadata.docs.rs]` 以設定 docs.rs 自動發布  
- 新增 `[lints.rustdoc]` 配置以啟用破損連結與缺失文件警告  
- 檔案變更：【F:Cargo.toml†L13-L20】

### 1.3 整合文件檢查至 CI 流程
- 在 `.github/workflows/build-test-audit-coverage.yml` 中新增「Check documentation」及「Test documentation examples」步驟  
- 檔案變更：【F:.github/workflows/build-test-audit-coverage.yml†L55-L64】

## 二、驗證

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test && cargo test --doc
```
結果：通過

## 三、後續事項

- 後續依照 `docs/rustdoc-guidelines.md` 完成各模組與函式的文件撰寫
---
**檔案異動**：
- docs/rustdoc-guidelines.md
- Cargo.toml
- .github/workflows/build-test-audit-coverage.yml
