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
///
/// The cache command provides utilities for managing SubX's internal cache
/// system, which stores results from computationally expensive operations
/// such as AI analysis, audio processing, and file matching results.
///
/// # Cache Types Managed
///
/// - **AI Analysis Results**: Cached responses from AI matching operations
/// - **Dry-run Results**: Preview results that haven't been applied yet
/// - **Audio Analysis Data**: Speech pattern detection and timing analysis
/// - **Processed Configuration**: Validated and normalized configuration data
///
/// # Use Cases
///
/// - **Development**: Clear cache between test runs
/// - **Troubleshooting**: Reset cache when encountering issues
/// - **Maintenance**: Periodic cleanup of stale cache data
/// - **Testing**: Ensure clean state for automated tests
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::{CacheArgs, CacheAction};
///
/// let clear_args = CacheArgs {
///     action: CacheAction::Clear,
/// };
/// ```
#[derive(Args, Debug)]
pub struct CacheArgs {
    /// The cache management action to perform.
    ///
    /// Specifies which cache operation should be executed. Currently supports
    /// clearing operations, with plans for additional cache management features
    /// such as status viewing, selective clearing, and cache statistics.
    #[command(subcommand)]
    pub action: CacheAction,
}

/// Cache management operations and subcommands.
///
/// Defines the available cache management operations that can be performed
/// through the SubX CLI. Each operation targets specific aspects of the
/// cache system to provide granular control over cached data.
///
/// # Current Operations
///
/// - **Clear**: Remove all or specific types of cached data
///
/// # Future Enhancements
///
/// Planned cache operations include:
/// - **Status**: View cache size, age, and utilization statistics
/// - **Validate**: Check cache integrity and consistency
/// - **Optimize**: Compact and defragment cache storage
/// - **Export/Import**: Backup and restore cache data
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::CacheAction;
///
/// // Clear all cache data
/// let clear_action = CacheAction::Clear;
/// ```
#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Clear all cached data including dry-run results and AI analysis.
    ///
    /// This operation removes all cached data from the SubX cache system,
    /// including:
    ///
    /// # Data Cleared
    ///
    /// - **Dry-run Results**: Cached matching results from preview operations
    /// - **AI Analysis Cache**: Stored results from AI-powered file matching
    /// - **Audio Analysis Cache**: Speech pattern detection results
    /// - **Temporary Files**: Processing artifacts and intermediate data
    ///
    /// # Impact
    ///
    /// After clearing the cache:
    /// - Next AI operations will require fresh API calls (increased cost/time)
    /// - Audio analysis will need to be re-performed
    /// - Dry-run results will need to be regenerated
    /// - Configuration cache will be rebuilt on next access
    ///
    /// # Safety
    ///
    /// This operation is safe and non-destructive:
    /// - No actual subtitle or video files are affected
    /// - Only internal cache data is removed
    /// - All data can be regenerated through normal operations
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Clear all cache data
    /// subx cache clear
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Development**: Reset state between test iterations
    /// - **Troubleshooting**: Clear potentially corrupted cache data
    /// - **Maintenance**: Periodic cleanup to reclaim disk space
    /// - **Fresh Start**: Ensure clean state for important operations
    Clear,
}
