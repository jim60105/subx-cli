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
pub use sync_args::{SyncArgs, SyncMethod, SyncMethodArg};
pub use ui::{
    create_progress_bar, display_ai_usage, display_match_results, print_error, print_success,
    print_warning,
};

/// Main CLI application structure defining the top-level interface.
///
/// This structure encapsulates the entire command-line interface for SubX,
/// providing access to all available subcommands and global options.
///
/// # Subcommands
///
/// - `match` - AI-powered subtitle file matching and intelligent renaming
/// - `convert` - Format conversion between different subtitle standards
/// - `sync` - Audio-subtitle synchronization with timing adjustment
/// - `detect-encoding` - Character encoding detection and conversion
/// - `config` - Configuration management and inspection utilities
/// - `generate-completion` - Shell completion script generation
/// - `cache` - Cache inspection and dry-run management
///
/// # Examples
///
/// ```rust,no_run
/// use subx_cli::cli::Cli;
/// use clap::Parser;
///
/// // Parse CLI arguments from specific args instead of std::env
/// let cli = Cli::parse_from(&["subx", "config", "show"]);
///
/// // Access the selected subcommand
/// match cli.command {
///     // Handle different commands...
///     _ => {}
/// }
/// ```
///
/// # Global Options
///
/// Currently, global options are handled within individual subcommands.
/// Future versions may include global flags such as verbosity control
/// or configuration file overrides.
#[derive(Parser, Debug)]
#[command(name = "subx-cli")]
#[command(about = "Intelligent subtitle processing CLI tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// The subcommand to execute.
    ///
    /// Each subcommand provides specialized functionality for different
    /// aspects of subtitle processing and management.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for the SubX CLI application.
///
/// Each variant represents a specific operation that can be performed
/// by the SubX tool, with associated argument structures that define
/// the parameters and options for that operation.
///
/// # Command Categories
///
/// ## Core Processing Commands
/// - `Match` - AI-powered subtitle matching and renaming
/// - `Convert` - Format conversion between subtitle standards
/// - `Sync` - Audio-subtitle synchronization and timing adjustment
///
/// ## Utility Commands  
/// - `DetectEncoding` - Character encoding detection and conversion
/// - `Config` - Configuration management and inspection
/// - `Cache` - Cache management and dry-run inspection
/// - `GenerateCompletion` - Shell completion script generation
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::{Commands, MatchArgs};
/// use std::path::PathBuf;
///
/// let match_command = Commands::Match(MatchArgs {
///     path: PathBuf::from("videos/"),
///     dry_run: true,
///     confidence: 80,
///     recursive: false,
///     backup: false,
///     copy: false,
///     move_files: false,
/// });
/// ```
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// AI-powered subtitle file matching and intelligent renaming.
    ///
    /// Uses artificial intelligence to analyze video and subtitle files,
    /// matching them based on content similarity and automatically
    /// renaming subtitle files to match their corresponding videos.
    Match(MatchArgs),

    /// Convert subtitle files between different formats.
    ///
    /// Supports conversion between popular subtitle formats including
    /// SRT, ASS, VTT, and others with format-specific options.
    Convert(ConvertArgs),

    /// Detect and convert character encoding of subtitle files.
    ///
    /// Automatically detects the character encoding of subtitle files
    /// and optionally converts them to UTF-8 for better compatibility.
    DetectEncoding(DetectEncodingArgs),

    /// Synchronize subtitle timing with audio tracks.
    ///
    /// Analyzes audio content and adjusts subtitle timing to match
    /// dialogue and speech patterns in the audio track.
    Sync(SyncArgs),

    /// Manage and inspect application configuration.
    ///
    /// Provides tools for viewing, validating, and managing SubX
    /// configuration settings from various sources.
    Config(ConfigArgs),

    /// Generate shell completion scripts.
    ///
    /// Creates completion scripts for various shells (bash, zsh, fish)
    /// to enable tab completion for SubX commands and arguments.
    GenerateCompletion(GenerateCompletionArgs),

    /// Manage cache and inspect dry-run results.
    ///
    /// Provides utilities for examining cached results, clearing cache
    /// data, and inspecting the effects of dry-run operations.
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
            let paths = args.get_file_paths()?;
            let string_paths: Vec<String> = paths
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            crate::commands::detect_encoding_command::detect_encoding_command(
                &string_paths,
                args.verbose,
            )?;
        }
    }
    Ok(())
}
