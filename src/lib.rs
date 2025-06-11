//! SubX: Intelligent Subtitle Processing Library
//!
//! SubX is a comprehensive Rust library for intelligent subtitle file processing,
//! featuring AI-powered matching, format conversion, audio synchronization,
//! and advanced encoding detection capabilities.
//!
//! # Key Features
//!
//! - **AI-Powered Matching**: Intelligent subtitle file matching and renaming
//! - **Format Conversion**: Support for multiple subtitle formats (SRT, ASS, VTT, etc.)
//! - **Audio Synchronization**: Advanced audio-subtitle timing adjustment
//! - **Encoding Detection**: Automatic character encoding detection and conversion
//! - **Parallel Processing**: High-performance batch operations
//! - **Configuration Management**: Flexible multi-source configuration system
//!
//! # Architecture Overview
//!
//! The library is organized into several key modules:
//!
//! - [`cli`] - Command-line interface and argument parsing
//! - [`commands`] - Implementation of all SubX commands
//! - [`config`] - Configuration management and validation
//! - [`core`] - Core processing engines (formats, matching, sync)
//! - [`error`] - Comprehensive error handling system
//! - [`services`] - External service integrations (AI, audio processing)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use subx_cli::config::{TestConfigService, ConfigService};
//!
//! // Create a configuration service
//! let config_service = TestConfigService::with_defaults();
//! let config = config_service.config();
//!
//! // Use the configuration for processing...
//! ```
//!
//! # Error Handling
//!
//! All operations return a [`Result<T>`] type that wraps [`error::SubXError`]:
//!
//! ```rust
//! use subx_cli::{Result, error::SubXError};
//!
//! fn example_operation() -> Result<String> {
//!     // This could fail with various error types
//!     Err(SubXError::config("Missing configuration"))
//! }
//! ```
//!
//! # Configuration
//!
//! SubX supports dependency injection-based configuration:
//!
//! ```rust,no_run
//! use subx_cli::config::{TestConfigService, Config};
//!
//! // Create configuration service with AI settings
//! let config_service = TestConfigService::with_ai_settings("openai", "gpt-4");
//! let config = config_service.config();
//!
//! // Access configuration values
//! println!("AI Provider: {}", config.ai.provider);
//! println!("AI Model: {}", config.ai.model);
//! ```
//!
//! # Performance Considerations
//!
//! - Use [`core::parallel`] for batch operations on large file sets
//! - Configure appropriate cache settings for repeated operations
//! - Consider memory usage when processing large audio files
//!
//! # Thread Safety
//!
//! The library is designed to be thread-safe where appropriate:
//! - Configuration manager uses `Arc<RwLock<T>>` for shared state
//! - File operations include rollback capabilities for atomicity
//! - Parallel processing uses safe concurrency patterns
//!
//! # Feature Flags
//!
//! SubX supports several optional features:
//! ```text
//! - ai - AI service integrations (default)
//! - audio - Audio processing capabilities (default)  
//! - parallel - Parallel processing support (default)
//! ```
#![allow(
    clippy::new_without_default,
    clippy::manual_clamp,
    clippy::useless_vec,
    clippy::items_after_test_module,
    clippy::needless_borrow
)]

/// Library version string.
///
/// This constant provides the current version of the SubX library,
/// automatically populated from `Cargo.toml` at compile time.
///
/// # Examples
///
/// ```rust
/// use subx_cli::VERSION;
///
/// println!("SubX version: {}", VERSION);
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cli;
pub mod commands;
pub mod config;
pub use config::Config;
// Re-export new configuration service system
pub use config::{ConfigService, ProductionConfigService, TestConfigBuilder, TestConfigService};
pub mod core;
pub mod error;
/// Convenient type alias for `Result<T, SubXError>`.
///
/// This type alias simplifies error handling throughout the SubX library
/// by providing a default error type for all fallible operations.
pub type Result<T> = error::SubXResult<T>;

pub mod services;

/// Main application structure with dependency injection support.
///
/// This struct provides the new dependency injection-based architecture
/// for the SubX application, allowing for better testability and
/// configuration management.
pub struct App {
    config_service: std::sync::Arc<dyn config::ConfigService>,
}

impl App {
    /// Create a new application instance with the provided configuration service.
    ///
    /// # Arguments
    ///
    /// * `config_service` - The configuration service to use
    pub fn new(config_service: std::sync::Arc<dyn config::ConfigService>) -> Self {
        Self { config_service }
    }

    /// Create a new application instance with the production configuration service.
    ///
    /// This is the default way to create an application instance for production use.
    ///
    /// # Errors
    ///
    /// Returns an error if the production configuration service cannot be created.
    pub fn new_with_production_config() -> Result<Self> {
        let config_service = std::sync::Arc::new(config::ProductionConfigService::new()?);
        Ok(Self::new(config_service))
    }

    /// Run the application with command-line argument parsing.
    ///
    /// This method parses command-line arguments and executes the appropriate command
    /// with the configured dependencies.
    ///
    /// # Errors
    ///
    /// Returns an error if command execution fails.
    pub async fn run(&self) -> Result<()> {
        let cli = <cli::Cli as clap::Parser>::parse();
        self.handle_command(cli.command).await
    }

    /// Handle a specific command with the current configuration.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to execute
    ///
    /// # Errors
    ///
    /// Returns an error if command execution fails.
    pub async fn handle_command(&self, command: cli::Commands) -> Result<()> {
        match command {
            cli::Commands::Match(args) => {
                crate::commands::match_command::execute_with_config(
                    args,
                    self.config_service.clone(),
                )
                .await
            }
            cli::Commands::Convert(args) => {
                crate::commands::convert_command::execute_with_config(
                    args,
                    self.config_service.clone(),
                )
                .await
            }
            cli::Commands::Sync(args) => {
                crate::commands::sync_command::execute_with_config(
                    args,
                    self.config_service.clone(),
                )
                .await
            }
            cli::Commands::Config(args) => {
                crate::commands::config_command::execute_with_config(
                    args,
                    self.config_service.clone(),
                )
                .await
            }
            cli::Commands::GenerateCompletion(args) => {
                let mut cmd = <cli::Cli as clap::CommandFactory>::command();
                let cmd_name = cmd.get_name().to_string();
                let mut stdout = std::io::stdout();
                clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
                Ok(())
            }
            cli::Commands::Cache(args) => {
                crate::commands::cache_command::execute_with_config(
                    args,
                    self.config_service.clone(),
                )
                .await
            }
            cli::Commands::DetectEncoding(args) => {
                crate::commands::detect_encoding_command::detect_encoding_command_with_config(
                    &args.file_paths,
                    args.verbose,
                    self.config_service.clone(),
                )?;
                Ok(())
            }
        }
    }

    /// Get a reference to the configuration service.
    ///
    /// This allows access to the configuration service for testing or
    /// advanced use cases.
    pub fn config_service(&self) -> &std::sync::Arc<dyn config::ConfigService> {
        &self.config_service
    }

    /// Get the current configuration.
    ///
    /// This is a convenience method that retrieves the configuration
    /// from the configured service.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn get_config(&self) -> Result<config::Config> {
        self.config_service.get_config()
    }
}

/// Backward compatibility function that has been replaced by the new CLI interface.
///
/// This function has been deprecated. Use `cli::run()` instead.
#[deprecated(since = "0.2.0", note = "Use cli::run() instead")]
pub async fn run_with_legacy_config() -> Result<()> {
    // Use the new CLI interface instead
    cli::run().await
}
