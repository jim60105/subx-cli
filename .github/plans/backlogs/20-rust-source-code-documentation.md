# Product Backlog #20: Rust 原始碼文件化計畫

## 領域範圍
使用 Rust 標準 rustdoc 方法為整個專案的原始碼撰寫完整且高品質的英文文件，建立完善的 API 文件與範例

## 背景脈絡

SubX 專案已具備完整的核心功能實作，但缺乏標準化的程式碼文件。良好的 rustdoc 文件對於專案的長期維護、新開發者上手、以及開源社群貢獻都至關重要。

### 當前狀態
- 部分模組已有基本的 module-level 文件 (如 `config/manager.rs`)
- 大多數函式和結構體缺乏詳細的文件說明
- 缺少使用範例和程式碼片段
- 沒有統一的文件撰寫風格指南

### 目標
- 為所有 public API 撰寫完整的 rustdoc 文件
- 提供清晰的使用範例和程式碼片段
- 建立統一的文件撰寫風格和標準
- 確保文件與程式碼同步更新
- 提升專案的專業度和可維護性

## 完成項目

### 🎯 第一階段：文件標準制定與核心模組 (2-3 天)

#### 1.1 建立 rustdoc 文件撰寫指南
- [ ] **建立文件撰寫標準檔案**
  - [ ] 建立 `docs/rustdoc-guidelines.md` 文件撰寫指南
  - [ ] 定義模組、結構體、函式的文件格式標準
  - [ ] 建立錯誤處理文件的統一格式
  - [ ] 定義範例程式碼的撰寫規範

**文件撰寫指南內容**：
```markdown
# SubX Rustdoc Guidelines

## General Principles
- All documentation must be written in English
- Use clear, concise language
- Provide practical examples where applicable
- Document error conditions and panics
- Include links to related functions/types

## Module Documentation
- Start with a brief one-line summary
- Explain the module's purpose and scope
- List key types and functions
- Provide usage examples for complex modules

## Function Documentation
- Brief description of what the function does
- Document all parameters with their constraints
- Document return values and error conditions
- Include examples for non-trivial functions
- Use `# Errors`, `# Panics`, `# Examples` sections
```

#### 1.2 核心錯誤處理模組文件化
- [ ] **error.rs 完整文件化**
  - [ ] `SubXError` enum 各變體的詳細說明
  - [ ] 錯誤處理最佳實踐範例
  - [ ] 自訂錯誤類型的使用指南
  - [ ] 錯誤訊息國際化說明

**實作範例**：
```rust
/// Comprehensive error handling for SubX application operations.
/// 
/// This enum covers all possible error conditions that can occur during
/// subtitle processing, AI integration, audio analysis, and file operations.
/// Each variant provides specific context to help with debugging and
/// user-friendly error reporting.
/// 
/// # Examples
/// 
/// ```rust
/// use subx_cli::error::SubXError;
/// use subx_cli::Result;
/// 
/// fn process_subtitle() -> Result<()> {
///     // Processing logic here
///     Err(SubXError::SubtitleFormat {
///         format: "SRT".to_string(),
///         message: "Invalid timestamp format".to_string(),
///     })
/// }
/// ```
#[derive(Error, Debug)]
pub enum SubXError {
    /// I/O operation failed during file system access.
    /// 
    /// This variant wraps standard library I/O errors and provides
    /// context about file operations that failed.
    /// 
    /// # Common Causes
    /// - File permission issues
    /// - Disk space shortage
    /// - Network file system problems
    #[error("IO 錯誤: {0}")]
    Io(#[from] std::io::Error),
    
    // ... 其他錯誤變體的完整文件
}
```

#### 1.3 配置管理模組文件化
- [ ] **config/ 模組完整文件化**
  - [ ] `manager.rs` - 配置管理器核心功能
  - [ ] `source.rs` - 配置來源層級系統
  - [ ] `validator.rs` - 配置驗證邏輯
  - [ ] `partial.rs` - 部分配置更新機制

### 🔧 第二階段：CLI 與命令模組文件化 (3-4 天)

#### 2.1 CLI 介面文件化
- [ ] **cli/ 模組完整文件化**
  - [ ] `mod.rs` - CLI 主要結構與子命令定義
  - [ ] 各 args 檔案的參數說明與驗證規則
  - [ ] `ui.rs` - 使用者介面輔助函式
  - [ ] `table.rs` - 表格格式化工具

**實作範例**：
```rust
/// Command-line interface for SubX subtitle processing tool.
/// 
/// This module provides a comprehensive CLI interface for subtitle file
/// processing, including AI-powered matching, format conversion, audio
/// synchronization, and encoding detection.
/// 
/// # Architecture
/// 
/// The CLI is built using `clap` and follows a subcommand pattern:
/// - `match` - AI-powered subtitle file matching and renaming
/// - `convert` - Subtitle format conversion between different standards
/// - `sync` - Audio-subtitle synchronization and timing adjustment
/// - `detect-encoding` - Character encoding detection and conversion
/// - `config` - Configuration management and settings
/// 
/// # Examples
/// 
/// ```bash
/// # Basic subtitle matching
/// subx match /path/to/videos /path/to/subtitles
/// 
/// # Convert SRT to ASS format
/// subx convert --input file.srt --output file.ass --format ass
/// 
/// # Detect file encoding
/// subx detect-encoding *.srt
/// ```

/// Main CLI application structure.
/// 
/// This struct defines the top-level command-line interface for SubX,
/// including global options and subcommand routing.
#[derive(Parser, Debug)]
#[command(name = "subx-cli")]
#[command(about = "智慧字幕處理 CLI 工具")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}
```

#### 2.2 命令實作模組文件化
- [ ] **commands/ 模組完整文件化**
  - [ ] `match_command.rs` - AI 匹配演算法實作
  - [ ] `convert_command.rs` - 格式轉換引擎
  - [ ] `sync_command.rs` - 音訊同步引擎
  - [ ] `detect_encoding_command.rs` - 編碼檢測引擎
  - [ ] `config_command.rs` - 配置管理命令
  - [ ] `cache_command.rs` - 快取管理功能

### 🎵 第三階段：核心處理引擎文件化 (4-5 天)

#### 3.1 檔案處理核心文件化
- [ ] **core/ 模組完整文件化**
  - [ ] `file_manager.rs` - 安全檔案操作管理
  - [ ] `language.rs` - 語言檢測與處理
  - [ ] `formats/` 子模組 - 字幕格式處理引擎
  - [ ] `matcher/` 子模組 - 檔案匹配演算法
  - [ ] `parallel/` 子模組 - 並行處理框架
  - [ ] `sync/` 子模組 - 時間軸同步引擎

**實作範例**：
```rust
/// Safe file operation manager with rollback capabilities.
/// 
/// The `FileManager` provides atomic file operations with automatic
/// rollback functionality. It tracks all file creations and deletions,
/// allowing complete operation reversal in case of errors.
/// 
/// # Use Cases
/// 
/// - Batch file operations that need to be atomic
/// - Temporary file creation during processing
/// - Safe file replacement with backup
/// - Error recovery in multi-step file operations
/// 
/// # Examples
/// 
/// ```rust
/// use subx_cli::core::file_manager::FileManager;
/// use std::path::Path;
/// 
/// let mut manager = FileManager::new();
/// 
/// // Create a new file (tracked for rollback)
/// manager.create_file("output.srt", "content")?;
/// 
/// // Remove an existing file (backed up for rollback)
/// manager.remove_file("old_file.srt")?;
/// 
/// // If something goes wrong, rollback all operations
/// if error_occurred {
///     manager.rollback()?;
/// }
/// ```
/// 
/// # Safety
/// 
/// The manager ensures that:
/// - Created files are properly removed on rollback
/// - Removed files are restored from backup on rollback
/// - No partial state is left after rollback completion
pub struct FileManager {
    /// List of operations performed, in execution order.
    operations: Vec<FileOperation>,
}
```

#### 3.2 服務層模組文件化
- [ ] **services/ 模組完整文件化**
  - [ ] `ai/` 子模組 - AI 服務整合與 API 處理
  - [ ] `audio/` 子模組 - 音訊分析與處理引擎

### 🔍 第四階段：進階功能與範例完善 (2-3 天)

#### 4.1 AI 服務整合文件化
- [ ] **ai/ 服務模組詳細文件**
  - [ ] API 整合模式與錯誤處理
  - [ ] 不同 AI 提供商的使用範例
  - [ ] 配置選項與最佳實踐
  - [ ] 成本控制與限制說明

#### 4.2 音訊處理引擎文件化
- [ ] **audio/ 服務模組詳細文件**
  - [ ] 音訊分析演算法說明
  - [ ] 支援的音訊格式清單
  - [ ] 性能調優參數說明
  - [ ] 處理限制與建議

#### 4.3 整合範例與測試案例
- [ ] **建立完整的使用範例**
  - [ ] 在 `lib.rs` 中建立模組級範例
  - [ ] 建立 `examples/` 目錄實際使用案例
  - [ ] 各主要功能的端到端範例
  - [ ] 錯誤處理最佳實踐範例

### 📚 第五階段：文件整合與驗證 (1-2 天)

#### 5.1 文件品質檢查
- [ ] **實作自動化文件檢查**
  - [ ] 設定 `cargo doc` 的警告級別
  - [ ] 檢查所有 public API 是否有文件
  - [ ] 驗證文件範例的正確性
  - [ ] 確保文件連結的有效性

**CI/CD 整合方案**：

在現有的 `.github/workflows/build-test-audit-coverage.yml` 中新增文件檢查步驟：

```yaml
# 在 test job 中新增文件檢查步驟
- name: Check documentation
  run: |
    # 檢查文件完整性（警告轉為錯誤）
    cargo clippy -- -W missing_docs -D warnings
    
    # 檢查文件格式和連結正確性
    cargo doc --all-features --no-deps --document-private-items 2>&1 | \
      tee doc_output.log && ! grep -i "warning\|error" doc_output.log

- name: Test documentation examples
  run: cargo test --doc --verbose
```

**Cargo.toml 配置調整**：
```toml
# 專案已發佈至 crates.io，文件會自動在 docs.rs 上產生
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

# 啟用文件 linting
[lints.rustdoc]
broken_intra_doc_links = "deny"
missing_docs = "warn"
private_doc_tests = "warn"
```

#### 5.2 整合到現有 CI/CD 流程
- [ ] **擴展現有的 build-test-audit-coverage.yml 工作流程**
  - [ ] 在測試階段新增文件檢查步驟
  - [ ] 新增文件範例測試驗證
  - [ ] 整合文件完整性檢查到 clippy 步驟
  - [ ] 確保文件與程式碼同步性檢查

#### 5.3 文件維護指南
- [ ] **建立文件維護流程**
  - [ ] 程式碼審查中的文件檢查清單
  - [ ] 新功能開發的文件要求
  - [ ] 文件更新的版本控制策略
  - [ ] 社群貢獻的文件指南

## 實作標準

### 文件撰寫規範

#### 1. 模組級文件 (Module-level documentation)
```rust
//! Brief description of the module's purpose.
//! 
//! Detailed explanation of the module's functionality, architecture,
//! and key concepts. Include examples of typical usage patterns.
//! 
//! # Examples
//! 
//! ```rust
//! use subx_cli::module_name::SomeStruct;
//! 
//! let instance = SomeStruct::new();
//! instance.do_something()?;
//! ```
```

#### 2. 結構體文件
```rust
/// Brief description of the struct's purpose.
/// 
/// Detailed explanation of the struct's role, typical usage,
/// and relationship to other types in the system.
/// 
/// # Examples
/// 
/// ```rust
/// let config = Config::new();
/// config.validate()?;
/// ```
/// 
/// # Fields
/// 
/// - `field_name`: Description of the field's purpose and constraints
pub struct MyStruct {
    /// Brief description of the field.
    pub field_name: String,
}
```

#### 3. 函式文件
```rust
/// Brief description of what the function does.
/// 
/// Detailed explanation of the function's behavior, including
/// any important algorithm details or implementation notes.
/// 
/// # Arguments
/// 
/// - `param1`: Description of the parameter and its constraints
/// - `param2`: Description of the parameter and its constraints
/// 
/// # Returns
/// 
/// Description of the return value and its meaning.
/// 
/// # Errors
/// 
/// This function returns an error if:
/// - Condition 1 occurs
/// - Condition 2 occurs
/// 
/// # Examples
/// 
/// ```rust
/// let result = my_function("input", 42)?;
/// assert_eq!(result, expected_value);
/// ```
pub fn my_function(param1: &str, param2: i32) -> Result<String> {
    // Implementation
}
```

### 品質標準

#### 1. 完整性要求
- 所有 public API 必須有文件
- 所有 pub(crate) API 建議有文件
- 複雜的 private 函式應有文件

#### 2. 範例品質
- 所有範例必須能夠編譯通過
- 範例應該實用且具代表性
- 使用 `cargo test --doc` 驗證範例正確性

#### 3. 內容品質
- 使用清晰、簡潔的英文
- 避免術語行話，或提供解釋
- 包含足夠的上下文資訊
- 提供相關函式/類型的連結

### 工具和驗證

#### 1. 文件生成與本地檢查
```bash
# 產生完整文件（本地開發）
cargo doc --all-features --no-deps --open

# 檢查文件警告
cargo doc --all-features --no-deps 2>&1 | grep warning

# 測試文件範例
cargo test --doc

# 檢查私有項目文件（開發階段）
cargo doc --all-features --document-private-items
```

#### 2. CI/CD 自動化檢查
```bash
# 文件完整性檢查（CI 環境）
cargo clippy -- -W missing_docs -D warnings

# 文件連結和格式檢查
cargo doc --all-features --no-deps --document-private-items

# 文件範例測試
cargo test --doc --verbose
```

#### 3. 發佈前檢查清單
```bash
# 確保所有 public API 有文件
cargo clippy -- -W missing_docs

# 驗證文件範例正確性
cargo test --doc

# 檢查文件品質
cargo doc --all-features --no-deps
```

## 交付成果

### 1. 完整的 rustdoc 文件
- 所有 public API 具備完整的英文文件
- 包含實用的程式碼範例和使用指南
- 符合 Rust 社群標準的文件格式

### 2. 文件基礎設施
- 自動化文件品質檢查流程（整合至現有 CI/CD）
- 文件範例的持續驗證機制
- 文件維護與更新指南

### 3. 開發者體驗提升
- 新開發者能快速理解專案結構
- API 使用方式清晰明確
- 錯誤處理和最佳實踐有明確指引

## 驗收標準

### 技術標準
- [ ] 所有 public API 都有完整的 rustdoc 文件
- [ ] 所有文件範例都能編譯並通過測試
- [ ] `cargo doc` 執行無警告
- [ ] `cargo test --doc` 全部通過

### 品質標準
- [ ] 文件使用一致的英文撰寫風格
- [ ] 包含足夠的使用範例和程式碼片段
- [ ] 錯誤條件和邊界情況有明確說明
- [ ] 文件內容與實際實作保持同步

### 流程標準
- [ ] 建立文件撰寫與維護指南
- [ ] 整合到程式碼審查流程
- [ ] 擴展現有 CI/CD 流程包含文件品質檢查
- [ ] 提供社群貢獻的文件規範
- [ ] 確保專案發佈至 crates.io 時文件完整可用

## 時程規劃

- **第 1-2 天**: 建立文件標準，完成核心錯誤與配置模組
- **第 3-5 天**: CLI 與命令模組文件化
- **第 6-9 天**: 核心處理引擎與服務層文件化
- **第 10-11 天**: 進階功能與整合範例
- **第 12-13 天**: 文件品質檢查與現有 CI/CD 流程整合

總計：**13 個工作天**

這個 backlog 完成後，SubX 專案將具備專業級的程式碼文件，並透過現有的 CI/CD 流程確保文件品質，大幅提升專案的可維護性和開發者體驗。專案發佈至 crates.io 時，完整的文件將自動在 docs.rs 上提供給社群使用。
