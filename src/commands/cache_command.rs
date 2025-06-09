//! Cache management command implementation.
//!
//! This module implements the `cache` subcommand for inspecting and clearing
//! the subtitle matching cache persisted on disk.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::CacheArgs;
//! use subx_cli::commands::cache_command;
//!
//! async fn demo(args: CacheArgs) -> subx_cli::Result<()> {
//!     cache_command::execute(args).await
//! }
//! ```
//!
use crate::Result;
use crate::cli::CacheArgs;
use crate::error::SubXError;
use dirs;

/// 執行 Cache 命令
pub async fn execute(args: CacheArgs) -> Result<()> {
    match args.action {
        crate::cli::CacheAction::Clear => {
            let dir = dirs::config_dir().ok_or_else(|| SubXError::config("無法確定快取目錄"))?;
            let path = dir.join("subx").join("match_cache.json");
            if path.exists() {
                std::fs::remove_file(&path)?;
                println!("已清除快取檔案：{}", path.display());
            } else {
                println!("未發現快取檔案");
            }
        }
    }
    Ok(())
}
