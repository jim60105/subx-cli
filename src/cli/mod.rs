//! Command-line interface for the SubX subtitle processing tool.
//!
//! This module provides the top-level CLI application structure and subcommands
//! for AI-powered matching, subtitle format conversion, audio synchronization,
//! encoding detection, configuration management, cache operations, and shell
//! completion generation.
//!
//! # Architecture
//!
//! The CLI is built using `clap` and follows a subcommand pattern:
//! - `match` - AI-powered subtitle file matching and renaming
//! - `convert` - Subtitle format conversion between standards
//! - `sync` - Audio-subtitle synchronization and timing adjustment
//! - `detect-encoding` - Character encoding detection and conversion
//! - `config` - Configuration management and inspection
//! - `cache` - Cache inspection and dry-run management
//! - `generate-completion` - Shell completion script generation
//!
//! # Examples
//!
//! ```bash
//! # Basic subtitle matching
//! subx match /path/to/videos /path/to/subtitles
//!
//! # Convert SRT to ASS format
//! subx convert --input file.srt --output file.ass --format ass
//!
//! # Detect file encoding
//! subx detect-encoding *.srt
//! ```

mod cache_args;
mod config_args;
mod convert_args;
mod detect_encoding_args;
mod generate_completion_args;
mod input_handler;
mod match_args;
mod sync_args;
pub mod table;
pub mod ui;

pub use cache_args::{CacheAction, CacheArgs};
use clap::{Parser, Subcommand};
pub use config_args::{ConfigAction, ConfigArgs};
pub use convert_args::{ConvertArgs, OutputSubtitleFormat};
pub use detect_encoding_args::DetectEncodingArgs;
pub use generate_completion_args::GenerateCompletionArgs;
pub use input_handler::InputPathHandler;
pub use match_args::MatchArgs;
pub use sync_args::{SyncArgs, SyncMethod, SyncMethodArg, SyncMode};
pub use ui::{
    create_progress_bar, display_ai_usage, display_match_results, print_error, print_success,
    print_warning,
};

/// Main CLI application structure defining the top-level interface.
#[derive(Parser, Debug)]
#[command(name = "subx-cli")]
#[command(about = "Intelligent subtitle processing CLI tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for the SubX CLI application.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// AI-powered subtitle file matching and intelligent renaming
    Match(MatchArgs),

    /// Convert subtitle files between different formats
    Convert(ConvertArgs),

    /// Detect and convert character encoding of subtitle files
    DetectEncoding(DetectEncodingArgs),

    /// Synchronize subtitle timing with audio tracks
    Sync(SyncArgs),

    /// Manage and inspect application configuration
    Config(ConfigArgs),

    /// Generate shell completion scripts
    GenerateCompletion(GenerateCompletionArgs),

    /// Manage cache and inspect dry-run results
    Cache(CacheArgs),
}

/// Executes the SubX CLI application with parsed arguments.
///
/// This is the main entry point for CLI execution, routing parsed
/// command-line arguments to their respective command handlers.
///
/// # Arguments Processing
///
/// The function takes ownership of parsed CLI arguments and dispatches
/// them to the appropriate command implementation based on the selected
/// subcommand.
///
/// # Error Handling
///
/// Returns a [`crate::Result<()>`] that wraps any errors encountered
/// during command execution. Errors are propagated up to the main
/// function for proper exit code handling.
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::run;
///
/// # tokio_test::block_on(async {
/// // This would typically be called from main()
/// // run().await?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # });
/// ```
///
/// # Async Context
///
/// This function is async because several subcommands perform I/O
/// operations that benefit from async execution, particularly:
/// - AI service API calls
/// - Large file processing operations
/// - Network-based configuration loading
pub async fn run() -> crate::Result<()> {
    // Create production configuration service
    let config_service = std::sync::Arc::new(crate::config::ProductionConfigService::new()?);
    run_with_config(config_service.as_ref()).await
}

/// Run the CLI with a provided configuration service.
///
/// This function enables dependency injection of configuration services,
/// making it easier to test and providing better control over configuration
/// management.
///
/// # Arguments
///
/// * `config_service` - The configuration service to use
///
/// # Errors
///
/// Returns an error if command execution fails.
pub async fn run_with_config(
    config_service: &dyn crate::config::ConfigService,
) -> crate::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Match(args) => {
            args.validate()
                .map_err(crate::error::SubXError::CommandExecution)?;
            crate::commands::match_command::execute(args, config_service).await?;
        }
        Commands::Convert(args) => {
            crate::commands::convert_command::execute(args, config_service).await?;
        }
        Commands::Sync(args) => {
            crate::commands::sync_command::execute(args, config_service).await?;
        }
        Commands::Config(args) => {
            crate::commands::config_command::execute(args, config_service).await?;
        }
        Commands::GenerateCompletion(args) => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let cmd_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            clap_complete::generate(args.shell, &mut cmd, cmd_name, &mut stdout);
        }
        Commands::Cache(args) => {
            crate::commands::cache_command::execute(args).await?;
        }
        Commands::DetectEncoding(args) => {
            crate::commands::detect_encoding_command::detect_encoding_command_with_config(
                args,
                config_service,
            )?;
        }
    }
    Ok(())
}
