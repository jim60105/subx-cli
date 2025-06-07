# Product Backlog #10: 命令整合與測試

## 領域範圍
命令執行邏輯整合、全功能測試、錯誤處理完善

## 完成項目

### 1. Match 命令整合
- [ ] 檔案發現與 AI 分析整合
- [ ] Dry-run 模式完整實作
- [ ] 信心度過濾機制
- [ ] 操作結果報告

### 2. Convert 命令整合
- [ ] 單檔案和批量轉換邏輯
- [ ] 格式驗證和錯誤處理
- [ ] 進度報告和統計
- [ ] 輸出品質檢查

### 3. Sync 命令整合
- [ ] 自動和手動同步模式
- [ ] 批量同步處理
- [ ] 同步結果驗證
- [ ] 錯誤恢復機制

### 4. 全域錯誤處理
- [ ] 統一錯誤訊息格式
- [ ] 用戶友善的錯誤提示
- [ ] 錯誤碼和退出狀態
- [ ] 詳細日誌記錄

### 5. 效能優化
- [ ] 記憶體使用優化
- [ ] 並行處理調優
- [ ] 快取策略實作
- [ ] 資源限制管理

### 6. 使用者體驗改善
- [ ] 互動式確認提示
- [ ] 彩色輸出和圖示
- [ ] 詳細進度顯示
- [ ] 幫助文件完善

## 技術設計

### 命令執行器基礎
```rust
// src/commands/mod.rs
pub mod match_command;
pub mod convert_command;
pub mod sync_command;
pub mod config_command;

use crate::cli::Cli;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;

pub struct CommandExecutor {
    config: crate::config::Config,
    progress_style: ProgressStyle,
}

impl CommandExecutor {
    pub fn new() -> crate::Result<Self> {
        let config = crate::config::Config::load()?;
        
        let progress_style = ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-");
        
        Ok(Self {
            config,
            progress_style,
        })
    }
    
    pub async fn execute(&self, cli: Cli) -> crate::Result<()> {
        match cli.command {
            crate::cli::Commands::Match(args) => {
                self.execute_match(args).await
            }
            crate::cli::Commands::Convert(args) => {
                self.execute_convert(args).await
            }
            crate::cli::Commands::Sync(args) => {
                self.execute_sync(args).await
            }
            crate::cli::Commands::Config(args) => {
                self.execute_config(args).await
            }
        }
    }
    
    fn create_progress_bar(&self, total: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(self.progress_style.clone());
        pb.set_message(message.to_string());
        pb
    }
    
    fn print_success(&self, message: &str) {
        println!("{} {}", "✓".green().bold(), message);
    }
    
    fn print_warning(&self, message: &str) {
        println!("{} {}", "⚠".yellow().bold(), message);
    }
    
    fn print_error(&self, message: &str) {
        eprintln!("{} {}", "✗".red().bold(), message);
    }
}
```

### Match 命令完整實作
```rust
// src/commands/match_command.rs
use crate::cli::MatchArgs;
use crate::core::matcher::{MatchEngine, MatchConfig};
use crate::services::ai::openai::OpenAIClient;

impl CommandExecutor {
    pub async fn execute_match(&self, args: MatchArgs) -> crate::Result<()> {
        // 1. 驗證輸入
        if !args.path.exists() {
            return Err(crate::SubXError::Io(
                std::io::Error::new(std::io::ErrorKind::NotFound, "指定路徑不存在")
            ));
        }
        
        // 2. 建立 AI 客戶端
        let ai_client = Box::new(OpenAIClient::new(
            self.config.ai.api_key.clone()
                .ok_or_else(|| crate::SubXError::Config("未設定 OpenAI API Key".to_string()))?,
            self.config.ai.model.clone(),
        ));
        
        // 3. 設定匹配配置
        let match_config = MatchConfig {
            confidence_threshold: args.confidence as f32 / 100.0,
            max_sample_length: self.config.ai.max_sample_length,
            enable_content_analysis: true,
            backup_enabled: args.backup || self.config.general.backup_enabled,
        };
        
        // 4. 建立匹配引擎
        let match_engine = MatchEngine::new(ai_client, match_config);
        
        println!("🔍 掃描檔案中...");
        let operations = match_engine.match_files(&args.path, args.recursive).await?;
        
        if operations.is_empty() {
            self.print_warning("未找到可匹配的檔案");
            return Ok(());
        }
        
        // 5. 顯示匹配結果
        println!("\n📋 匹配結果:");
        for (i, op) in operations.iter().enumerate() {
            println!("{}. {} -> {}", 
                i + 1,
                op.subtitle_file.name.cyan(),
                op.new_subtitle_name.green()
            );
            println!("   信心度: {:.1}% | 原因: {}", 
                op.confidence * 100.0,
                op.reasoning.join(", ")
            );
        }
        
        // 6. 預覽模式或確認執行
        if args.dry_run {
            println!("\n{} 預覽模式 - 未實際執行操作", "ℹ".blue().bold());
            return Ok(());
        }
        
        if !self.confirm_execution(&operations)? {
            println!("操作已取消");
            return Ok(());
        }
        
        // 7. 執行重命名操作
        let pb = self.create_progress_bar(operations.len() as u64, "重命名檔案");
        
        let mut success_count = 0;
        let mut error_count = 0;
        
        for operation in &operations {
            pb.inc(1);
            
            match match_engine.execute_rename(operation).await {
                Ok(_) => {
                    success_count += 1;
                    pb.set_message(format!("完成: {}", operation.new_subtitle_name));
                }
                Err(e) => {
                    error_count += 1;
                    self.print_error(&format!("重命名失敗 {}: {}", operation.subtitle_file.name, e));
                }
            }
        }
        
        pb.finish_with_message("重命名完成");
        
        // 8. 顯示執行結果
        println!("\n📊 執行結果:");
        self.print_success(&format!("成功: {} 個檔案", success_count));
        
        if error_count > 0 {
            self.print_error(&format!("失敗: {} 個檔案", error_count));
        }
        
        Ok(())
    }
    
    fn confirm_execution(&self, operations: &[crate::core::matcher::MatchOperation]) -> crate::Result<bool> {
        use dialoguer::Confirm;
        
        let total_files = operations.len();
        let high_confidence = operations.iter().filter(|op| op.confidence > 0.8).count();
        
        println!("\n📈 統計資訊:");
        println!("  總檔案數: {}", total_files);
        println!("  高信心度 (>80%): {}", high_confidence);
        
        let confirmed = Confirm::new()
            .with_prompt("確認執行重命名操作?")
            .default(false)
            .interact()?;
        
        Ok(confirmed)
    }
}
```

### Convert 命令完整實作
```rust
// src/commands/convert_command.rs
use crate::cli::ConvertArgs;
use crate::core::formats::converter::{FormatConverter, ConversionConfig};

impl CommandExecutor {
    pub async fn execute_convert(&self, args: ConvertArgs) -> crate::Result<()> {
        let config = ConversionConfig {
            preserve_styling: self.config.formats.preserve_styling,
            target_encoding: args.encoding.clone(),
            keep_original: args.keep_original,
            validate_output: true,
        };
        
        let converter = FormatConverter::new(config);
        
        if args.input.is_file() {
            self.convert_single_file(&converter, &args).await
        } else if args.input.is_dir() {
            self.convert_batch(&converter, &args).await
        } else {
            Err(crate::SubXError::Io(
                std::io::Error::new(std::io::ErrorKind::NotFound, "輸入路徑不存在")
            ))
        }
    }
    
    async fn convert_single_file(
        &self,
        converter: &FormatConverter,
        args: &ConvertArgs,
    ) -> crate::Result<()> {
        let output_path = match &args.output {
            Some(path) => path.clone(),
            None => args.input.with_extension(&args.format.to_string()),
        };
        
        println!("🔄 轉換 {} -> {}", 
            args.input.display(), 
            output_path.display()
        );
        
        let result = converter.convert_file(
            &args.input,
            &output_path,
            &args.format.to_string(),
        ).await?;
        
        if result.success {
            self.print_success(&format!(
                "轉換完成: {} 條字幕", 
                result.converted_entries
            ));
            
            if !result.warnings.is_empty() {
                for warning in &result.warnings {
                    self.print_warning(warning);
                }
            }
        } else {
            for error in &result.errors {
                self.print_error(error);
            }
            return Err(crate::SubXError::SubtitleParse("轉換失敗".to_string()));
        }
        
        Ok(())
    }
    
    async fn convert_batch(
        &self,
        converter: &FormatConverter,
        args: &ConvertArgs,
    ) -> crate::Result<()> {
        println!("🔍 掃描字幕檔案...");
        
        let results = converter.convert_batch(
            &args.input,
            &args.format.to_string(),
            true,
        ).await?;
        
        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();
        
        println!("\n📊 批量轉換結果:");
        self.print_success(&format!("成功: {}/{} 檔案", success_count, total_count));
        
        // 顯示失敗的檔案
        for result in results.iter().filter(|r| !r.success) {
            for error in &result.errors {
                self.print_error(error);
            }
        }
        
        Ok(())
    }
}
```

### 全域錯誤處理
```rust
// src/error.rs (擴展)
impl SubXError {
    pub fn exit_code(&self) -> i32 {
        match self {
            SubXError::Io(_) => 1,
            SubXError::Config(_) => 2,
            SubXError::AIService(_) => 3,
            SubXError::SubtitleParse(_) => 4,
            SubXError::AudioProcessing(_) => 5,
        }
    }
    
    pub fn user_friendly_message(&self) -> String {
        match self {
            SubXError::Io(e) => {
                format!("檔案操作錯誤: {}", e)
            }
            SubXError::Config(msg) => {
                format!("配置錯誤: {}\n提示: 使用 'subx config --help' 查看配置說明", msg)
            }
            SubXError::AIService(msg) => {
                format!("AI 服務錯誤: {}\n提示: 檢查網路連接和 API 金鑰設定", msg)
            }
            SubXError::SubtitleParse(msg) => {
                format!("字幕處理錯誤: {}\n提示: 檢查檔案格式和編碼", msg)
            }
            SubXError::AudioProcessing(msg) => {
                format!("音訊處理錯誤: {}\n提示: 確認影片檔案完整且格式支援", msg)
            }
        }
    }
}

// 主程式錯誤處理
// src/main.rs (更新)
#[tokio::main]
async fn main() {
    env_logger::init();
    
    let result = run().await;
    
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_friendly_message());
            std::process::exit(e.exit_code());
        }
    }
}
```

### 整合測試
```rust
// tests/integration_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_full_workflow() {
        let temp_dir = TempDir::new().unwrap();
        
        // 建立測試檔案
        create_test_media_files(&temp_dir).await;
        
        // 測試 match 命令
        let match_result = test_match_command(&temp_dir).await;
        assert!(match_result.is_ok());
        
        // 測試 convert 命令
        let convert_result = test_convert_command(&temp_dir).await;
        assert!(convert_result.is_ok());
        
        // 測試 sync 命令
        let sync_result = test_sync_command(&temp_dir).await;
        assert!(sync_result.is_ok());
    }
    
    async fn create_test_media_files(temp_dir: &TempDir) {
        // 建立測試用的影片和字幕檔案
        // ...
    }
}
```

## 驗收標準
1. 所有命令功能完整運作
2. 錯誤處理用戶友善
3. 進度顯示清楚直觀
4. 整合測試通過
5. 效能符合要求

## 估計工時
4-5 天

## 相依性
- 依賴所有前面的 Backlog (01-08)

## 風險評估
- 中風險：整合複雜度較高
- 注意事項：模組間介面一致性、錯誤處理完整性
