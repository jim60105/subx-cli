//! Media file discovery utilities.
//!
//! This module provides `FileDiscovery` to scan directories,
//! classify media files (video and subtitle), and collect metadata needed for matching.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::matcher::discovery::FileDiscovery;
//! let disco = FileDiscovery::new();
//! let files = disco.scan_directory("./path".as_ref(), true).unwrap();
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::Result;

/// Media file record representing a discovered file.
///
/// Contains metadata about a media file discovered during the scanning process,
/// including its path, type classification, and basic file properties.
#[derive(Debug, Clone)]
pub struct MediaFile {
    /// Unique identifier for this media file (deterministic hash)
    pub id: String,
    /// Full path to the media file
    pub path: PathBuf,
    /// Classification of the file (Video or Subtitle)
    pub file_type: MediaFileType,
    /// File size in bytes
    pub size: u64,
    /// Complete filename with extension (e.g., "movie.mkv")
    pub name: String,
    /// File extension (without the dot)
    pub extension: String,
    /// Relative path from scan root for recursive matching
    pub relative_path: String,
}
/// Generate a deterministic unique identifier for a media file
///
/// Uses a fast hash algorithm combining the absolute path and file size to
/// produce a consistent ID regardless of scanning method.
pub fn generate_file_id(path: &std::path::Path, file_size: u64) -> String {
    let mut hasher = DefaultHasher::new();
    // Use absolute path to ensure consistency across different scanning methods
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    abs_path.to_string_lossy().as_ref().hash(&mut hasher);
    file_size.hash(&mut hasher);
    format!("file_{:016x}", hasher.finish())
}

// Unit tests: FileDiscovery file matching logic
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files(dir: &std::path::Path) {
        let _ = fs::write(dir.join("video1.mp4"), b"");
        let _ = fs::write(dir.join("video2.mkv"), b"");
        let _ = fs::write(dir.join("subtitle1.srt"), b"");
        let sub = dir.join("season1");
        fs::create_dir_all(&sub).unwrap();
        let _ = fs::write(sub.join("episode1.mp4"), b"");
        let _ = fs::write(sub.join("episode1.srt"), b"");
        let _ = fs::write(dir.join("note.txt"), b"");
    }

    #[test]
    fn test_file_discovery_non_recursive() {
        let temp = TempDir::new().unwrap();
        create_test_files(temp.path());
        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), false).unwrap();
        let vids = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .count();
        let subs = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .count();
        assert_eq!(vids, 2);
        assert_eq!(subs, 1);
        assert!(!files.iter().any(|f| f.relative_path.contains("episode1")));
    }

    #[test]
    fn test_file_discovery_recursive() {
        let temp = TempDir::new().unwrap();
        create_test_files(temp.path());
        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), true).unwrap();
        let vids = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .count();
        let subs = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .count();
        assert_eq!(vids, 3);
        assert_eq!(subs, 2);
        assert!(files.iter().any(|f| f.relative_path.contains("episode1")));
    }

    #[test]
    fn test_file_classification_and_extensions() {
        let temp = TempDir::new().unwrap();
        let v = temp.path().join("t.mp4");
        fs::write(&v, b"").unwrap();
        let s = temp.path().join("t.srt");
        fs::write(&s, b"").unwrap();
        let x = temp.path().join("t.txt");
        fs::write(&x, b"").unwrap();
        let disco = FileDiscovery::new();
        let vf = disco.classify_file(&v, temp.path()).unwrap().unwrap();
        assert!(matches!(vf.file_type, MediaFileType::Video));
        assert_eq!(vf.name, "t.mp4");
        let sf = disco.classify_file(&s, temp.path()).unwrap().unwrap();
        assert!(matches!(sf.file_type, MediaFileType::Subtitle));
        assert_eq!(sf.name, "t.srt");
        let none = disco.classify_file(&x, temp.path()).unwrap();
        assert!(none.is_none());
        assert!(disco.video_extensions.contains(&"mp4".to_string()));
        assert!(disco.subtitle_extensions.contains(&"srt".to_string()));
    }

    #[test]
    fn test_empty_and_nonexistent_directory() {
        let temp = TempDir::new().unwrap();
        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), false).unwrap();
        assert!(files.is_empty());
        let res = disco.scan_directory(&std::path::Path::new("/nonexistent/path"), false);
        assert!(res.is_err());
    }
}

// Unit tests for unique ID generation and MediaFile structure
#[cfg(test)]
mod id_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_media_file_structure_with_unique_id() {
        let temp = TempDir::new().unwrap();
        let video_path = temp.path().join("[Test][01].mkv");
        fs::write(&video_path, b"dummy content").unwrap();

        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), false).unwrap();

        let video_file = files
            .iter()
            .find(|f| matches!(f.file_type, MediaFileType::Video))
            .unwrap();

        assert!(!video_file.id.is_empty());
        assert!(video_file.id.starts_with("file_"));
        assert_eq!(video_file.id.len(), 21);

        assert_eq!(video_file.name, "[Test][01].mkv");
        assert_eq!(video_file.extension, "mkv");
        assert_eq!(video_file.relative_path, "[Test][01].mkv");
    }

    #[test]
    fn test_deterministic_id_generation() {
        use std::path::Path;
        let path1 = Path::new("test/file.mkv");
        let path2 = Path::new("test/file.mkv");
        let path3 = Path::new("test/file2.mkv");

        let id1 = generate_file_id(path1, 1000);
        let id2 = generate_file_id(path2, 1000);
        assert_eq!(id1, id2);

        let id3 = generate_file_id(path3, 1000);
        assert_ne!(id1, id3);

        let id4 = generate_file_id(path1, 2000);
        assert_ne!(id1, id4);

        assert!(id1.starts_with("file_"));
        assert_eq!(id1.len(), 21);
    }

    #[test]
    fn test_recursive_mode_with_unique_ids() {
        let temp = TempDir::new().unwrap();
        let sub_dir = temp.path().join("season1");
        fs::create_dir_all(&sub_dir).unwrap();

        let video1 = temp.path().join("movie.mkv");
        let video2 = sub_dir.join("episode1.mkv");
        fs::write(&video1, b"content1").unwrap();
        fs::write(&video2, b"content2").unwrap();

        let disco = FileDiscovery::new();
        let files = disco.scan_directory(temp.path(), true).unwrap();

        let root_video = files.iter().find(|f| f.name == "movie.mkv").unwrap();
        let sub_video = files.iter().find(|f| f.name == "episode1.mkv").unwrap();

        assert_ne!(root_video.id, sub_video.id);
        assert_eq!(root_video.relative_path, "movie.mkv");
        assert_eq!(sub_video.relative_path, "season1/episode1.mkv");
    }

    #[test]
    fn test_hash_generation_basic() {
        use std::path::Path;
        let path = Path::new("test/file.mkv");
        let id = generate_file_id(path, 1000);
        assert!(id.starts_with("file_"));
        assert_eq!(id.len(), 21);
    }
}

impl Default for FileDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Enumeration of supported media file types.
///
/// Classifies discovered files into their primary categories for
/// processing by the subtitle matching system.
#[derive(Debug, Clone)]
pub enum MediaFileType {
    /// Video file (e.g., .mp4, .mkv, .avi)
    Video,
    /// Subtitle file (e.g., .srt, .ass, .vtt)
    Subtitle,
}

/// File discovery engine for scanning and classifying media files.
pub struct FileDiscovery {
    video_extensions: Vec<String>,
    subtitle_extensions: Vec<String>,
}

impl FileDiscovery {
    /// Creates a new `FileDiscovery` with default video and subtitle extensions.
    pub fn new() -> Self {
        Self {
            video_extensions: vec![
                "mp4".to_string(),
                "mkv".to_string(),
                "avi".to_string(),
                "mov".to_string(),
                "wmv".to_string(),
                "flv".to_string(),
                "m4v".to_string(),
                "webm".to_string(),
            ],
            subtitle_extensions: vec![
                "srt".to_string(),
                "ass".to_string(),
                "vtt".to_string(),
                "sub".to_string(),
                "ssa".to_string(),
                "idx".to_string(),
            ],
        }
    }

    /// Scans the given directory and returns all media files found.
    ///
    /// # Arguments
    ///
    /// * `path` - The root directory to scan.
    /// * `recursive` - Whether to scan subdirectories recursively.
    pub fn scan_directory(&self, root_path: &Path, recursive: bool) -> Result<Vec<MediaFile>> {
        let mut files = Vec::new();

        let walker = if recursive {
            WalkDir::new(root_path).into_iter()
        } else {
            WalkDir::new(root_path).max_depth(1).into_iter()
        };

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(media_file) = self.classify_file(path, root_path)? {
                    files.push(media_file);
                }
            }
        }

        Ok(files)
    }

    /// Creates MediaFile objects from a list of file paths.
    ///
    /// This method processes each file path individually, creating MediaFile objects
    /// with consistent IDs that match those generated by scan_directory.
    ///
    /// # Arguments
    ///
    /// * `file_paths` - A slice of file paths to process
    ///
    /// # Returns
    ///
    /// A vector of `MediaFile` objects for valid media files, or an error if file access fails.
    pub fn scan_file_list(&self, file_paths: &[PathBuf]) -> Result<Vec<MediaFile>> {
        let mut media_files = Vec::new();

        for path in file_paths {
            if !path.exists() {
                continue; // Skip non-existent files
            }

            if !path.is_file() {
                continue; // Skip directories
            }

            if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                let extension_lower = extension.to_lowercase();

                // Check if it's a video or subtitle file
                let file_type = if self.video_extensions.contains(&extension_lower) {
                    MediaFileType::Video
                } else if self.subtitle_extensions.contains(&extension_lower) {
                    MediaFileType::Subtitle
                } else {
                    continue; // Skip non-media files
                };

                if let Ok(metadata) = path.metadata() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    // For file list scanning, use filename as relative path
                    // This maintains compatibility with existing display logic
                    let relative_path = name.clone();

                    let media_file = MediaFile {
                        id: generate_file_id(path, metadata.len()),
                        path: path.clone(),
                        file_type,
                        size: metadata.len(),
                        name,
                        extension: extension_lower,
                        relative_path,
                    };
                    media_files.push(media_file);
                }
            }
        }

        Ok(media_files)
    }

    /// Classifies a file by its extension and gathers its metadata.
    ///
    /// Returns `Some(MediaFile)` if the file is a recognized media type,
    /// or `None` otherwise.
    fn classify_file(&self, path: &Path, scan_root: &Path) -> Result<Option<MediaFile>> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let file_type = if self.video_extensions.contains(&extension) {
            MediaFileType::Video
        } else if self.subtitle_extensions.contains(&extension) {
            MediaFileType::Subtitle
        } else {
            return Ok(None);
        };

        let metadata = std::fs::metadata(path)?;
        // Complete filename with extension
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        // Compute relative path with normalized separators
        let relative_path = path
            .strip_prefix(scan_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/"); // Normalize to Unix-style separators for consistency

        // Generate unique ID based on absolute path and file size
        let id = generate_file_id(path, metadata.len());

        Ok(Some(MediaFile {
            id,
            path: path.to_path_buf(),
            file_type,
            size: metadata.len(),
            name,
            extension,
            relative_path,
        }))
    }
}
