---
title: "Job Report: Backlog #01 - 專案基礎建設"
date: "2025-06-04T22:24:57Z"
---

# Backlog #01 - 專案基礎建設 工作報告

**日期**：2025-06-04T22:24:57Z  
**任務**：初始化 SubX 專案的基礎架構

## 一、Rust 專案初始化

- 建立 `Cargo.toml`，內容包含：
  - package metadata：名稱、版本、edition、作者、描述、license、repository
  - 主要相依套件：clap、clap_complete、tokio、anyhow、thiserror、serde、serde_json、toml、reqwest、log、env_logger、colored、indicatif
  - 開發相依套件：tokio-test、assert_cmd、predicates、tempfile
```toml
[package]
name = "subx"
version = "0.1.0"
edition = "2021"
...
[dependencies]
clap = { version = "4.4", features = ["derive", "cargo"] }
...
```
【F:Cargo.toml†L1-L68】

## 二、目錄結構與 Stub 檔案

依照設計文件建立以下目錄與檔案，確保未完成模組皆編譯通過：
```text
src/
├── main.rs         # CLI 入口
├── lib.rs          # Library root (包含 VERSION 常數)
├── cli/
│   └── mod.rs      # CLI 參數與指令定義
├── core/
│   ├── mod.rs
│   ├── matcher/    # AI 匹配引擎
│   │   └── mod.rs
│   ├── formats/    # 字幕格式引擎
│   │   └── mod.rs
│   └── sync/       # 音訊同步引擎
│       └── mod.rs
├── services/
│   ├── mod.rs
│   ├── ai/         # AI 服務整合
│   │   └── mod.rs
│   └── audio/      # 音訊處理封裝
│       └── mod.rs
├── error.rs        # SubXError 定義
└── config.rs       # 配置管理 stub
```
【F:.github/instructions/01-project-foundation.md†L16-L31】

## 三、錯誤處理與版本管理

- 在 `src/error.rs` 實作 `SubXError` enum 及 `SubXResult<T>` alias，封裝 IO、配置、字幕解析、AI 服務、音訊處理等錯誤類型【F:src/error.rs†L1-L54】
- 在 `src/lib.rs` 定義 `VERSION` 常數，透過 `env!("CARGO_PKG_VERSION")` 自動對應 Cargo.toml 版本【F:src/lib.rs†L1-L8】

## 四、開發工具與 CI 設定

- 已新增 `.gitignore`、`rustfmt.toml`、`clippy.toml`，統一格式與 Lint 規範【F:.gitignore†L1-L12】【F:rustfmt.toml†L1-L2】【F:clippy.toml†L1-L2】
- 已新增 GitHub Actions 流水線 `rust-ci-test.yml`，包含：
  1. 安裝 stable toolchain (+ rustfmt、clippy)
  2. 快取 cargo registry/index/target 目錄
  3. 檢查程式碼格式 (`cargo fmt -- --check`)
  4. Clippy Lint (`cargo clippy -- -D warnings`)
  5. Build & Test (`cargo build`、`cargo test`)
【F:.github/workflows/rust-ci-test.yml†L1-L50】

## 五、驗收標準

1. `cargo fmt -- --check` 無變動
2. `cargo clippy -- -D warnings` 無警告
3. `cargo check` / `cargo build` / `cargo test` 全部通過
4. 專案目錄結構與 stub 檔案符合設計文件

## 六、後續工作

- Backlog #02：實作 CLI 參數解析與指令執行邏輯
- Backlog #03：完成配置管理系統（讀寫 TOML、環境變數、命令設定）
