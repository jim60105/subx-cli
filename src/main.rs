// src/main.rs
#[tokio::main]
async fn main() {
    // 初始化日誌
    env_logger::init();

    // 初始化配置管理器
    if let Err(e) = subx_cli::config::init_config_manager() {
        eprintln!("配置初始化失敗: {}", e.user_friendly_message());
        std::process::exit(1);
    }

    let result = subx_cli::cli::run().await;
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_friendly_message());
            std::process::exit(e.exit_code());
        }
    }
}
