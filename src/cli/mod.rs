//! SubX CLI 模組

mod match_args;
mod convert_args;
mod sync_args;
mod config_args;
mod generate_completion_args;
mod ui;

use clap::{Parser, Subcommand};
pub use match_args::MatchArgs;
pub use convert_args::{ConvertArgs, OutputSubtitleFormat};
pub use sync_args::{SyncArgs, SyncMethod};
pub use config_args::{ConfigArgs, ConfigAction};
pub use generate_completion_args::GenerateCompletionArgs;
pub use ui::{create_progress_bar, print_error, print_success, print_warning};

/// SubX CLI 主體
#[derive(Parser, Debug)]
#[command(name = "subx")]
#[command(about = "智慧字幕處理 CLI 工具")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// 子命令選項
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// AI 匹配重命名字幕檔案
    Match(MatchArgs),
    /// 轉換字幕格式
    Convert(ConvertArgs),
    /// 時間軸同步校正
    Sync(SyncArgs),
    /// 配置管理
    Config(ConfigArgs),
    /// 產生 shell completion script
    GenerateCompletion(GenerateCompletionArgs),
}

/// 執行 CLI
pub async fn run() -> crate::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Match(args) => {
            println!("執行 Match 命令: {:?}", args);
        }
        Commands::Convert(args) => {
            println!("執行 Convert 命令: {:?}", args);
        }
        Commands::Sync(args) => {
            println!("執行 Sync 命令: {:?}", args);
        }
        Commands::Config(args) => {
            println!("執行 Config 命令: {:?}", args);
        }
        Commands::GenerateCompletion(args) => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
        }
    }
    Ok(())
}
