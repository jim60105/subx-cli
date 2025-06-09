use serde::{Deserialize, Serialize};

/// Snapshot item representing a file state for directory comparison.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SnapshotItem {
    pub name: String,
    pub size: u64,
    pub mtime: u64,
    pub file_type: String,
}

/// Single match operation cache item storing result details.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpItem {
    pub video_file: String,
    pub subtitle_file: String,
    pub new_subtitle_name: String,
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

/// Dry-run cache data structure containing snapshot and match history.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    pub cache_version: String,
    pub directory: String,
    pub file_snapshot: Vec<SnapshotItem>,
    pub match_operations: Vec<OpItem>,
    pub created_at: u64,
    pub ai_model_used: String,
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
