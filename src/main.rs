// src/main.rs
#[tokio::main]
async fn main() {
    // 初始化日誌
    env_logger::init();

    let result = subx_cli::cli::run().await;
    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_friendly_message());
            std::process::exit(e.exit_code());
        }
    }
}
