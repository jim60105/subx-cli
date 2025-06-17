# 41 - lib.rs App 結構職責釐清：移除重複邏輯或明確函式庫用途

## 概述

本計劃旨在釐清 `src/lib.rs` 中 `App` 結構的用途和定位，解決其與 `src/cli/mod.rs` 中 `run()` 函式的功能重疊問題。目前 `App` 結構提供了另一種應用程式啟動方式，但與主要的 CLI 執行流程存在重複邏輯，這可能導致維護複雜性和使用者混淆。需要評估並決定是否保留 `App` 結構作為函式庫 API，或是移除重複的邏輯以簡化架構。

## 問題描述

### 當前狀況
- `main.rs` 呼叫 `cli::run()` 作為主要執行路徑
- `lib.rs` 中的 `App` 結構提供 `run()` 和 `handle_command()` 方法
- 兩者都實作了相似的命令分派邏輯
- `App::handle_command()` 與 `cli::run_with_config()` 有重複的 match 邏輯

### 重複邏輯範例
```rust
// In lib.rs App::handle_command()
match command {
    cli::Commands::Match(args) => {
        crate::commands::match_command::execute_with_config(args, self.config_service.clone()).await
    }
    // ... other commands
}

// In cli/mod.rs run_with_config()  
match cli.command {
    Commands::Match(args) => {
        crate::commands::match_command::execute_with_config(args, config_service).await
    }
    // ... same commands with same logic
}
```

### 使用混淆
- 開發者不清楚應該使用哪種方式
- 兩種初始化方式可能導致不一致的行為
- 測試需要覆蓋兩套類似的邏輯

## 技術需求

### 主要目標
1. 評估 `App` 結構的實際用途和必要性
2. 決定保留還是移除 `App` 結構
3. 如果保留，明確其作為函式庫 API 的定位
4. 如果移除，確保不影響現有功能
5. 消除重複的命令分派邏輯
6. 改善程式碼可維護性

### 設計選項
**選項 A：保留 App 作為函式庫 API**
- 明確 `App` 作為程式化 API 使用
- 重構以避免重複邏輯
- 完善文件說明用途差異

**選項 B：移除 App 結構**
- 簡化架構，只保留 CLI 路徑
- 移除重複的邏輯
- 如需程式化使用，提供其他方式

## 實作計劃

### 階段 1：評估 App 結構的使用狀況
**預估時間：1 小時**

1. **搜尋 App 結構的使用**：
   ```bash
   # Find all references to App struct
   grep -r "App::" src/ --include="*.rs"
   grep -r "App {" src/ --include="*.rs"
   grep -r "use.*App" src/ --include="*.rs"
   ```

2. **檢查測試中的使用**：
   ```bash
   # Check if App is used in tests
   grep -r "App::" tests/ --include="*.rs" || echo "No test usage found"
   find tests/ -name "*.rs" -exec grep -l "App" {} \; || echo "No App references in tests"
   ```

3. **檢查是否有外部 API 使用**：
   ```bash
   # Check public exports
   grep -r "pub.*App" src/lib.rs
   ```

4. **分析實際需求**：
   - 是否有程式化使用 SubX 的需求？
   - 是否需要在其他 Rust 程式中嵌入 SubX？
   - CLI 路徑是否足以滿足所有使用場景？

### 階段 2：決定處理方案
**預估時間：30 分鐘**

根據階段 1 的分析結果，選擇以下方案之一：

#### 方案 A：保留並完善 App 結構（如果有函式庫使用需求）

#### 方案 B：移除 App 結構（如果只是冗餘程式碼）

### 階段 3A：實作方案 A - 保留並完善 App 結構
**預估時間：3 小時**（如果選擇此方案）

1. **重構命令分派邏輯**：
   ```rust
   // Create src/commands/dispatcher.rs
   use crate::{Result, config::ConfigService, cli::Commands};
   use std::sync::Arc;

   /// Central command dispatcher to avoid code duplication.
   ///
   /// This module provides a unified way to dispatch commands,
   /// eliminating duplication between CLI and library API paths.
   pub async fn dispatch_command(
       command: Commands,
       config_service: Arc<dyn ConfigService>,
   ) -> Result<()> {
       match command {
           Commands::Match(args) => {
               crate::commands::match_command::execute_with_config(args, config_service).await
           }
           Commands::Convert(args) => {
               crate::commands::convert_command::execute_with_config(args, config_service).await
           }
           Commands::Sync(args) => {
               crate::commands::sync_command::execute_with_config(args, config_service).await
           }
           Commands::Config(args) => {
               crate::commands::config_command::execute_with_config(args, config_service).await
           }
           Commands::GenerateCompletion(args) => {
               // Handle shell completion generation
               let mut cmd = <crate::cli::Cli as clap::CommandFactory>::command();
               let cmd_name = cmd.get_name().to_string();
               let mut stdout = std::io::stdout();
               clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
               Ok(())
           }
           Commands::Cache(args) => {
               crate::commands::cache_command::execute_with_config(args, config_service).await
           }
           Commands::DetectEncoding(args) => {
               crate::commands::detect_encoding_command::detect_encoding_command_with_config(
                   args,
                   config_service.as_ref(),
               )?;
               Ok(())
           }
       }
   }
   ```

2. **更新 lib.rs 中的 App 實作**：
   ```rust
   // Update src/lib.rs
   impl App {
       /// Run the application with command-line argument parsing.
       ///
       /// This method provides a programmatic way to run SubX with CLI-style
       /// arguments, useful for embedding SubX in other Rust applications.
       ///
       /// # Examples
       ///
       /// ```rust,no_run
       /// use subx_cli::{App, config::ProductionConfigService};
       /// use std::sync::Arc;
       ///
       /// # async fn example() -> subx_cli::Result<()> {
       /// let config_service = Arc::new(ProductionConfigService::new()?);
       /// let app = App::new(config_service);
       /// 
       /// // This parses std::env::args() just like the CLI
       /// app.run().await?;
       /// # Ok(())
       /// # }
       /// ```
       ///
       /// # Errors
       ///
       /// Returns an error if command execution fails.
       pub async fn run(&self) -> Result<()> {
           let cli = <cli::Cli as clap::Parser>::parse();
           self.handle_command(cli.command).await
       }

       /// Handle a specific command with the current configuration.
       ///
       /// This method allows programmatic execution of specific SubX commands
       /// without parsing command-line arguments.
       ///
       /// # Examples
       ///
       /// ```rust,no_run
       /// use subx_cli::{App, cli::{Commands, MatchArgs}, config::TestConfigService};
       /// use std::sync::Arc;
       ///
       /// # async fn example() -> subx_cli::Result<()> {
       /// let config_service = Arc::new(TestConfigService::default());
       /// let app = App::new(config_service);
       /// 
       /// let match_args = MatchArgs {
       ///     input_path: "/path/to/files".into(),
       ///     dry_run: true,
       ///     // ... other fields
       /// };
       /// 
       /// app.handle_command(Commands::Match(match_args)).await?;
       /// # Ok(())
       /// # }
       /// ```
       ///
       /// # Arguments
       ///
       /// * `command` - The command to execute
       ///
       /// # Errors
       ///
       /// Returns an error if command execution fails.
       pub async fn handle_command(&self, command: cli::Commands) -> Result<()> {
           // Use the centralized dispatcher
           crate::commands::dispatcher::dispatch_command(command, self.config_service.clone()).await
       }

       /// Execute a specific command with custom arguments.
       ///
       /// This is a convenience method for programmatic usage without
       /// needing to construct the Commands enum manually.
       ///
       /// # Examples
       ///
       /// ```rust,no_run
       /// use subx_cli::{App, config::TestConfigService};
       /// use std::sync::Arc;
       ///
       /// # async fn example() -> subx_cli::Result<()> {
       /// let config_service = Arc::new(TestConfigService::default());
       /// let app = App::new(config_service);
       /// 
       /// // Match files programmatically
       /// app.match_files("/path/to/files", true).await?; // dry_run = true
       /// # Ok(())
       /// # }
       /// ```
       pub async fn match_files(&self, input_path: &str, dry_run: bool) -> Result<()> {
           let args = cli::MatchArgs {
               input_path: input_path.into(),
               dry_run,
               ..Default::default()
           };
           self.handle_command(cli::Commands::Match(args)).await
       }

       /// Convert subtitle files programmatically.
       pub async fn convert_files(
           &self,
           input_path: &str,
           output_format: &str,
           output_path: Option<&str>,
       ) -> Result<()> {
           let args = cli::ConvertArgs {
               input_path: input_path.into(),
               output_format: output_format.into(),
               output_path: output_path.map(Into::into),
               ..Default::default()
           };
           self.handle_command(cli::Commands::Convert(args)).await
       }

       /// Synchronize subtitle files programmatically.
       pub async fn sync_files(&self, input_path: &str, method: &str) -> Result<()> {
           let args = cli::SyncArgs {
               input_path: input_path.into(),
               method: Some(method.into()),
               ..Default::default()
           };
           self.handle_command(cli::Commands::Sync(args)).await
       }
   }
   ```

3. **更新 cli/mod.rs 使用分派器**：
   ```rust
   // Update src/cli/mod.rs
   pub async fn run_with_config(config_service: Arc<dyn ConfigService>) -> Result<()> {
       let cli = Cli::parse();
       
       // Use the centralized dispatcher
       crate::commands::dispatcher::dispatch_command(cli.command, config_service).await
   }
   ```

4. **新增完整的 App API 文件**：
   ```rust
   // Update src/lib.rs documentation
   /// Main application structure with dependency injection support.
   ///
   /// The `App` struct provides a programmatic interface to SubX functionality,
   /// designed for embedding SubX in other Rust applications or for advanced
   /// use cases requiring fine-grained control over configuration and execution.
   ///
   /// # Use Cases
   ///
   /// - **Embedding**: Use SubX as a library component in larger applications
   /// - **Testing**: Programmatic testing of SubX functionality with custom configurations
   /// - **Automation**: Scripted execution of SubX operations without shell commands
   /// - **Custom Workflows**: Building complex workflows that combine multiple SubX operations
   ///
   /// # vs CLI Interface
   ///
   /// | Feature | CLI (`subx` command) | App (Library API) |
   /// |---------|---------------------|-------------------|
   /// | Usage | Command line tool | Embedded in Rust code |
   /// | Config | Files + Environment | Programmatic injection |
   /// | Output | Terminal/stdout | Programmatic control |
   /// | Error Handling | Exit codes | Result types |
   ///
   /// # Examples
   ///
   /// ## Basic Usage
   ///
   /// ```rust,no_run
   /// use subx_cli::{App, config::ProductionConfigService};
   /// use std::sync::Arc;
   ///
   /// # async fn example() -> subx_cli::Result<()> {
   /// let config_service = Arc::new(ProductionConfigService::new()?);
   /// let app = App::new(config_service);
   /// 
   /// // Execute operations programmatically
   /// app.match_files("/movies", true).await?; // dry run
   /// app.convert_files("/subs", "srt", Some("/output")).await?;
   /// # Ok(())
   /// # }
   /// ```
   ///
   /// ## With Custom Configuration
   ///
   /// ```rust,no_run
   /// use subx_cli::{App, config::{TestConfigService, Config}};
   /// use std::sync::Arc;
   ///
   /// # async fn example() -> subx_cli::Result<()> {
   /// let mut config_service = TestConfigService::default();
   /// config_service.set_ai_settings("openai", "gpt-4", "your-api-key");
   /// 
   /// let app = App::new(Arc::new(config_service));
   /// app.match_files("/path", false).await?;
   /// # Ok(())
   /// # }
   /// ```
   pub struct App {
       config_service: std::sync::Arc<dyn config::ConfigService>,
   }
   ```

### 階段 3B：實作方案 B - 移除 App 結構
**預估時間：1.5 小時**（如果選擇此方案）

1. **評估影響範圍**：
   ```bash
   # Double-check no external usage
   grep -r "pub.*App" src/lib.rs
   ```

2. **移除 App 結構**：
   ```rust
   // Remove from src/lib.rs
   // Delete the entire App struct and its impl block
   
   // Keep only essential re-exports
   pub const VERSION: &str = env!("CARGO_PKG_VERSION");
   
   pub mod cli;
   pub mod commands;
   pub mod config;
   pub use config::Config;
   pub use config::{
       ConfigService, EnvironmentProvider, ProductionConfigService, SystemEnvironmentProvider,
       TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
   };
   pub mod core;
   pub mod error;
   pub type Result<T> = error::SubXResult<T>;
   pub mod services;
   ```

3. **更新 lib.rs 文件**：
   ```rust
   //! SubX: Intelligent Subtitle Processing Library
   //!
   //! SubX is a comprehensive Rust library for intelligent subtitle file processing,
   //! featuring AI-powered matching, format conversion, audio synchronization,
   //! and advanced encoding detection capabilities.
   //!
   //! # Usage Modes
   //!
   //! ## Command Line Tool
   //!
   //! SubX is primarily designed as a command-line tool:
   //!
   //! ```bash
   //! subx match /path/to/videos
   //! subx convert /path/to/subtitles srt
   //! subx sync /path/to/files
   //! ```
   //!
   //! ## Library Components
   //!
   //! While SubX can be used as a library, it's recommended to use specific
   //! components rather than the full application:
   //!
   //! ```rust,no_run
   //! use subx_cli::{
   //!     core::{ComponentFactory, DIContainer},
   //!     config::ProductionConfigService,
   //! };
   //! use std::sync::Arc;
   //!
   //! # async fn example() -> subx_cli::Result<()> {
   //! // Use specific components
   //! let config_service = Arc::new(ProductionConfigService::new()?);
   //! let container = DIContainer::new(config_service)?;
   //! let factory = container.component_factory();
   //! 
   //! // Create specific engines
   //! let match_engine = factory.create_match_engine()?;
   //! let file_manager = factory.create_file_manager();
   //! # Ok(())
   //! # }
   //! ```
   //!
   //! This approach provides better control and avoids the overhead of
   //! the full CLI interface.
   ```

### 階段 4：更新相關檔案
**預估時間：1 小時**

1. **更新 commands/mod.rs**（如果建立了分派器）：
   ```rust
   // Add to src/commands/mod.rs if dispatcher is created
   pub mod dispatcher;
   ```

2. **更新測試**：
   - 如果保留 App，新增程式化使用的測試
   - 如果移除 App，確保沒有遺留的測試

3. **更新範例和文件**：
   - 根據選擇的方案更新所有相關文件
   - 確保範例程式碼準確反映推薦用法

### 階段 5：測試與驗證
**預估時間：1 小時**

1. **執行完整測試**：
   ```bash
   cargo test
   cargo test --release
   ```

2. **檢查 API 一致性**：
   ```bash
   # Check that CLI interface still works
   cargo run -- --help
   cargo run -- match --help
   ```

3. **驗證程式化使用**（如果保留 App）：
   ```rust
   #[test]
   fn test_app_programmatic_usage() {
       // Test the App API works as expected
   }
   ```

4. **執行品質檢查**：
   ```bash
   cargo clippy -- -D warnings
   timeout 30 scripts/quality_check.sh
   ```

### 階段 6：文件更新
**預估時間：45 分鐘**

1. **更新 CHANGELOG.md**：
   
   **如果保留 App**：
   ```markdown
   ### Changed
   - Enhanced App struct as a dedicated library API for programmatic usage
   - Eliminated code duplication between CLI and library interfaces
   - Added convenient methods for common operations (match_files, convert_files, etc.)
   
   ### Added
   - Centralized command dispatcher to reduce code duplication
   - Comprehensive documentation for programmatic SubX usage
   ```

   **如果移除 App**：
   ```markdown
   ### Removed
   - App struct from lib.rs (was redundant with CLI interface)
   - Eliminated duplicate command dispatch logic
   
   ### Changed
   - Simplified library structure to focus on component-based usage
   - Improved documentation for using SubX components directly
   ```

2. **更新技術架構文件**：
   - 根據選擇的方案更新 `docs/tech-architecture.md`
   - 明確說明推薦的使用模式

## 驗收標準

### 功能性需求
- [ ] 消除重複的命令分派邏輯
- [ ] CLI 功能保持完全不變
- [ ] 如果保留 App，提供清晰的程式化 API
- [ ] 如果移除 App，提供替代的組件使用方式

### 非功能性需求
- [ ] 程式碼維護性提高
- [ ] 使用者不再對多種啟動方式感到混淆
- [ ] 文件清晰說明推薦用法
- [ ] 向後相容性（如果有公開使用）

### 品質保證
- [ ] 所有現有測試通過
- [ ] 新增適當的測試覆蓋
- [ ] 程式碼符合專案風格指南
- [ ] 文件完整且準確

## 風險評估

### 高風險項目
- **破壞現有使用**：如果移除 App 而有外部使用者依賴它
- **功能不一致**：如果保留 App 但實作不一致

### 中風險項目
- **測試覆蓋不足**：新的實作可能缺乏適當的測試
- **文件不清晰**：使用者可能不知道應該如何使用

### 緩解策略
- 仔細檢查是否有外部使用 App 的情況
- 提供清晰的遷移指南和範例
- 確保充分的測試覆蓋

## 後續工作

### 立即後續
- 根據使用者回饋調整 API 設計
- 考慮是否需要更多便利方法

### 長期改進
- 評估是否需要支援異步回調或事件系統
- 考慮提供更細粒度的操作控制
- 評估是否需要支援插件或擴展機制

## 實作注意事項

### API 設計原則
- 保持簡潔明確的介面
- 提供充足的錯誤訊息
- 遵循 Rust 的慣用模式

### 效能考量
- 避免不必要的重複工作
- 確保程式化使用的效能合理
- 考慮批次操作的效率

### 使用者體驗
- 提供清晰的文件和範例
- 確保錯誤訊息有幫助
- 考慮常見使用場景的便利性

這個重構將解決程式碼重複問題，並明確 SubX 的使用模式，為使用者提供更清晰的選擇。
