//! SubX CLI 模組

mod cache_args;
mod config_args;
mod convert_args;
mod generate_completion_args;
mod match_args;
mod sync_args;
mod ui;

pub use cache_args::{CacheAction, CacheArgs};
use clap::{Parser, Subcommand};
pub use config_args::{ConfigAction, ConfigArgs};
pub use convert_args::{ConvertArgs, OutputSubtitleFormat};
pub use generate_completion_args::GenerateCompletionArgs;
pub use match_args::MatchArgs;
pub use sync_args::{SyncArgs, SyncMethod};
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
    /// Dry-run 快取管理
    Cache(CacheArgs),
}

/// 執行 CLI
pub async fn run() -> crate::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Match(args) => {
            crate::commands::match_command::execute(args).await?;
        }
        Commands::Convert(args) => {
            crate::commands::convert_command::execute(args).await?;
        }
        Commands::Sync(args) => {
            crate::commands::sync_command::execute(args).await?;
        }
        Commands::Config(args) => {
            crate::commands::config_command::execute(args).await?;
        }
        Commands::GenerateCompletion(args) => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
        }
        Commands::Cache(args) => {
            crate::commands::cache_command::execute(args).await?;
        }
    }
    Ok(())
}
