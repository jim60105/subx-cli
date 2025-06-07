# Product Backlog #01: 專案基礎建設

## 領域範圍
專案初始化、基礎架構設定、開發環境建構

## 完成項目

### 1. Rust 專案初始化
- [ ] 建立 Cargo.toml 配置
- [ ] 設定專案 metadata (名稱、版本、作者、描述)
- [ ] 配置 edition = "2021"
- [ ] 設定基本相依套件版本

### 2. 目錄結構建立
```
src/
├── main.rs
├── cli/
│   └── mod.rs
├── core/
│   ├── mod.rs
│   ├── matcher/
│   ├── formats/
│   └── sync/
├── services/
│   ├── mod.rs
│   ├── ai/
│   └── audio/
├── error.rs
├── config.rs
└── lib.rs
```

### 3. 基礎相依套件設定
- [ ] CLI 框架: `clap = { version = "4.4", features = ["derive", "cargo"] }`, `clap_complete = "4.4"`
- [ ] 異步運行時: `tokio = { version = "1.0", features = ["full"] }`
- [ ] 錯誤處理: `anyhow = "1.0"`, `thiserror = "1.0"`
- [ ] 序列化: `serde = { version = "1.0", features = ["derive"] }`, `serde_json = "1.0"`, `toml = "0.8"`
- [ ] HTTP 客戶端: `reqwest = { version = "0.11", features = ["json", "rustls-tls"] }`
- [ ] 日誌: `log = "0.4"`, `env_logger = "0.10"`
- [ ] 用戶介面: `colored = "2.0"`, `indicatif = "0.17"`
- [ ] 開發相依套件: `tokio-test = "0.4"`, `assert_cmd = "2.0"`, `predicates = "3.0"`, `tempfile = "3.8"`

### 4. 開發工具設定
- [ ] 格式化配置: `rustfmt.toml`
- [ ] Lint 配置: `clippy.toml`
- [ ] Git 忽略文件: `.gitignore`
- [ ] GitHub Actions CI/CD 基礎設定

### 5. 錯誤處理架構
- [ ] 建立 `SubXError` enum
- [ ] 實作 `thiserror::Error` trait
- [ ] 定義各模組特定錯誤類型
- [ ] 建立 `Result<T>` type alias

## 技術設計

### Cargo.toml 基礎配置
```toml
[package]
name = "subx-cli"
version = "0.1.0"
edition = "2021"
authors = ["CHEN, CHUN <jim60105@gmail.com>"]
description = "智慧字幕處理 CLI 工具，使用 AI 技術自動匹配、重命名和處理字幕檔案"
license = "GPLv3"
repository = "https://github.com/jim60105/subx-cli"
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
name = "subx-cli"
path = "src/main.rs"
```

### 錯誤處理設計
```rust
// src/error.rs
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
    AiService(#[from] reqwest::Error), // 改為 from reqwest::Error
    
    /// 音訊處理錯誤
    #[error("音訊處理錯誤: {message}")]
    AudioProcessing { message: String },
    
    /// 文件匹配錯誤
    #[error("文件匹配錯誤: {message}")]
    FileMatching { message: String },
    
    /// 一般錯誤
    #[error("未知錯誤: {0}")]
    Other(#[from] anyhow::Error), // 新增 Other 錯誤類型
}

/// SubX 應用程式的 Result 類型
pub type SubXResult<T> = Result<T, SubXError>; // 改名為 SubXResult

// 新增輔助構造函式
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

### 主程式進入點
```rust
// src/main.rs
use anyhow::Result; // anyhow::Result for main
use env_logger;
use log::info;
// use subx::cli::run; // 假設 run 在 lib.rs 或 cli/mod.rs 中定義並匯出

#[tokio::main]
async fn main() -> Result<()> { // main 函式返回 anyhow::Result
    // 初始化日誌
    env_logger::init();
    
    info!("啟動 SubX v{}", subx::VERSION); // 假設 subx::VERSION 在 lib.rs 定義

    // 這裡的 run() 呼叫假設是 CLI 的主要執行邏輯
    // if let Err(e) = subx::cli::run().await {
    //     eprintln!("錯誤: {}", e); // 錯誤處理將在 Backlog #09 中完善
    //     std::process::exit(1);
    // }
    
    // 暫時的輸出，直到 CLI 介面實作
    println!("🎬 SubX - 智慧字幕處理工具");
    println!("版本: {}", subx::VERSION); // 假設 subx::VERSION
    println!("狀態: 基礎架構已建立 ✅");


    Ok(())
}
```

## 驗收標準
1. `cargo check` 無錯誤
2. `cargo clippy` 無警告
3. 專案結構符合架構設計
4. 基本錯誤處理機制運作正常
5. CI/CD pipeline 基本設定完成

## 估計工時
2-3 天

## 相依性
無

## 風險評估
- 低風險：基礎設定工作
- 注意事項：確保相依套件版本相容性
