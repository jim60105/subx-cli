//! SubX CLI Application Entry Point
//!
//! This module contains the main entry point for the SubX subtitle processing
//! command-line application. It initializes logging, configuration management,
//! and handles the application lifecycle.

#[tokio::main]
async fn main() {
    // Initialize logging subsystem
    env_logger::init();

    // Initialize configuration manager
    if let Err(e) = subx_cli::config::init_config_manager() {
        eprintln!(
            "Configuration initialization failed: {}",
            e.user_friendly_message()
        );
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
