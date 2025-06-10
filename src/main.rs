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
/// This function demonstrates the new dependency injection approach
/// while maintaining backward compatibility.
async fn run_application() -> subx_cli::Result<()> {
    // Option 1: Use new dependency injection approach
    match subx_cli::App::new_with_production_config() {
        Ok(app) => app.run().await,
        Err(_) => {
            // Option 2: Fall back to legacy configuration system if needed
            eprintln!("Warning: Falling back to legacy configuration system");
            subx_cli::run_with_legacy_config().await
        }
    }
}
