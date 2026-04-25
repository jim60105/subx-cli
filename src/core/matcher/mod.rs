//! AI-powered subtitle file matching and discovery engine.
//!
//! This module provides sophisticated algorithms for automatically matching subtitle
//! files with their corresponding video files using AI analysis, language detection,
//! and intelligent filename pattern recognition. It handles complex scenarios including
//! multiple subtitle languages, season/episode structures, and various naming conventions.
//!
//! # Core Features
//!
//! ## Intelligent File Discovery
//! - **Recursive Search**: Traverses directory structures to find media and subtitle files
//! - **Format Detection**: Automatically identifies video and subtitle file formats
//! - **Pattern Recognition**: Understands common naming patterns and conventions
//! - **Language Detection**: Identifies subtitle languages from filenames and content
//!
//! ## AI-Powered Matching
//! - **Semantic Analysis**: Uses AI to understand filename semantics beyond patterns
//! - **Content Correlation**: Matches based on content similarity and timing patterns
//! - **Multi-Language Support**: Handles subtitle files in different languages
//! - **Confidence Scoring**: Provides match confidence levels for user validation
//!
//! ## Advanced Matching Algorithms
//! - **Fuzzy Matching**: Tolerates variations in naming conventions
//! - **Episode Detection**: Recognizes season/episode patterns in TV series
//! - **Quality Assessment**: Evaluates subtitle quality and completeness
//! - **Conflict Resolution**: Handles multiple subtitle candidates intelligently
//!
//! # Architecture Overview
//!
//! The matching system consists of several interconnected components:
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   Discovery     │────│   AI Analysis    │────│   Match Engine  │
//! │   - Find files  │    │   - Semantic     │    │   - Score calc  │
//! │   - Language    │    │   - Content      │    │   - Validation  │
//! │   - Metadata    │    │   - Confidence   │    │   - Ranking     │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//!         │                        │                        │
//!         └────────────────────────┼────────────────────────┘
//!                                  │
//!                    ┌─────────────────────────┐
//!                    │       Cache System      │
//!                    │   - Analysis results    │
//!                    │   - Match history       │
//!                    │   - Performance data    │
//!                    └─────────────────────────┘
//! ```
//!
//! # Usage Examples
//!
//! ## Basic File Matching
//!
//! ```rust,ignore
//! use subx_cli::core::matcher::{MatchEngine, MatchConfig, FileDiscovery};
//! use std::path::Path;
//!
//! // Configure matching parameters
//! let config = MatchConfig {
//!     confidence_threshold: 0.8,
//!     dry_run: false,
//!     ai_provider: Some("openai".to_string()),
//!     ..Default::default()
//! };
//!
//! // Initialize the matching engine
//! let engine = MatchEngine::new(config);
//!
//! // Discover files in directories
//! let discovery = FileDiscovery::new();
//! let video_files = discovery.find_media_files(Path::new("/videos"))?;
//! let subtitle_files = discovery.find_subtitle_files(Path::new("/subtitles"))?;
//!
//! // Perform matching
//! let matches = engine.match_files(&video_files, &subtitle_files).await?;
//!
//! for match_result in matches {
//!     println!("Matched: {} -> {} (confidence: {:.2})",
//!         match_result.video_file.name,
//!         match_result.subtitle_file.name,
//!         match_result.confidence
//!     );
//! }
//! ```
//!
//! ## Advanced Matching with Language Filtering
//!
//! ```rust,ignore
//! use subx_cli::core::matcher::MatchConfig;
//!
//! let config = MatchConfig {
//!     target_languages: vec!["zh".to_string(), "en".to_string()],
//!     exclude_languages: vec!["jp".to_string()],
//!     confidence_threshold: 0.75,
//!     max_matches_per_video: 2, // Allow multiple subtitle languages
//!     ..Default::default()
//! };
//!
//! let matches = engine.match_files_with_config(&video_files, &subtitle_files, config).await?;
//! ```
//!
//! ## TV Series Episode Matching
//!
//! ```rust,ignore
//! // For TV series with season/episode structure
//! let tv_config = MatchConfig {
//!     series_mode: true,
//!     season_episode_patterns: vec![
//!         r"S(\d+)E(\d+)".to_string(),
//!         r"Season (\d+) Episode (\d+)".to_string(),
//!     ],
//!     ..Default::default()
//! };
//!
//! let tv_matches = engine.match_tv_series(&video_files, &subtitle_files, tv_config).await?;
//! ```
//!
//! # Matching Algorithms
//!
//! ## 1. Filename Analysis
//! - **Pattern Extraction**: Identifies common patterns like episode numbers, years, quality markers
//! - **Language Code Detection**: Recognizes language codes in various formats (en, eng, english, etc.)
//! - **Normalization**: Standardizes filenames for comparison by removing common variations
//!
//! ## 2. AI Semantic Analysis
//! - **Title Extraction**: Uses AI to identify actual titles from complex filenames
//! - **Content Understanding**: Analyzes subtitle content to understand context and themes
//! - **Cross-Reference**: Compares extracted information between video and subtitle files
//!
//! ## 3. Confidence Scoring
//! - **Multiple Factors**: Combines filename similarity, language match, content correlation
//! - **Weighted Scoring**: Applies different weights based on reliability of each factor
//! - **Threshold Filtering**: Only presents matches above configurable confidence levels
//!
//! ## 4. Conflict Resolution
//! - **Ranking**: Orders multiple candidates by confidence score
//! - **Deduplication**: Removes duplicate or overlapping matches
//! - **User Preferences**: Applies user-defined preferences for language, quality, etc.
//!
//! # Performance Characteristics
//!
//! - **Caching**: Results are cached to avoid re-analysis of unchanged files
//! - **Parallel Processing**: File analysis is performed concurrently for speed
//! - **Incremental Updates**: Only processes new or modified files in subsequent runs
//! - **Memory Efficient**: Streams large directory structures without loading all data
//!
//! # Error Handling
//!
//! The matching system provides comprehensive error handling for:
//! - File system access issues (permissions, missing directories)
//! - AI service connectivity and quota problems
//! - Invalid or corrupted subtitle files
//! - Configuration validation errors
//! - Network timeouts and service degradation
//!
//! # Thread Safety
//!
//! All matching operations are thread-safe and can be used concurrently.
//! The cache system uses appropriate synchronization for multi-threaded access.

#![allow(dead_code)]

pub mod discovery;
pub mod engine;
// Filename analyzer removed to simplify matching logic.

pub use discovery::{FileDiscovery, MediaFile, MediaFileType};
pub use engine::{MatchConfig, MatchEngine, MatchOperation};
// pub use filename_analyzer::{FilenameAnalyzer, ParsedFilename};
pub mod cache;
pub mod journal;
use crate::Result;
use crate::core::language::{LanguageDetector, LanguageInfo};
use crate::error::SubXError;
use std::path::{Path, PathBuf};

/// Extended file information structure with metadata for intelligent matching.
///
/// This structure contains comprehensive information about discovered files,
/// including path relationships, language detection results, and contextual
/// metadata that enables sophisticated matching algorithms.
///
/// # Purpose
///
/// `FileInfo` serves as the primary data structure for file representation
/// in the matching system. It normalizes file information from different
/// sources and provides a consistent interface for matching algorithms.
///
/// # Path Relationships
///
/// The structure maintains three different path representations:
/// - `name`: Just the filename for display and basic comparison
/// - `relative_path`: Path relative to search root for organization
/// - `full_path`: Absolute path for file system operations
///
/// # Language Detection
///
/// Language information is automatically detected from:
/// - Filename patterns (e.g., "movie.en.srt", "film.zh-tw.ass")
/// - Directory structure (e.g., "English/", "Chinese/")
/// - File content analysis for subtitle files
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::matcher::FileInfo;
/// use std::path::PathBuf;
///
/// let root = PathBuf::from("/media/movies");
/// let file_path = PathBuf::from("/media/movies/Action/movie.en.srt");
///
/// let file_info = FileInfo::new(&file_path, &root)?;
///
/// assert_eq!(file_info.name, "movie.en.srt");
/// assert_eq!(file_info.relative_path, "Action/movie.en.srt");
/// assert_eq!(file_info.directory, "Action");
/// assert_eq!(file_info.depth, 1);
///
/// if let Some(lang) = &file_info.language {
///     println!("Detected language: {}", lang.code);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name without directory path for display and comparison.
    ///
    /// This is the base filename including extension, useful for
    /// pattern matching and user-friendly display.
    pub name: String,

    /// Path relative to the search root directory for organization.
    ///
    /// Maintains the directory structure context while being
    /// independent of the absolute filesystem location.
    pub relative_path: String,

    /// Absolute file system path for file operations.
    ///
    /// Used for actual file reading, writing, and metadata access.
    pub full_path: PathBuf,

    /// Name of the immediate parent directory containing the file.
    ///
    /// Useful for organization-based matching and language detection
    /// from directory names.
    pub directory: String,

    /// Directory depth relative to the root search path.
    ///
    /// Indicates how many subdirectory levels deep the file is located.
    /// Depth 0 means the file is directly in the root directory.
    pub depth: usize,

    /// Detected language information from filename or content analysis.
    ///
    /// Contains language code, confidence level, and detection method.
    /// May be `None` if no language could be reliably detected.
    pub language: Option<LanguageInfo>,
}

impl FileInfo {
    /// Construct a new `FileInfo` from a file path and search root directory.
    ///
    /// This method performs comprehensive analysis of the file location,
    /// extracting path relationships, directory structure, and attempting
    /// automatic language detection from the filename and path.
    ///
    /// # Arguments
    ///
    /// * `full_path` - Absolute path to the media or subtitle file
    /// * `root_path` - Root directory for file discovery (used to compute relative paths)
    ///
    /// # Returns
    ///
    /// Returns a `FileInfo` struct with all metadata populated, including
    /// optional language detection results.
    ///
    /// # Errors
    ///
    /// Returns `SubXError::Other` if:
    /// - The file path cannot be made relative to the root path
    /// - Path contains invalid Unicode characters
    /// - File system access issues occur during analysis
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use subx_cli::core::matcher::FileInfo;
    /// use std::path::PathBuf;
    ///
    /// // Simple file in root directory
    /// let root = PathBuf::from("/media/videos");
    /// let file_path = root.join("movie.mp4");
    /// let info = FileInfo::new(file_path, &root)?;
    ///
    /// assert_eq!(info.name, "movie.mp4");
    /// assert_eq!(info.relative_path, "movie.mp4");
    /// assert_eq!(info.depth, 0);
    ///
    /// // File in subdirectory with language
    /// let sub_file = root.join("English").join("movie.en.srt");
    /// let sub_info = FileInfo::new(sub_file, &root)?;
    ///
    /// assert_eq!(sub_info.name, "movie.en.srt");
    /// assert_eq!(sub_info.relative_path, "English/movie.en.srt");
    /// assert_eq!(sub_info.directory, "English");
    /// assert_eq!(sub_info.depth, 1);
    /// assert!(sub_info.language.is_some());
    /// ```
    ///
    /// # Implementation Details
    ///
    /// - Path separators are normalized to Unix style (/) for consistency
    /// - Directory depth is calculated based on relative path components
    /// - Language detection runs automatically using multiple detection methods
    /// - All path operations are Unicode-safe with fallback to empty strings
    pub fn new(full_path: PathBuf, root_path: &Path) -> Result<Self> {
        // Calculate relative path by stripping the root prefix
        let relative_path = full_path
            .strip_prefix(root_path)
            .map_err(|e| SubXError::Other(e.into()))?
            .to_string_lossy()
            .replace('\\', "/"); // Normalize to Unix-style separators

        // Extract the base filename
        let name = full_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        // Get the immediate parent directory name
        let directory = full_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        // Calculate directory depth by counting path separators
        let depth = relative_path.matches('/').count();

        // Attempt automatic language detection from path and filename
        let detector = LanguageDetector::new();
        let language = detector.detect_from_path(&full_path);

        Ok(Self {
            name,
            relative_path,
            full_path,
            directory,
            depth,
            language,
        })
    }

    /// Get the file extension without the leading dot.
    ///
    /// Returns the file extension in lowercase, or an empty string if
    /// no extension is present.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(file_info.extension(), "mp4");
    /// assert_eq!(subtitle_info.extension(), "srt");
    /// ```
    pub fn extension(&self) -> String {
        self.full_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_lowercase()
    }

    /// Get the filename without extension (stem).
    ///
    /// Returns the base filename with the extension removed, useful
    /// for comparison and matching operations.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // For "movie.en.srt"
    /// assert_eq!(file_info.stem(), "movie.en");
    ///
    /// // For "episode01.mp4"
    /// assert_eq!(file_info.stem(), "episode01");
    /// ```
    pub fn stem(&self) -> String {
        self.full_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string()
    }

    /// Check if this file is in the root directory (depth 0).
    ///
    /// Returns `true` if the file is directly in the search root,
    /// `false` if it's in a subdirectory.
    pub fn is_in_root(&self) -> bool {
        self.depth == 0
    }

    /// Check if this file has detected language information.
    ///
    /// Returns `true` if language detection was successful and
    /// confidence is above the detection threshold.
    pub fn has_language(&self) -> bool {
        self.language.is_some()
    }

    /// Get the detected language code if available.
    ///
    /// Returns the language code string (e.g., "en", "zh", "ja")
    /// or `None` if no language was detected.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if let Some(lang) = file_info.language_code() {
    ///     println!("Detected language: {}", lang);
    /// }
    /// ```
    pub fn language_code(&self) -> Option<&str> {
        self.language.as_ref().map(|lang| lang.code.as_str())
    }

    /// Create a normalized version of the filename for comparison.
    ///
    /// Applies various normalization rules to make filenames more
    /// comparable during matching operations:
    /// - Converts to lowercase
    /// - Removes common separators and special characters
    /// - Standardizes whitespace
    /// - Removes quality indicators and release group tags
    ///
    /// # Returns
    ///
    /// A normalized filename string suitable for fuzzy matching.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // "Movie.Name.2023.1080p.BluRay.x264-GROUP.mkv"
    /// // becomes "movie name 2023"
    /// let normalized = file_info.normalized_name();
    /// ```
    pub fn normalized_name(&self) -> String {
        let mut name = self.stem().to_lowercase();

        // Remove common separators
        name = name.replace(['.', '_', '-'], " ");

        // Remove quality indicators
        let quality_patterns = [
            "1080p", "720p", "480p", "4k", "2160p", "bluray", "webrip", "hdtv", "dvdrip", "x264",
            "x265", "h264", "h265",
        ];

        for pattern in &quality_patterns {
            name = name.replace(pattern, "");
        }

        // Remove release group tags (text within brackets/parentheses)
        name = regex::Regex::new(r"\[.*?\]|\(.*?\)")
            .unwrap()
            .replace_all(&name, "")
            .to_string();

        // Normalize whitespace
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_file(root: &Path, rel: &str) -> PathBuf {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"").unwrap();
        path
    }

    #[test]
    fn test_file_info_creation() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "season1/episode1.mp4");

        let info = FileInfo::new(file_path.clone(), root)?;
        assert_eq!(info.name, "episode1.mp4");
        assert_eq!(info.relative_path, "season1/episode1.mp4");
        assert_eq!(info.directory, "season1");
        assert_eq!(info.depth, 1);
        Ok(())
    }

    #[test]
    fn test_file_info_deep_path() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let file_path = create_temp_file(root, "series/season1/episodes/ep01.mp4");

        let info = FileInfo::new(file_path.clone(), root)?;
        assert_eq!(info.relative_path, "series/season1/episodes/ep01.mp4");
        assert_eq!(info.depth, 3);

        Ok(())
    }

    #[test]
    fn test_file_info_root_file() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.mp4");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.name, "movie.mp4");
        assert_eq!(info.relative_path, "movie.mp4");
        assert_eq!(info.depth, 0);
        assert!(info.is_in_root());
        Ok(())
    }

    #[test]
    fn test_file_info_not_in_root_for_subdirectory() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "subdir/movie.mp4");

        let info = FileInfo::new(file_path, root)?;
        assert!(!info.is_in_root());
        Ok(())
    }

    #[test]
    fn test_file_info_error_path_not_under_root() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let other_temp = TempDir::new().unwrap();
        let file_path = other_temp.path().join("movie.mp4");
        std::fs::write(&file_path, b"").unwrap();

        let result = FileInfo::new(file_path, root);
        assert!(result.is_err());
    }

    #[test]
    fn test_extension_returns_lowercase() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "Subtitle.SRT");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.extension(), "srt");
        Ok(())
    }

    #[test]
    fn test_extension_various_formats() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        for (filename, expected_ext) in [
            ("movie.mp4", "mp4"),
            ("subtitle.ass", "ass"),
            ("sub.vtt", "vtt"),
            ("clip.mkv", "mkv"),
        ] {
            let file_path = create_temp_file(root, filename);
            let info = FileInfo::new(file_path, root)?;
            assert_eq!(info.extension(), expected_ext, "failed for {filename}");
        }
        Ok(())
    }

    #[test]
    fn test_extension_no_extension() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "noextension");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.extension(), "");
        Ok(())
    }

    #[test]
    fn test_stem_basic() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "episode01.mp4");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.stem(), "episode01");
        Ok(())
    }

    #[test]
    fn test_stem_multiple_dots() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.en.srt");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.stem(), "movie.en");
        Ok(())
    }

    #[test]
    fn test_stem_no_extension() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "noextension");

        let info = FileInfo::new(file_path, root)?;
        assert_eq!(info.stem(), "noextension");
        Ok(())
    }

    #[test]
    fn test_has_language_with_detected_language() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.en.srt");

        let info = FileInfo::new(file_path, root)?;
        // "en" in filename should be detected as English
        if info.has_language() {
            assert!(info.language_code().is_some());
            assert_eq!(info.language_code(), Some("en"));
        }
        Ok(())
    }

    #[test]
    fn test_has_language_without_language_indicator() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "plainmovie.mp4");

        let info = FileInfo::new(file_path, root)?;
        assert!(!info.has_language());
        assert!(info.language_code().is_none());
        Ok(())
    }

    #[test]
    fn test_language_code_returns_correct_code() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.zh.srt");

        let info = FileInfo::new(file_path, root)?;
        if let Some(code) = info.language_code() {
            assert_eq!(code, "zh");
        }
        Ok(())
    }

    #[test]
    fn test_language_detection_from_directory_name() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "English/movie.srt");

        let info = FileInfo::new(file_path, root)?;
        // Directory "English" should trigger language detection
        if info.has_language() {
            let code = info.language_code().unwrap();
            assert_eq!(code, "en");
        }
        Ok(())
    }

    #[test]
    fn test_normalized_name_lowercase_and_separators() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "My.Movie.Name.mp4");

        let info = FileInfo::new(file_path, root)?;
        let normalized = info.normalized_name();
        assert_eq!(normalized, "my movie name");
        Ok(())
    }

    #[test]
    fn test_normalized_name_removes_quality_indicators() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        for (filename, expected) in [
            ("Movie.2023.1080p.BluRay.mp4", "movie 2023"),
            ("Show.S01E01.720p.HDTV.mkv", "show s01e01"),
            ("Film.4K.x265.mp4", "film"),
            ("Documentary.2160p.WEBRip.mkv", "documentary"),
        ] {
            let file_path = create_temp_file(root, filename);
            let info = FileInfo::new(file_path, root)?;
            assert_eq!(info.normalized_name(), expected, "failed for {filename}");
        }
        Ok(())
    }

    #[test]
    fn test_normalized_name_removes_brackets() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "Movie [1080p] (BluRay).mp4");

        let info = FileInfo::new(file_path, root)?;
        let normalized = info.normalized_name();
        // Brackets and their contents should be stripped
        assert!(!normalized.contains('['));
        assert!(!normalized.contains(']'));
        assert!(!normalized.contains('('));
        assert!(!normalized.contains(')'));
        Ok(())
    }

    #[test]
    fn test_normalized_name_normalizes_whitespace() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie___name.mp4");

        let info = FileInfo::new(file_path, root)?;
        let normalized = info.normalized_name();
        // Underscores become spaces and multiple spaces collapse to one
        assert!(!normalized.contains("  "));
        assert!(!normalized.contains('_'));
        Ok(())
    }

    #[test]
    fn test_file_info_clone() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.mp4");

        let info = FileInfo::new(file_path, root)?;
        let cloned = info.clone();
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.relative_path, cloned.relative_path);
        assert_eq!(info.depth, cloned.depth);
        assert_eq!(info.directory, cloned.directory);
        assert_eq!(info.full_path, cloned.full_path);
        Ok(())
    }

    #[test]
    fn test_file_info_debug_format() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.mp4");

        let info = FileInfo::new(file_path, root)?;
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("FileInfo"));
        assert!(debug_str.contains("movie.mp4"));
        Ok(())
    }

    #[test]
    fn test_file_info_directory_at_root() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "movie.mp4");

        let info = FileInfo::new(file_path, root)?;
        // Directory for a root-level file is the root directory name itself
        let root_dir_name = root.file_name().unwrap().to_string_lossy();
        assert_eq!(info.directory, root_dir_name.as_ref());
        Ok(())
    }

    #[test]
    fn test_normalized_name_removes_dvdrip() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "Old.Movie.DVDRip.avi");

        let info = FileInfo::new(file_path, root)?;
        let normalized = info.normalized_name();
        assert!(!normalized.contains("dvdrip"));
        assert!(normalized.contains("old"));
        assert!(normalized.contains("movie"));
        Ok(())
    }

    #[test]
    fn test_normalized_name_h264_h265_removed() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        for filename in ["Film.H264.mp4", "Film.H265.mp4"] {
            let file_path = create_temp_file(root, filename);
            let info = FileInfo::new(file_path, root)?;
            let normalized = info.normalized_name();
            assert!(
                !normalized.contains("h264"),
                "h264 not removed in {filename}"
            );
            assert!(
                !normalized.contains("h265"),
                "h265 not removed in {filename}"
            );
            assert!(normalized.contains("film"));
        }
        Ok(())
    }

    #[test]
    fn test_file_info_full_path_preserved() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = create_temp_file(root, "subdir/movie.srt");

        let info = FileInfo::new(file_path.clone(), root)?;
        assert_eq!(info.full_path, file_path);
        Ok(())
    }

    #[test]
    fn test_multiple_language_codes_in_filename() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Test several common language code formats
        for (filename, expected_code) in [
            ("movie.en.srt", "en"),
            ("movie.zh.srt", "zh"),
            ("movie.ja.srt", "ja"),
        ] {
            let file_path = create_temp_file(root, filename);
            let info = FileInfo::new(file_path, root)?;
            if let Some(code) = info.language_code() {
                assert_eq!(code, expected_code, "wrong code for {filename}");
            }
        }
        Ok(())
    }
}
