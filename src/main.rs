//! SubX CLI Application Entry Point
//!
//! This module contains the main entry point for the SubX subtitle processing
//! command-line application. It initializes logging, configuration management,
//! and handles the application lifecycle.
//!
//! # Output mode resolution
//!
//! Before clap parses argv this binary performs a permissive sniff of
//! `--output`/`--output=`/`SUBX_OUTPUT` so it can render an envelope
//! when clap rejects the arguments. `Cli::try_parse()` is then used
//! (instead of `parse()`) so we can distinguish `--help`/`--version`
//! (always text, exit 0) from genuine parse errors (synthetic JSON
//! envelope when the tentative mode is JSON).

use std::ffi::OsString;
use subx_cli::cli::output::{self, OutputMode};

#[tokio::main]
async fn main() {
    // 1. Tentative output mode for clap-error rendering.
    let argv: Vec<OsString> = std::env::args_os().collect();
    let tentative_mode = sniff_output_mode(&argv);
    let tentative_quiet = sniff_quiet(&argv);

    // Initialize logging subsystem. When `--quiet` is combined with
    // `--output json`, the machine-readable-output spec requires that
    // structured `tracing`/`log` records on stderr be suppressed in
    // addition to free-form chatter, so we install a hard `Off` filter
    // that wins over any `RUST_LOG` value the user may have set.
    if tentative_quiet && tentative_mode.is_json() {
        let _ = env_logger::Builder::new()
            .filter_level(log::LevelFilter::Off)
            .try_init();
    } else {
        env_logger::init();
    }

    // 2. Try to parse argv; help/version are routed through clap's own
    //    text rendering even in tentative-JSON mode.
    let cli_parse = <subx_cli::cli::Cli as clap::Parser>::try_parse_from(argv.iter());

    if let Err(err) = cli_parse {
        handle_clap_error(err, tentative_mode);
        // handle_clap_error never returns.
    }

    // 3. Successful parse — run the application.
    let outcome = run_application().await;

    // 4. Render final envelope on Err.
    match outcome.result {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            if outcome.output_mode.is_json() {
                output::emit_error(outcome.output_mode, outcome.command, &e);
            } else {
                eprintln!("{}", e.user_friendly_message());
            }
            std::process::exit(e.exit_code());
        }
    }
}

/// Permissive scan of argv for the `--quiet` flag.
///
/// Mirrors [`sniff_output_mode`]'s placement constraint: only flags
/// that appear *before* the subcommand token are considered, matching
/// clap's own parsing rules for the top-level boolean.
fn sniff_quiet(argv: &[OsString]) -> bool {
    let mut iter = argv.iter().skip(1);
    while let Some(arg) = iter.next() {
        let s = match arg.to_str() {
            Some(s) => s,
            None => continue,
        };
        if s == "--quiet" {
            return true;
        }
        if s == "--output" {
            // Skip the value token so it isn't misread as the subcommand.
            let _ = iter.next();
            continue;
        }
        if s.starts_with("--output=") {
            continue;
        }
        if !s.starts_with('-') {
            // First positional token is the subcommand; stop scanning.
            break;
        }
    }
    false
}

/// Permissive scan of argv + `SUBX_OUTPUT` for the tentative output mode.
///
/// The sniff stops at the first non-option token (treated as the
/// subcommand), respecting the documented "flag must precede the
/// subcommand" rule. If the mode cannot be resolved the function
/// returns [`OutputMode::Text`].
fn sniff_output_mode(argv: &[OsString]) -> OutputMode {
    let mut iter = argv.iter().skip(1);
    while let Some(arg) = iter.next() {
        let s = match arg.to_str() {
            Some(s) => s,
            None => continue,
        };
        if let Some(value) = s.strip_prefix("--output=") {
            if let Some(mode) = OutputMode::from_token(value) {
                return mode;
            }
        } else if s == "--output" {
            if let Some(value) = iter.next().and_then(|v| v.to_str())
                && let Some(mode) = OutputMode::from_token(value)
            {
                return mode;
            }
        } else if !s.starts_with('-') {
            // First positional token is the subcommand; stop scanning.
            break;
        }
    }
    if let Ok(value) = std::env::var("SUBX_OUTPUT")
        && let Some(mode) = OutputMode::from_token(&value)
    {
        return mode;
    }
    OutputMode::Text
}

/// Render a clap parse error and exit.
///
/// `--help` and `--version` paths (kinds `DisplayHelp`,
/// `DisplayVersion`, `DisplayHelpOnMissingArgumentOrSubcommand`) are
/// always rendered as clap's own text output regardless of the
/// tentative mode, matching the documented exemption.
fn handle_clap_error(err: clap::Error, tentative_mode: OutputMode) -> ! {
    use clap::error::ErrorKind;

    let exit_code = err.exit_code();
    match err.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            // Help/version: preserve today's text rendering verbatim.
            let _ = err.print();
            std::process::exit(exit_code);
        }
        _ => {
            if tentative_mode.is_json() {
                let rendered = err.render().to_string();
                let message = output::strip_ansi(&rendered);
                output::emit_argument_parsing_error(None, message, exit_code);
            } else {
                let _ = err.print();
            }
            std::process::exit(exit_code);
        }
    }
}

/// Main application runner with proper error handling.
///
/// This function uses the new CLI interface with dependency injection
/// and returns the structured [`subx_cli::cli::RunOutcome`] so the
/// process boundary can render the final envelope without re-parsing
/// argv.
async fn run_application() -> subx_cli::cli::RunOutcome {
    match subx_cli::config::ProductionConfigService::new() {
        Ok(config_service) => subx_cli::cli::run_with_config(&config_service).await,
        Err(e) => subx_cli::cli::RunOutcome {
            output_mode: output::active_mode(),
            quiet: false,
            command: "",
            result: Err(e),
        },
    }
}
