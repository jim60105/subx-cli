# 實作指導：專案基礎建設

## 概覽
本指導文件對應 [Product Backlog #01](01-project-foundation.md)，提供詳細的實作步驟和最佳實踐。

## 實作順序

### 步驟 1: 初始化 Rust 專案
```bash
# 建立新的 Rust 專案
cargo new subx --bin
cd subx

# 驗證基本編譯
cargo check
```

### 步驟 2: 設定 Cargo.toml
```toml
[package]
name = "subx"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "智慧字幕處理 CLI 工具"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/subx"
keywords = ["subtitle", "cli", "ai", "video"]
categories = ["command-line-utilities", "multimedia"]

[dependencies]
# CLI 框架
clap = { version = "4.4", features = ["derive", "cargo"] }
clap_complete = "4.4"

# 非同步執行時
tokio = { version = "1.0", features = ["full"] }

# 錯誤處理
anyhow = "1.0"
thiserror = "1.0"

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# HTTP 客戶端
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# 日誌
log = "0.4"
env_logger = "0.10"

# 用戶介面
colored = "2.0"
indicatif = "0.17"

[dev-dependencies]
tokio-test = "0.4"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.8"

[[bin]]
name = "subx"
path = "src/main.rs"
```

### 步驟 3: 建立目錄結構
```bash
mkdir -p src/{cli,core/{matcher,formats,sync},services/{ai,audio}}
touch src/{main.rs,lib.rs,error.rs,config.rs}
touch src/cli/mod.rs
touch src/core/{mod.rs,matcher/mod.rs,formats/mod.rs,sync/mod.rs}
touch src/services/{mod.rs,ai/mod.rs,audio/mod.rs}
```

### 步驟 4: 實作基礎錯誤處理
建立 `src/error.rs`:
```rust
use thiserror::Error;

/// SubX 應用程式的主要錯誤類型
#[derive(Error, Debug)]
pub enum SubXError {
    /// IO 相關錯誤
    #[error("IO 錯誤: {0}")]
    Io(#[from] std::io::Error),
    
    /// 配置錯誤
    #[error("配置錯誤: {message}")]
    Config { message: String },
    
    /// 字幕格式錯誤
    #[error("字幕格式錯誤: {format} - {message}")]
    SubtitleFormat { format: String, message: String },
    
    /// AI 服務錯誤
    #[error("AI 服務錯誤: {0}")]
    AiService(#[from] reqwest::Error),
    
    /// 音訊處理錯誤
    #[error("音訊處理錯誤: {message}")]
    AudioProcessing { message: String },
    
    /// 文件匹配錯誤
    #[error("文件匹配錯誤: {message}")]
    FileMatching { message: String },
    
    /// 一般錯誤
    #[error("未知錯誤: {0}")]
    Other(#[from] anyhow::Error),
}

/// SubX 應用程式的 Result 類型
pub type SubXResult<T> = Result<T, SubXError>;

impl SubXError {
    /// 建立配置錯誤
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }
    
    /// 建立字幕格式錯誤
    pub fn subtitle_format<S1, S2>(format: S1, message: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self::SubtitleFormat {
            format: format.into(),
            message: message.into(),
        }
    }
    
    /// 建立音訊處理錯誤
    pub fn audio_processing<S: Into<String>>(message: S) -> Self {
        Self::AudioProcessing {
            message: message.into(),
        }
    }
    
    /// 建立文件匹配錯誤
    pub fn file_matching<S: Into<String>>(message: S) -> Self {
        Self::FileMatching {
            message: message.into(),
        }
    }
}
```

### 步驟 5: 建立 lib.rs
建立 `src/lib.rs`:
```rust
//! SubX - 智慧字幕處理 CLI 工具
//! 
//! 此函式庫提供了字幕文件的解析、格式轉換、AI 匹配和音訊同步功能。

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod services;

// 重新匯出主要類型
pub use error::{SubXError, SubXResult};

/// 應用程式版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 應用程式名稱
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
```

### 步驟 6: 建立基本 main.rs
建立 `src/main.rs`:
```rust
use anyhow::Result;
use env_logger;
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    env_logger::init();
    
    info!("啟動 SubX v{}", subx::VERSION);
    
    // TODO: 在後續 Backlog 中實作 CLI 介面
    println!("🎬 SubX - 智慧字幕處理工具");
    println!("版本: {}", subx::VERSION);
    println!("狀態: 基礎架構已建立 ✅");
    
    Ok(())
}
```

### 步驟 7: 設定模組骨架

建立 `src/cli/mod.rs`:
```rust
//! CLI 介面模組
//! 
//! 此模組將在 Backlog #02 中實作

// TODO: 在 Backlog #02 中實作 CLI 指令結構
```

建立 `src/config.rs`:
```rust
//! 配置管理模組
//! 
//! 此模組將在 Backlog #03 中實作

use crate::SubXResult;

// TODO: 在 Backlog #03 中實作配置結構
```

建立 `src/core/mod.rs`:
```rust
//! 核心功能模組

pub mod formats;
pub mod matcher; 
pub mod sync;

// TODO: 在後續 Backlogs 中實作核心功能
```

### 步驟 8: 開發工具設定

建立 `rustfmt.toml`:
```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

建立 `clippy.toml`:
```toml
# 嚴格的 clippy 設定
avoid-breaking-exported-api = false
```

建立 `.gitignore`:
```gitignore
# Rust
/target/
Cargo.lock

# IDE
.vscode/
.idea/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# 測試和暫存文件
/test_data/
/temp/
*.tmp

# 日誌文件
*.log

# 環境變數
.env
.env.local
```

### 步驟 9: 基本 CI/CD 設定

建立 `.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: 安裝 Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    
    - name: 快取依賴
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target/
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: 格式檢查
      run: cargo fmt --all -- --check
    
    - name: Clippy 檢查
      run: cargo clippy -- -D warnings
    
    - name: 執行測試
      run: cargo test
    
    - name: 編譯檢查
      run: cargo build --verbose
```

### 步驟 10: 驗證設定

```bash
# 檢查格式
cargo fmt --check

# 執行 clippy
cargo clippy -- -D warnings

# 編譯專案
cargo build

# 執行程式
cargo run

# 執行測試 (目前沒有測試)
cargo test
```

## 完成檢查清單

驗證以下項目已完成：

- [ ] Cargo.toml 配置正確且包含所有必要依賴
- [ ] 目錄結構按照架構圖建立
- [ ] 錯誤處理架構實作完成
- [ ] 基本模組骨架建立
- [ ] 開發工具配置 (rustfmt, clippy)
- [ ] Git 設定和 .gitignore
- [ ] CI/CD 基礎管道
- [ ] 程式可以成功編譯和執行
- [ ] 所有 clippy 警告已解決

## 下一步

完成此階段後，可以進入 [Product Backlog #02: CLI 介面框架](02-cli-interface.md)。

## 常見問題

### Q: 依賴版本衝突
A: 使用 `cargo tree` 檢查依賴樹，必要時鎖定特定版本。

### Q: 編譯錯誤
A: 檢查 Rust 版本是否為 1.75+，確保所有依賴正確安裝。

### Q: CI/CD 失敗
A: 檢查 GitHub Actions 設定，確保所有必要的 secrets 已配置。
