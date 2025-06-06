use crate::cli::CacheArgs;
use crate::error::SubXError;
use crate::Result;
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
