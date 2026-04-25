//! Shared validation helpers for archive extraction.
//!
//! Centralises path-traversal checking, decompression-bomb limits, and
//! size / entry-count tracking so that every format-specific extractor
//! enforces identical security invariants.

use std::io;
use std::path::{Component, Path, PathBuf};

use log::warn;

/// Maximum total expanded size per archive (1 GiB).
pub(super) const MAX_EXPANDED_SIZE: u64 = 1024 * 1024 * 1024;

/// Maximum number of entries per archive.
pub(super) const MAX_ENTRY_COUNT: usize = 10_000;

/// Validates that `entry_path` is safe to extract under `dest_dir`.
///
/// Returns the resolved target path on success, or `None` if the entry
/// should be skipped (absolute path, parent-directory traversal, or
/// canonical-path escape).
pub(super) fn validate_entry_path(dest_dir: &Path, entry_path: &Path) -> Option<PathBuf> {
    // Reject absolute paths
    if entry_path.has_root() {
        return None;
    }

    // Reject parent-directory components
    if entry_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }

    let target = dest_dir.join(entry_path);

    // Double-check with canonical path resolution when possible
    if let Ok(canonical) = target.canonicalize() {
        let dest_canonical = dest_dir
            .canonicalize()
            .unwrap_or_else(|_| dest_dir.to_path_buf());
        if !canonical.starts_with(&dest_canonical) {
            return None;
        }
    }

    Some(target)
}

/// Tracks cumulative extracted size and entry count for bomb protection.
pub(super) struct ExtractionLimits {
    total_size: u64,
    entry_count: usize,
    archive_display: String,
}

impl ExtractionLimits {
    /// Creates a new limiter for the given archive.
    pub fn new(archive_path: &Path) -> Self {
        Self {
            total_size: 0,
            entry_count: 0,
            archive_display: archive_path.display().to_string(),
        }
    }

    /// Accounts for one more entry of `uncompressed_size` bytes.
    ///
    /// Returns `Err` if either the entry count or the cumulative size
    /// limit is exceeded.
    pub fn check_entry(&mut self, uncompressed_size: u64) -> io::Result<()> {
        self.entry_count += 1;
        if self.entry_count > MAX_ENTRY_COUNT {
            warn!(
                "Archive {} exceeds maximum entry count ({MAX_ENTRY_COUNT}), aborting extraction",
                self.archive_display
            );
            return Err(io::Error::other(format!(
                "Archive exceeds maximum entry count ({MAX_ENTRY_COUNT})"
            )));
        }

        self.total_size += uncompressed_size;
        if self.total_size > MAX_EXPANDED_SIZE {
            warn!(
                "Archive {} exceeds maximum expanded size (1 GiB), aborting extraction",
                self.archive_display
            );
            return Err(io::Error::other(
                "Archive exceeds maximum expanded size (1 GiB)",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_normal_path() {
        let tmp = TempDir::new().unwrap();
        let result = validate_entry_path(tmp.path(), Path::new("subdir/file.srt"));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), tmp.path().join("subdir/file.srt"));
    }

    #[test]
    fn validate_rejects_absolute_path() {
        let tmp = TempDir::new().unwrap();
        assert!(validate_entry_path(tmp.path(), Path::new("/etc/passwd")).is_none());
    }

    #[test]
    fn validate_rejects_parent_traversal() {
        let tmp = TempDir::new().unwrap();
        assert!(validate_entry_path(tmp.path(), Path::new("../../etc/passwd")).is_none());
    }

    #[test]
    fn limits_entry_count() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.zip");
        let mut limits = ExtractionLimits::new(&path);
        for _ in 0..MAX_ENTRY_COUNT {
            assert!(limits.check_entry(1).is_ok());
        }
        assert!(limits.check_entry(1).is_err());
    }

    #[test]
    fn limits_total_size() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.zip");
        let mut limits = ExtractionLimits::new(&path);
        assert!(limits.check_entry(MAX_EXPANDED_SIZE).is_ok());
        assert!(limits.check_entry(1).is_err());
    }
}
