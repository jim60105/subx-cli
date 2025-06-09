//! Caching utilities for the file matching engine.
//!
//! Defines cache data structures and operations to store and retrieve
//! previous matching results for faster repeated execution.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::matcher::cache::{CacheData, SnapshotItem, OpItem};
//! // Load existing cache or initialize a new one
//! ```

use serde::{Deserialize, Serialize};

/// Snapshot item representing a file state for directory comparison.
///
/// Used to detect changes in the filesystem since the last cache update.
/// Contains essential file metadata for comparison purposes.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SnapshotItem {
    /// File name (without path)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Last modification time as Unix timestamp
    pub mtime: u64,
    /// File type classification (e.g., "video", "subtitle")
    pub file_type: String,
}

/// Single match operation cache item storing result details.
///
/// Represents a cached match operation between a video and subtitle file,
/// including all the analysis results and metadata.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpItem {
    /// Path to the video file
    pub video_file: String,
    /// Path to the subtitle file
    pub subtitle_file: String,
    /// The new name assigned to the subtitle file
    pub new_subtitle_name: String,
    /// Confidence score of the match (0.0 to 1.0)
    pub confidence: f32,
    /// List of reasoning factors for this match
    pub reasoning: Vec<String>,
}

/// Dry-run cache data structure containing snapshot and match history.
///
/// Stores the complete state of a directory scan and match operations,
/// enabling efficient incremental processing and result caching.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    /// Version of the cache format for compatibility checking
    pub cache_version: String,
    /// Path to the directory that was processed
    pub directory: String,
    /// Snapshot of all files found during scanning
    pub file_snapshot: Vec<SnapshotItem>,
    /// List of all match operations performed
    pub match_operations: Vec<OpItem>,
    /// Timestamp when the cache was created
    pub created_at: u64,
    /// AI model used for matching operations
    pub ai_model_used: String,
    /// Hash of configuration used for matching
    pub config_hash: String,
}

impl CacheData {
    /// Loads cache data from the specified file path.
    pub fn load(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }
}
