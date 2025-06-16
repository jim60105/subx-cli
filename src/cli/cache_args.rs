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

use clap::{Args, Subcommand};

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
    /// Clear all cached data including dry-run results and AI analysis
    Clear,
}
