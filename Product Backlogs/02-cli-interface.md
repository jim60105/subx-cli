# Product Backlog #02: CLI 介面框架

## 領域範圍
命令行介面設計、參數解析、基本命令路由

## 完成項目

### 1. CLI 命令結構設計
- [ ] 實作主要命令 enum: `Match`, `Convert`, `Sync`, `Config`
- [ ] 設計各命令的參數結構
- [ ] 實作 `clap` derive macros
- [ ] 建立命令執行路由

### 2. Match 命令參數
- [ ] `--dry-run`: 預覽模式參數
- [ ] `--confidence <NUM>`: 信心度閾值 (0-100)
- [ ] `--recursive`: 遞歸處理子資料夾
- [ ] `--backup`: 重命名前備份原文件
- [ ] 目標路徑位置參數

### 3. Convert 命令參數
- [ ] `--format <FORMAT>`: 目標格式 (srt|ass|vtt|sub)
- [ ] `--output, -o <FILE>`: 輸出文件名
- [ ] `--keep-original`: 保留原始文件
- [ ] `--encoding <ENC>`: 指定文字編碼
- [ ] 輸入文件路徑參數

### 4. Sync 命令參數
- [ ] `--offset <SECONDS>`: 手動指定偏移量
- [ ] `--batch`: 批量處理模式
- [ ] `--range <SECONDS>`: 偏移檢測範圍
- [ ] `--method <METHOD>`: 同步方法 (audio|manual)
- [ ] 影片和字幕文件路徑參數

### 5. Config 命令參數
- [ ] `set <KEY> <VALUE>`: 設定配置值
- [ ] `get <KEY>`: 獲取配置值
- [ ] `list`: 列出所有配置
- [ ] `reset`: 重置配置

### 6. 用戶介面增強
- [ ] 整合 `colored` 套件用於彩色輸出
- [ ] 整合 `indicatif` 套件用於進度條
- [ ] 整合 `dialoguer` 套件用於互動式提示
- [ ] 實作使用說明和範例

## 技術設計

### CLI 結構定義
```rust
// src/cli/mod.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "subx")]
#[command(about = "智慧字幕處理 CLI 工具")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// AI 匹配重命名字幕文件
    Match(MatchArgs),
    /// 轉換字幕格式
    Convert(ConvertArgs),
    /// 時間軸同步校正
    Sync(SyncArgs),
    /// 配置管理
    Config(ConfigArgs),
    /// 產生 shell completion script (新增)
    GenerateCompletion(GenerateCompletionArgs),
}
```

### Match 命令參數
```rust
// src/cli/match_args.rs
#[derive(Args)]
pub struct MatchArgs {
    /// 目標資料夾路徑
    pub path: PathBuf,
    
    /// 預覽模式，不實際執行操作
    #[arg(long)]
    pub dry_run: bool,
    
    /// 最低信心度閾值 (0-100)
    #[arg(long, default_value = "80")]
    pub confidence: u8,
    
    /// 遞歸處理子資料夾
    #[arg(short, long)]
    pub recursive: bool,
    
    /// 重命名前備份原文件
    #[arg(long)]
    pub backup: bool,
}
```

### Convert 命令參數
```rust
// src/cli/convert_args.rs
#[derive(Args)]
pub struct ConvertArgs {
    /// 輸入文件或資料夾路徑
    pub input: PathBuf,
    
    /// 目標格式
    #[arg(long, value_enum)]
    pub format: OutputSubtitleFormat, // 更新 enum 名稱以更清晰
    
    /// 輸出文件路徑
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// 保留原始文件
    #[arg(long)]
    pub keep_original: bool,
    
    /// 文字編碼
    #[arg(long, default_value = "utf-8")]
    pub encoding: String,
}

#[derive(ValueEnum, Clone)]
pub enum OutputSubtitleFormat { // 更新 enum 名稱
    Srt,
    Ass,
    Vtt,
    Sub,
}
```

### 新增 GenerateCompletionArgs
```rust
// src/cli/generate_completion_args.rs
#[derive(Args)]
pub struct GenerateCompletionArgs {
    /// The shell to generate the completion script for
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}
```

### 命令執行路由
```rust
// src/cli/mod.rs
pub async fn run() -> crate::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Match(args) => {
            // 實際執行邏輯將在 Backlog #09 中整合
            println!("執行 Match 命令: {:?}", args);
            crate::commands::match_command::execute(args).await
        }
        Commands::Convert(args) => {
            // 實際執行邏輯將在 Backlog #09 中整合
            println!("執行 Convert 命令: {:?}", args);
            crate::commands::convert_command::execute(args).await
        }
        Commands::Sync(args) => {
            // 實際執行邏輯將在 Backlog #09 中整合
            println!("執行 Sync 命令: {:?}", args);
            crate::commands::sync_command::execute(args).await
        }
        Commands::Config(args) => {
            // 實際執行邏輯將在 Backlog #03 中實作
            println!("執行 Config 命令: {:?}", args);
            crate::commands::config_command::execute(args).await
        }
        // 新增 GenerateCompletion 命令的處理
        Commands::GenerateCompletion(args) => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut std::io::stdout());
            Ok(())
        }
    }
}
```

### 用戶介面輔助函式
```rust
// src/cli/ui.rs
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
    );
    pb
}
```

## 驗收標準
1. 所有命令和參數正確解析
2. 幫助資訊完整且易懂
3. 錯誤命令有適當的錯誤提示
4. 彩色輸出和進度條正常顯示
5. `clap` 自動生成的幫助符合設計規範

## 估計工時
3-4 天

## 相依性
- 依賴 Backlog #01 (專案基礎建設)

## 風險評估
- 低風險：成熟的 CLI 框架
- 注意事項：確保參數設計符合用戶習慣
