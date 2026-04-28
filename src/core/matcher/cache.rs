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
use std::path::PathBuf;

/// Snapshot item representing a file state for directory comparison.
///
/// Used to detect changes in the filesystem since the last cache update.
/// Contains essential file metadata for comparison purposes.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SnapshotItem {
    /// Canonical absolute path to the file.
    ///
    /// Defaults to an empty string when deserializing cache files produced
    /// by older SubX versions that did not record the full path.
    #[serde(default)]
    pub path: String,
    /// File name (without path)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// Last modification time as Unix timestamp
    pub mtime: u64,
    /// File type classification (e.g., "video", "subtitle")
    pub file_type: String,
}

/// Describes a file whose on-disk state no longer matches the cached snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleFile {
    /// The absolute path of the file (as recorded in the snapshot).
    pub path: String,
    /// Human-readable reason explaining why the entry is stale.
    pub reason: String,
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
    /// Records the relocation mode when the cache was generated
    #[serde(default)]
    pub original_relocation_mode: String,
    /// Records whether backup was enabled when the cache was generated
    #[serde(default)]
    pub original_backup_enabled: bool,
}

impl CacheData {
    /// Loads cache data from the specified file path.
    pub fn load(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Returns `true` when the cache carries no file snapshot entries.
    ///
    /// Legacy caches generated before snapshot population was implemented
    /// will report `true`; callers should treat such caches as unable to
    /// perform freshness validation.
    pub fn has_empty_snapshot(&self) -> bool {
        self.file_snapshot.is_empty()
    }

    /// Validates every snapshot entry against the current filesystem state.
    ///
    /// Returns a list of [`StaleFile`] records describing each file whose
    /// on-disk state diverges from the snapshot (missing, size mismatch, or
    /// modification-time mismatch). An empty result indicates that the
    /// snapshot is still consistent with the filesystem.
    pub fn validate_snapshot(&self) -> Vec<StaleFile> {
        let mut stale = Vec::new();
        for item in &self.file_snapshot {
            if item.path.is_empty() {
                stale.push(StaleFile {
                    path: item.name.clone(),
                    reason: "snapshot entry missing canonical path".to_string(),
                });
                continue;
            }

            let path = std::path::Path::new(&item.path);
            let metadata = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(e) => {
                    stale.push(StaleFile {
                        path: item.path.clone(),
                        reason: format!("file missing or inaccessible: {}", e),
                    });
                    continue;
                }
            };

            if metadata.len() != item.size {
                stale.push(StaleFile {
                    path: item.path.clone(),
                    reason: format!(
                        "size changed (snapshot={}, current={})",
                        item.size,
                        metadata.len()
                    ),
                });
                continue;
            }

            let current_mtime = metadata
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if current_mtime != item.mtime {
                stale.push(StaleFile {
                    path: item.path.clone(),
                    reason: format!(
                        "mtime changed (snapshot={}, current={})",
                        item.mtime, current_mtime
                    ),
                });
            }
        }
        stale
    }

    /// Checks whether any planned subtitle target path already exists on disk.
    ///
    /// For every cached operation, the target path is computed based on the
    /// recorded relocation mode: when the mode is anything other than
    /// `"None"`, the target resides next to the video file; otherwise it
    /// stays alongside the original subtitle file. Returns the list of
    /// target paths that already exist and would therefore conflict with
    /// applying the cached plan.
    pub fn validate_target_paths(&self) -> Vec<PathBuf> {
        let mut conflicts = Vec::new();
        let relocation_mode = self.original_relocation_mode.as_str();
        let relocates = !matches!(relocation_mode, "" | "None");

        for op in &self.match_operations {
            let parent = if relocates {
                std::path::Path::new(&op.video_file).parent()
            } else {
                std::path::Path::new(&op.subtitle_file).parent()
            };

            let Some(parent) = parent else { continue };
            let target = parent.join(&op.new_subtitle_name);

            let source = std::path::Path::new(&op.subtitle_file);
            if target.exists() && target != source {
                conflicts.push(target);
            }
        }
        conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::tempdir;

    fn snapshot_for(path: &std::path::Path, file_type: &str) -> SnapshotItem {
        let meta = fs::metadata(path).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        SnapshotItem {
            path: path.to_string_lossy().to_string(),
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            size: meta.len(),
            mtime,
            file_type: file_type.to_string(),
        }
    }

    fn make_cache(snapshot: Vec<SnapshotItem>, ops: Vec<OpItem>, mode: &str) -> CacheData {
        CacheData {
            cache_version: "2.0".to_string(),
            directory: String::new(),
            file_snapshot: snapshot,
            match_operations: ops,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ai_model_used: "test".to_string(),
            config_hash: "hash".to_string(),
            original_relocation_mode: mode.to_string(),
            original_backup_enabled: false,
        }
    }

    #[test]
    fn validate_snapshot_returns_empty_when_files_match() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        let subtitle = dir.path().join("movie.srt");
        fs::write(&video, b"video").unwrap();
        fs::write(&subtitle, b"sub").unwrap();

        let snapshot = vec![
            snapshot_for(&video, "video"),
            snapshot_for(&subtitle, "subtitle"),
        ];
        let cache = make_cache(snapshot, vec![], "None");
        assert!(cache.validate_snapshot().is_empty());
    }

    #[test]
    fn validate_snapshot_detects_modified_file() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        fs::write(&video, b"video").unwrap();
        let snap = snapshot_for(&video, "video");

        // Modify file contents so size changes.
        let mut f = fs::OpenOptions::new().write(true).open(&video).unwrap();
        f.write_all(b"video-edited-and-grown").unwrap();
        drop(f);

        let cache = make_cache(vec![snap], vec![], "None");
        let stale = cache.validate_snapshot();
        assert_eq!(stale.len(), 1);
        assert!(stale[0].reason.contains("size changed"));
    }

    #[test]
    fn validate_snapshot_detects_missing_file() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        fs::write(&video, b"video").unwrap();
        let snap = snapshot_for(&video, "video");
        fs::remove_file(&video).unwrap();

        let cache = make_cache(vec![snap], vec![], "None");
        let stale = cache.validate_snapshot();
        assert_eq!(stale.len(), 1);
        assert!(stale[0].reason.contains("missing"));
    }

    #[test]
    fn validate_target_paths_returns_empty_when_no_conflict() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        let subtitle = dir.path().join("original.srt");
        fs::write(&video, b"video").unwrap();
        fs::write(&subtitle, b"sub").unwrap();

        let op = OpItem {
            video_file: video.to_string_lossy().to_string(),
            subtitle_file: subtitle.to_string_lossy().to_string(),
            new_subtitle_name: "movie.srt".to_string(),
            confidence: 0.9,
            reasoning: vec![],
        };
        let cache = make_cache(vec![], vec![op], "None");
        assert!(cache.validate_target_paths().is_empty());
    }

    #[test]
    fn validate_target_paths_detects_existing_target() {
        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        let subtitle = dir.path().join("original.srt");
        let existing = dir.path().join("movie.srt");
        fs::write(&video, b"video").unwrap();
        fs::write(&subtitle, b"sub").unwrap();
        fs::write(&existing, b"conflict").unwrap();

        let op = OpItem {
            video_file: video.to_string_lossy().to_string(),
            subtitle_file: subtitle.to_string_lossy().to_string(),
            new_subtitle_name: "movie.srt".to_string(),
            confidence: 0.9,
            reasoning: vec![],
        };
        let cache = make_cache(vec![], vec![op], "None");
        let conflicts = cache.validate_target_paths();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0], existing);
    }

    #[test]
    fn validate_target_paths_uses_video_dir_when_relocating() {
        let dir = tempdir().unwrap();
        let video_dir = dir.path().join("videos");
        let sub_dir = dir.path().join("subs");
        fs::create_dir_all(&video_dir).unwrap();
        fs::create_dir_all(&sub_dir).unwrap();

        let video = video_dir.join("movie.mkv");
        let subtitle = sub_dir.join("original.srt");
        let target = video_dir.join("movie.srt");
        fs::write(&video, b"video").unwrap();
        fs::write(&subtitle, b"sub").unwrap();
        fs::write(&target, b"conflict").unwrap();

        let op = OpItem {
            video_file: video.to_string_lossy().to_string(),
            subtitle_file: subtitle.to_string_lossy().to_string(),
            new_subtitle_name: "movie.srt".to_string(),
            confidence: 0.9,
            reasoning: vec![],
        };
        let cache = make_cache(vec![], vec![op], "Copy");
        let conflicts = cache.validate_target_paths();
        assert_eq!(conflicts, vec![target]);
    }

    #[test]
    fn has_empty_snapshot_reports_legacy_caches() {
        let legacy = make_cache(vec![], vec![], "None");
        assert!(legacy.has_empty_snapshot());

        let dir = tempdir().unwrap();
        let video = dir.path().join("movie.mkv");
        fs::write(&video, b"video").unwrap();
        let populated = make_cache(vec![snapshot_for(&video, "video")], vec![], "None");
        assert!(!populated.has_empty_snapshot());
    }
}
