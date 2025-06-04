// src/main.rs
use anyhow::Result;
use env_logger;
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日誌
    env_logger::init();

    info!("啟動 SubX v{}", subx::VERSION);

    // CLI 執行邏輯待實作
    // if let Err(e) = subx::cli::run().await {
    //     eprintln!("錯誤: {}", e);
    //     std::process::exit(1);
    // }

    // 暫時的輸出，直到 CLI 介面實作
    println!("🎬 SubX - 智慧字幕處理工具");
    println!("版本: {}", subx::VERSION);
    println!("狀態: 基礎架構已建立 ✅");

    Ok(())
}
