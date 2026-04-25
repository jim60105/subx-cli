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
pub mod sync_args;
pub mod table;
pub mod ui;

pub use cache_args::{
    ApplyArgs, CacheAction, CacheArgs, ClearArgs, ClearType, RollbackArgs, StatusArgs,
};
use clap::{Parser, Subcommand};
pub use config_args::{ConfigAction, ConfigArgs};
pub use convert_args::{ConvertArgs, OutputSubtitleFormat};
pub use detect_encoding_args::DetectEncodingArgs;
pub use generate_completion_args::GenerateCompletionArgs;
pub use input_handler::{CollectedFiles, InputPathHandler};
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

    // Switch to workspace directory for file operations if specified via env or config
    if let Some(ws_env) = std::env::var_os("SUBX_WORKSPACE") {
        std::env::set_current_dir(&ws_env).map_err(|e| {
            crate::error::SubXError::CommandExecution(format!(
                "Failed to set workspace directory to {}: {}",
                std::path::PathBuf::from(&ws_env).display(),
                e
            ))
        })?;
    } else if let Ok(config) = config_service.get_config() {
        let ws_dir = &config.general.workspace;
        if !ws_dir.as_os_str().is_empty() {
            std::env::set_current_dir(ws_dir).map_err(|e| {
                crate::error::SubXError::CommandExecution(format!(
                    "Failed to set workspace directory to {}: {}",
                    ws_dir.display(),
                    e
                ))
            })?;
        }
    }

    // Use the centralized dispatcher to avoid code duplication
    crate::commands::dispatcher::dispatch_command_with_ref(cli.command, config_service).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    // ─── Subcommand routing ──────────────────────────────────────────────────

    #[test]
    fn test_match_subcommand_routes_to_match_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "match", "."]).unwrap();
        assert!(matches!(cli.command, Commands::Match(_)));
    }

    #[test]
    fn test_convert_subcommand_routes_to_convert_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "convert", "file.srt"]).unwrap();
        assert!(matches!(cli.command, Commands::Convert(_)));
    }

    #[test]
    fn test_detect_encoding_subcommand_routes_to_detect_encoding_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "detect-encoding", "file.srt"]).unwrap();
        assert!(matches!(cli.command, Commands::DetectEncoding(_)));
    }

    #[test]
    fn test_sync_subcommand_routes_to_sync_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "video.mp4"]).unwrap();
        assert!(matches!(cli.command, Commands::Sync(_)));
    }

    #[test]
    fn test_config_subcommand_routes_to_config_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "config", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::Config(_)));
    }

    #[test]
    fn test_generate_completion_subcommand_routes_to_generate_completion_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "generate-completion", "bash"]).unwrap();
        assert!(matches!(cli.command, Commands::GenerateCompletion(_)));
    }

    #[test]
    fn test_cache_subcommand_routes_to_cache_variant() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "status"]).unwrap();
        assert!(matches!(cli.command, Commands::Cache(_)));
    }

    // ─── Help and version flags ──────────────────────────────────────────────

    #[test]
    fn test_help_flag_exits_with_error() {
        // --help causes clap to print and return an Err with kind DisplayHelp
        let err = Cli::try_parse_from(["subx-cli", "--help"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_version_flag_exits_with_error() {
        let err = Cli::try_parse_from(["subx-cli", "--version"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }

    #[test]
    fn test_subcommand_help_flag() {
        let err = Cli::try_parse_from(["subx-cli", "match", "--help"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    // ─── Invalid / missing arguments ────────────────────────────────────────

    #[test]
    fn test_no_subcommand_returns_error() {
        let result = Cli::try_parse_from(["subx-cli"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_subcommand_returns_error() {
        let result = Cli::try_parse_from(["subx-cli", "nonexistent-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_flag_returns_error() {
        let result = Cli::try_parse_from(["subx-cli", "--unknown-flag"]);
        assert!(result.is_err());
    }

    // ─── Default values propagated through Commands ──────────────────────────

    #[test]
    fn test_match_default_confidence_is_80() {
        let cli = Cli::try_parse_from(["subx-cli", "match", "."]).unwrap();
        if let Commands::Match(args) = cli.command {
            assert_eq!(args.confidence, 80);
        } else {
            panic!("Expected Match command");
        }
    }

    #[test]
    fn test_match_default_flags_are_false() {
        let cli = Cli::try_parse_from(["subx-cli", "match", "."]).unwrap();
        if let Commands::Match(args) = cli.command {
            assert!(!args.dry_run);
            assert!(!args.recursive);
            assert!(!args.backup);
            assert!(!args.copy);
            assert!(!args.move_files);
            assert!(!args.no_extract);
        } else {
            panic!("Expected Match command");
        }
    }

    #[test]
    fn test_convert_default_encoding_is_utf8() {
        let cli = Cli::try_parse_from(["subx-cli", "convert", "file.srt"]).unwrap();
        if let Commands::Convert(args) = cli.command {
            assert_eq!(args.encoding, "utf-8");
            assert!(!args.keep_original);
            assert!(!args.recursive);
        } else {
            panic!("Expected Convert command");
        }
    }

    #[test]
    fn test_cache_clear_default_type_is_all() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "clear"]).unwrap();
        if let Commands::Cache(cache_args) = cli.command {
            if let CacheAction::Clear(clear_args) = cache_args.action {
                assert_eq!(clear_args.r#type, ClearType::All);
            } else {
                panic!("Expected Clear action");
            }
        } else {
            panic!("Expected Cache command");
        }
    }

    // ─── Cache subcommand variants ───────────────────────────────────────────

    #[test]
    fn test_cache_status_parses_json_flag() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "status", "--json"]).unwrap();
        if let Commands::Cache(cache_args) = cli.command {
            if let CacheAction::Status(status_args) = cache_args.action {
                assert!(status_args.json);
            } else {
                panic!("Expected Status action");
            }
        } else {
            panic!("Expected Cache command");
        }
    }

    #[test]
    fn test_cache_apply_parses_yes_and_force() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "apply", "--yes", "--force"]).unwrap();
        if let Commands::Cache(cache_args) = cli.command {
            if let CacheAction::Apply(apply_args) = cache_args.action {
                assert!(apply_args.yes);
                assert!(apply_args.force);
            } else {
                panic!("Expected Apply action");
            }
        } else {
            panic!("Expected Cache command");
        }
    }

    #[test]
    fn test_cache_rollback_parses_force() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "rollback", "--force"]).unwrap();
        if let Commands::Cache(cache_args) = cli.command {
            if let CacheAction::Rollback(rollback_args) = cache_args.action {
                assert!(rollback_args.force);
            } else {
                panic!("Expected Rollback action");
            }
        } else {
            panic!("Expected Cache command");
        }
    }

    #[test]
    fn test_cache_clear_journal_type() {
        let cli = Cli::try_parse_from(["subx-cli", "cache", "clear", "--type", "journal"]).unwrap();
        if let Commands::Cache(cache_args) = cli.command {
            if let CacheAction::Clear(clear_args) = cache_args.action {
                assert_eq!(clear_args.r#type, ClearType::Journal);
            } else {
                panic!("Expected Clear action");
            }
        } else {
            panic!("Expected Cache command");
        }
    }

    // ─── Config subcommand variants ──────────────────────────────────────────

    #[test]
    fn test_config_set_parses_key_and_value() {
        let cli =
            Cli::try_parse_from(["subx-cli", "config", "set", "ai.provider", "openai"]).unwrap();
        if let Commands::Config(config_args) = cli.command {
            if let ConfigAction::Set { key, value } = config_args.action {
                assert_eq!(key, "ai.provider");
                assert_eq!(value, "openai");
            } else {
                panic!("Expected Set action");
            }
        } else {
            panic!("Expected Config command");
        }
    }

    #[test]
    fn test_config_get_parses_key() {
        let cli = Cli::try_parse_from(["subx-cli", "config", "get", "ai.model"]).unwrap();
        if let Commands::Config(config_args) = cli.command {
            if let ConfigAction::Get { key } = config_args.action {
                assert_eq!(key, "ai.model");
            } else {
                panic!("Expected Get action");
            }
        } else {
            panic!("Expected Config command");
        }
    }

    #[test]
    fn test_config_list_routes_to_list_action() {
        let cli = Cli::try_parse_from(["subx-cli", "config", "list"]).unwrap();
        if let Commands::Config(config_args) = cli.command {
            assert!(matches!(config_args.action, ConfigAction::List));
        } else {
            panic!("Expected Config command");
        }
    }

    #[test]
    fn test_config_reset_routes_to_reset_action() {
        let cli = Cli::try_parse_from(["subx-cli", "config", "reset"]).unwrap();
        if let Commands::Config(config_args) = cli.command {
            assert!(matches!(config_args.action, ConfigAction::Reset));
        } else {
            panic!("Expected Config command");
        }
    }

    // ─── Generate-completion subcommand ──────────────────────────────────────

    #[test]
    fn test_generate_completion_bash() {
        use clap_complete::Shell;
        let cli = Cli::try_parse_from(["subx-cli", "generate-completion", "bash"]).unwrap();
        if let Commands::GenerateCompletion(args) = cli.command {
            assert_eq!(args.shell, Shell::Bash);
        } else {
            panic!("Expected GenerateCompletion command");
        }
    }

    #[test]
    fn test_generate_completion_zsh() {
        use clap_complete::Shell;
        let cli = Cli::try_parse_from(["subx-cli", "generate-completion", "zsh"]).unwrap();
        if let Commands::GenerateCompletion(args) = cli.command {
            assert_eq!(args.shell, Shell::Zsh);
        } else {
            panic!("Expected GenerateCompletion command");
        }
    }

    #[test]
    fn test_generate_completion_missing_shell_arg_returns_error() {
        let result = Cli::try_parse_from(["subx-cli", "generate-completion"]);
        assert!(result.is_err());
    }

    // ─── Sync subcommand ─────────────────────────────────────────────────────

    #[test]
    fn test_sync_video_and_subtitle_flags() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--video",
            "video.mp4",
            "--subtitle",
            "sub.srt",
        ])
        .unwrap();
        if let Commands::Sync(args) = cli.command {
            assert_eq!(args.video, Some(PathBuf::from("video.mp4")));
            assert_eq!(args.subtitle, Some(PathBuf::from("sub.srt")));
        } else {
            panic!("Expected Sync command");
        }
    }

    #[test]
    fn test_sync_manual_offset_flag() {
        let cli = Cli::try_parse_from([
            "subx-cli", "sync", "--method", "manual", "--offset", "2.5", "sub.srt",
        ])
        .unwrap();
        if let Commands::Sync(args) = cli.command {
            assert_eq!(args.offset, Some(2.5));
            assert_eq!(args.method, Some(SyncMethodArg::Manual));
        } else {
            panic!("Expected Sync command");
        }
    }

    // ─── Detect-encoding subcommand ──────────────────────────────────────────

    #[test]
    fn test_detect_encoding_verbose_flag() {
        let cli =
            Cli::try_parse_from(["subx-cli", "detect-encoding", "--verbose", "file.srt"]).unwrap();
        if let Commands::DetectEncoding(args) = cli.command {
            assert!(args.verbose);
            assert_eq!(args.file_paths, vec!["file.srt".to_string()]);
        } else {
            panic!("Expected DetectEncoding command");
        }
    }

    #[test]
    fn test_detect_encoding_missing_file_returns_error() {
        let result = Cli::try_parse_from(["subx-cli", "detect-encoding"]);
        assert!(result.is_err());
    }

    // ─── Debug formatting ────────────────────────────────────────────────────

    #[test]
    fn test_cli_debug_format() {
        let cli = Cli::try_parse_from(["subx-cli", "match", "."]).unwrap();
        let debug_str = format!("{cli:?}");
        assert!(debug_str.contains("Cli"));
    }

    #[test]
    fn test_commands_debug_format_for_each_variant() {
        let commands = [
            Cli::try_parse_from(["subx-cli", "match", "."]),
            Cli::try_parse_from(["subx-cli", "convert", "f.srt"]),
            Cli::try_parse_from(["subx-cli", "detect-encoding", "f.srt"]),
            Cli::try_parse_from(["subx-cli", "config", "list"]),
            Cli::try_parse_from(["subx-cli", "cache", "status"]),
            Cli::try_parse_from(["subx-cli", "generate-completion", "fish"]),
        ];
        for result in &commands {
            let cli = result.as_ref().expect("parse should succeed");
            let s = format!("{:?}", cli.command);
            assert!(!s.is_empty());
        }
    }
}
