//! Cache management command-line arguments and operations.
//!
//! This module defines the command-line interface for cache-related operations
//! in SubX. The cache system stores intermediate results from AI analysis and
//! other computationally expensive operations to improve performance and reduce
//! API costs during development and testing.
//!
//! # Cache System Overview
//!
//! SubX maintains several types of caches:
//! - **AI Analysis Cache**: Results from AI-powered file matching
//! - **Audio Analysis Cache**: Speech pattern detection results
//! - **Configuration Cache**: Processed configuration data
//! - **Dry-run Cache**: Preview results that can be applied later
//!
//! # Cache Benefits
//!
//! - **Performance**: Avoid re-analyzing the same content
//! - **Cost Savings**: Reduce AI API calls during testing
//! - **Development**: Faster iteration during debugging
//! - **Reliability**: Consistent results across multiple runs
//!
//! # Examples
//!
//! ```bash
//! # Clear all cache data
//! subx cache clear
//!
//! # View cache status (future enhancement)
//! subx cache status
//!
//! # Selective cache clearing (future enhancement)
//! subx cache clear --type ai
//! ```

use clap::{Args, Subcommand, ValueEnum};

/// Command-line arguments for cache management operations.
#[derive(Args, Debug)]
pub struct CacheArgs {
    /// The cache management action to perform
    #[command(subcommand)]
    pub action: CacheAction,
}

/// Cache management operations and subcommands.
#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Display cache status including size, age, and validity
    Status(StatusArgs),
    /// Apply cached dry-run results without calling the AI provider
    Apply(ApplyArgs),
    /// Undo the most recent batch of file operations
    Rollback(RollbackArgs),
    /// Clear cached data
    Clear(ClearArgs),
}

/// Arguments for the `cache status` subcommand.
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Output status in JSON format for scripting
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the `cache apply` subcommand.
#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Skip interactive confirmation prompt
    #[arg(long)]
    pub yes: bool,
    /// Bypass staleness and validation checks
    #[arg(long)]
    pub force: bool,
    /// Minimum confidence threshold (0-100) to filter operations
    #[arg(long, value_parser = clap::value_parser!(u8).range(0..=100))]
    pub confidence: Option<u8>,
}

/// Arguments for the `cache rollback` subcommand.
#[derive(Args, Debug)]
pub struct RollbackArgs {
    /// Bypass integrity verification checks
    #[arg(long)]
    pub force: bool,
}

/// Arguments for the `cache clear` subcommand.
#[derive(Args, Debug)]
pub struct ClearArgs {
    /// Type of data to clear
    #[arg(long, value_enum, default_value_t = ClearType::All)]
    pub r#type: ClearType,
}

/// Types of cache data that can be cleared.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    /// Clear only the match cache file
    Cache,
    /// Clear only the operation journal
    Journal,
    /// Clear both cache and journal
    All,
}
