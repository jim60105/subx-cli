use clap::{Args, Subcommand};

/// 快取管理參數
#[derive(Args, Debug)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub action: CacheAction,
}

/// 快取操作子命令
#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// 清除所有 Dry-run 快取檔案
    Clear,
}
