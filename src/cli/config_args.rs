// src/cli/config_args.rs
use clap::{Args, Subcommand};

/// 配置管理參數
#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

/// 配置子命令
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 設定配置值
    Set {
        /// 配置名稱
        key: String,
        /// 配置值
        value: String,
    },
    /// 獲取配置值
    Get {
        /// 配置名稱
        key: String,
    },
    /// 列出所有配置
    List,
    /// 重置配置
    Reset,
}
