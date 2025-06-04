// src/cli/generate_completion_args.rs
use clap::Args;
use clap_complete::Shell;

/// 產生 shell completion script 參數
#[derive(Args, Debug)]
pub struct GenerateCompletionArgs {
    /// 要產生 completion script 的 shell 類型
    #[arg(value_enum)]
    pub shell: Shell,
}
