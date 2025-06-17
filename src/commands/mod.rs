//! SubX command execution module.
//!
//! This module contains implementations of each CLI subcommand's business logic,
//! including AI matching, format conversion, synchronization, encoding detection,
//! configuration management, and cache operations.
//!
//! The `dispatcher` module provides centralized command routing to eliminate
//! code duplication between CLI and library API interfaces.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::Cli;
//! use subx_cli::commands;
//!
//! // Execute the match command logic
//! async fn run_match() -> subx_cli::Result<()> {
//!     let args = Cli::parse().command;
//!     commands::match_command::execute(args).await
//! }
//! ```
pub mod cache_command;
pub mod config_command;
pub mod convert_command;
pub mod detect_encoding_command;
/// Central command dispatcher for unified command execution across CLI and library interfaces.
pub mod dispatcher;
pub mod match_command;
pub mod sync_command;
