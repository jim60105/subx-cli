// src/main.rs
use anyhow::Result;
use env_logger;
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    env_logger::init();

    info!("啟動 SubX v{}", subx::VERSION);

    // 執行 CLI 主邏輯
    if let Err(e) = subx::cli::run().await {
        eprintln!("錯誤: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
