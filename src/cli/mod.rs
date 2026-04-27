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
pub mod output;
pub mod sync_args;
pub mod table;
mod translate_args;
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
pub use output::{OutputMode, SCHEMA_VERSION};
pub use sync_args::{SyncArgs, SyncMethod, SyncMethodArg, SyncMode};
pub use translate_args::TranslateArgs;
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
    /// Output mode for the entire invocation.
    ///
    /// Defaults to `text` (the existing human-friendly UI). Set to
    /// `json` to receive a versioned, machine-readable envelope on
    /// stdout. The flag is intentionally NOT `global(true)` so that it
    /// must precede the subcommand token; this avoids colliding with
    /// the per-subcommand `--output <PATH>` arguments on `convert`,
    /// `sync`, and `translate`.
    #[arg(long, value_enum, value_name = "MODE", global = false)]
    pub output: Option<OutputMode>,

    /// Suppress non-fatal status chatter.
    ///
    /// In text mode this silences `print_success`/`print_warning` and
    /// progress bars. In JSON mode, free-form `eprintln!` / `println!`
    /// chatter (matcher analysis blocks, conflict-resolution warnings,
    /// AI candidate listings) is already suppressed unconditionally;
    /// `--quiet` additionally silences the structured `tracing` / `log`
    /// records that JSON mode would otherwise still allow on stderr.
    /// Like `--output`, this flag must precede the subcommand token.
    #[arg(long, global = false)]
    pub quiet: bool,

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

    /// Translate subtitle cue text into a target language using the
    /// configured AI provider.
    Translate(TranslateArgs),
}

/// Outcome of a CLI invocation, surfaced to `main.rs` so it can render
/// the final envelope without re-parsing argv.
///
/// The active [`OutputMode`] is resolved from `--output`, the
/// `SUBX_OUTPUT` environment variable, and the built-in default in that
/// order; `command` is the kebab-cased subcommand name (`"match"`,
/// `"sync"`, …); `result` carries any [`crate::error::SubXError`]
/// produced during dispatch.
#[derive(Debug)]
pub struct RunOutcome {
    /// Active output mode for the invocation.
    pub output_mode: OutputMode,
    /// `--quiet` was set on the command line.
    pub quiet: bool,
    /// Stable subcommand identifier used as `envelope.command`.
    pub command: &'static str,
    /// Result of the dispatched subcommand.
    pub result: crate::Result<()>,
}

/// Resolve the active output mode from a parsed [`Cli`] plus the
/// `SUBX_OUTPUT` environment variable.
///
/// `--output` always wins over the environment fallback.
pub fn resolve_output_mode(cli_flag: Option<OutputMode>) -> OutputMode {
    if let Some(mode) = cli_flag {
        return mode;
    }
    if let Ok(value) = std::env::var("SUBX_OUTPUT") {
        if let Some(mode) = OutputMode::from_token(&value) {
            return mode;
        }
    }
    OutputMode::Text
}

/// Return the stable kebab-cased command name for a parsed subcommand.
pub fn command_name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Match(_) => "match",
        Commands::Convert(_) => "convert",
        Commands::DetectEncoding(_) => "detect-encoding",
        Commands::Sync(_) => "sync",
        Commands::Config(_) => "config",
        Commands::GenerateCompletion(_) => "generate-completion",
        Commands::Cache(_) => "cache",
        Commands::Translate(_) => "translate",
    }
}

/// Executes the SubX CLI application with the production configuration.
///
/// Backward-compatible shim returning `crate::Result<()>`. Prefer
/// [`run_with_config`] (which returns a [`RunOutcome`]) for new
/// integrations that need the resolved [`OutputMode`].
pub async fn run() -> crate::Result<()> {
    let config_service = std::sync::Arc::new(crate::config::ProductionConfigService::new()?);
    run_with_config(config_service.as_ref()).await.result
}

/// Run the CLI with a provided configuration service.
///
/// Returns a structured [`RunOutcome`] so the caller (typically
/// `main.rs`) can render the final JSON envelope without re-parsing
/// argv. The output mode and `--quiet` flag are installed into the
/// process-wide UI state via [`output::install_active_mode`] before
/// dispatch, so all UI helpers and progress-bar construction sites
/// observe the resolved mode.
///
/// # Arguments
///
/// * `config_service` - The configuration service to use
pub async fn run_with_config(config_service: &dyn crate::config::ConfigService) -> RunOutcome {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // `main.rs` is responsible for rendering clap errors with
            // mode-aware envelopes. When this function is invoked
            // directly (tests, library callers), surface a generic
            // CommandExecution error so the caller still gets a
            // RunOutcome.
            let mode = resolve_output_mode(None);
            return RunOutcome {
                output_mode: mode,
                quiet: false,
                command: "",
                result: Err(crate::error::SubXError::CommandExecution(format!(
                    "argument parsing failed: {err}"
                ))),
            };
        }
    };

    let output_mode = resolve_output_mode(cli.output);
    let quiet = cli.quiet;
    output::install_active_mode(output_mode, quiet);
    let command = command_name(&cli.command);

    // Switch to workspace directory for file operations if specified via env or config
    if let Some(ws_env) = std::env::var_os("SUBX_WORKSPACE") {
        if let Err(e) = std::env::set_current_dir(&ws_env) {
            return RunOutcome {
                output_mode,
                quiet,
                command,
                result: Err(crate::error::SubXError::CommandExecution(format!(
                    "Failed to set workspace directory to {}: {}",
                    std::path::PathBuf::from(&ws_env).display(),
                    e
                ))),
            };
        }
    } else if let Ok(config) = config_service.get_config() {
        let ws_dir = &config.general.workspace;
        if !ws_dir.as_os_str().is_empty() {
            if let Err(e) = std::env::set_current_dir(ws_dir) {
                return RunOutcome {
                    output_mode,
                    quiet,
                    command,
                    result: Err(crate::error::SubXError::CommandExecution(format!(
                        "Failed to set workspace directory to {}: {}",
                        ws_dir.display(),
                        e
                    ))),
                };
            }
        }
    }

    let result = crate::commands::dispatcher::dispatch_command_with_ref(
        cli.command,
        config_service,
        output_mode,
    )
    .await;

    RunOutcome {
        output_mode,
        quiet,
        command,
        result,
    }
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

    // ─── Top-level --output / --quiet placement ──────────────────────────────

    #[test]
    fn test_output_flag_before_subcommand_parses() {
        // `subx --output json convert ...` must parse and set the
        // top-level `Cli.output` to `Json` while leaving the
        // subcommand-local `convert --output PATH` untouched.
        let cli = Cli::try_parse_from([
            "subx-cli", "--output", "json", "convert", "file.srt", "--output", "out.ass",
            "--format", "ass",
        ])
        .expect("parses");
        assert_eq!(cli.output, Some(OutputMode::Json));
        if let Commands::Convert(args) = cli.command {
            assert_eq!(
                args.output.as_deref(),
                Some(std::path::Path::new("out.ass"))
            );
        } else {
            panic!("expected Convert");
        }
    }

    #[test]
    fn test_convert_local_output_path_does_not_set_output_mode() {
        // `subx-cli convert --output a.ass --format ass` must parse
        // (the convert-local --output is the file path), and
        // `Cli.output` must default to None (i.e., text mode).
        let cli = Cli::try_parse_from([
            "subx-cli", "convert", "file.srt", "--output", "a.ass", "--format", "ass",
        ])
        .expect("parses");
        assert_eq!(cli.output, None);
    }

    #[test]
    fn test_output_flag_after_subcommand_does_not_apply_globally() {
        // `subx-cli convert --output json --format ass` is the convert
        // command's local file-path argument; clap routes the value
        // `json` to ConvertArgs.output rather than the top-level mode.
        // (We assert the local field captured the value; this guards
        // against accidentally making the top-level flag global.)
        let cli = Cli::try_parse_from([
            "subx-cli", "convert", "file.srt", "--output", "json", "--format", "ass",
        ])
        .expect("parses");
        assert_eq!(cli.output, None, "top-level mode must not flip");
        if let Commands::Convert(args) = cli.command {
            assert_eq!(args.output.as_deref(), Some(std::path::Path::new("json")));
        } else {
            panic!("expected Convert");
        }
    }

    #[test]
    fn test_quiet_flag_before_subcommand_parses() {
        let cli = Cli::try_parse_from(["subx-cli", "--quiet", "match", "."]).expect("parses");
        assert!(cli.quiet);
    }

    #[test]
    fn test_quiet_flag_after_subcommand_is_rejected() {
        // No subcommand currently defines a local `--quiet`; this
        // guards against accidentally making the flag global.
        let result = Cli::try_parse_from(["subx-cli", "match", ".", "--quiet"]);
        assert!(
            result.is_err(),
            "--quiet must appear before the subcommand, got: {:?}",
            result.map(|_| "unexpected ok")
        );
    }

    #[test]
    fn test_resolve_output_mode_prefers_flag_over_env() {
        unsafe {
            std::env::set_var("SUBX_OUTPUT", "json");
        }
        // Explicit flag wins.
        assert_eq!(
            super::resolve_output_mode(Some(OutputMode::Text)),
            OutputMode::Text
        );
        // Env fallback.
        assert_eq!(super::resolve_output_mode(None), OutputMode::Json);
        unsafe {
            std::env::remove_var("SUBX_OUTPUT");
        }
        assert_eq!(super::resolve_output_mode(None), OutputMode::Text);
    }

    #[test]
    fn test_command_name_returns_kebab_case() {
        let cli = Cli::try_parse_from(["subx-cli", "detect-encoding", "f.srt"]).unwrap();
        assert_eq!(super::command_name(&cli.command), "detect-encoding");
        let cli = Cli::try_parse_from(["subx-cli", "match", "."]).unwrap();
        assert_eq!(super::command_name(&cli.command), "match");
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
