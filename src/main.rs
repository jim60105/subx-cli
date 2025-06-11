//! SubX CLI Application Entry Point
//!
//! This module contains the main entry point for the SubX subtitle processing
//! command-line application. It initializes logging, configuration management,
//! and handles the application lifecycle.

#[tokio::main]
async fn main() {
    // Initialize logging subsystem
    env_logger::init();

    // Create and run the application with dependency injection
    let result = run_application().await;

    match result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e.user_friendly_message());
            std::process::exit(e.exit_code());
        }
    }
}

/// Main application runner with proper error handling.
///
/// This function uses the new CLI interface with dependency injection.
async fn run_application() -> subx_cli::Result<()> {
    // Use the new CLI interface
    subx_cli::cli::run().await
}
