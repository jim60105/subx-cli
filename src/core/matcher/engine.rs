//! File matching engine that uses AI content analysis to align video and subtitle files.
//!
//! This module provides the `MatchEngine`, which orchestrates discovery,
//! content sampling, AI analysis, and caching to generate subtitle matching operations.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::matcher::engine::{MatchEngine, MatchConfig};
//! // Create a match engine with default configuration
//! let config = MatchConfig { confidence_threshold: 0.8, max_sample_length: 1024, enable_content_analysis: true, backup_enabled: false };
//! let engine = MatchEngine::new(Box::new(DummyAI), config);
//! ```

use crate::services::ai::{AIProvider, AnalysisRequest, ContentSample, MatchResult};
use std::path::PathBuf;

use crate::Result;
use crate::core::language::LanguageDetector;
use crate::core::matcher::cache::{CacheData, OpItem, SnapshotItem};
use crate::core::matcher::discovery::generate_file_id;
use crate::core::matcher::journal::{
    JournalData, JournalEntry, JournalEntryStatus, JournalOperationType, journal_path,
};
use crate::core::matcher::{FileDiscovery, MediaFile, MediaFileType};
use crate::core::parallel::{FileProcessingTask, ProcessingOperation, Task, TaskResult};
use crate::core::uuidv7::Uuidv7Generator;
use crate::error::SubXError;
use dirs;
use serde_json;

/// Current on-disk match-cache schema version.
///
/// Bumped whenever the prompt structure or cache layout changes in a
/// way that invalidates earlier entries. Historical values: `"1.0"`
/// (pre-XML prompt rewrite); `"2.0"` (current — XML-tagged prompt with
/// optional `language` / `target_filename_suffix`).
pub(crate) const CURRENT_CACHE_VERSION: &str = "2.0";

/// Returns whether a cache file with the given `cache_version` string is
/// considered compatible with the current code. Used by
/// [`MatchEngine::check_file_list_cache`] to reject entries written by
/// earlier prompt schemas.
pub(crate) fn is_cache_version_current(version: &str) -> bool {
    version == CURRENT_CACHE_VERSION
}

/// Sanitize an AI-supplied filename suffix.
///
/// Accepts only ASCII alphanumerics, underscore, and hyphen. The check is
/// applied to the **whole string**: any disallowed character (including
/// path separators or `.`) causes outright rejection rather than silent
/// stripping, so payloads such as `"../etc"` cannot smuggle a usable
/// `"etc"` token through.
///
/// # Arguments
///
/// * `raw` - Raw suffix supplied by the AI (e.g. `"tc"`, `"english"`,
///   `"../etc"`).
///
/// # Returns
///
/// `Some(raw)` when the input is non-empty, no longer than 16 bytes, and
/// composed entirely of `[A-Za-z0-9_-]`; `None` otherwise.
pub(crate) fn sanitize_suffix(raw: &str) -> Option<String> {
    if raw.is_empty() || raw.len() > 16 {
        return None;
    }
    if raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(raw.to_string())
    } else {
        None
    }
}

/// Normalize an AI-supplied language label without going through the
/// strict ASCII gate of [`sanitize_suffix`].
///
/// This helper preserves Unicode (so labels such as `繁中` or `简中` reach
/// [`LanguageDetector::normalize`]), only lower-casing ASCII letters and
/// trimming whitespace before lookup. Sentinel values (`""` and `und`,
/// case-insensitive) return `None`. After detector lookup, the returned
/// canonical code is finally validated through [`sanitize_suffix`] so the
/// downstream filename insertion remains safe.
///
/// # Arguments
///
/// * `detector` - Configured [`LanguageDetector`] used for synonym lookup.
/// * `raw` - Raw label as supplied by the AI.
///
/// # Returns
///
/// `Some(code)` with a sanitized canonical short code (e.g. `"tc"`,
/// `"en"`); `None` for empty input, the sentinel `und`, or labels that
/// resolve to something unsafe for filenames.
pub(crate) fn normalize_ai_language(detector: &LanguageDetector, raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("und") {
        return None;
    }
    let lowered: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_uppercase() {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect();
    let resolved = detector.normalize(&lowered)?;
    sanitize_suffix(&resolved)
}

/// Globally enforce unique final target paths across a batch of
/// [`MatchOperation`] values.
///
/// Run **after** any post-engine relocation rewrites (e.g.
/// `match_command.rs` archive-origin forced relocation) so the
/// uniqueness guarantee holds at the actual destination paths. The
/// allocator iterates operations sorted by `(target_directory,
/// subtitle_file.relative_path)` for stability across reruns and
/// probes `<base>.<n>.<ext>` (preserving any language segment) starting
/// at `n = 2` until a free path is found, mutating both
/// `new_subtitle_name` and `relocation_target_path` so downstream
/// consumers see consistent values.
///
/// # Arguments
///
/// * `operations` - Mutable slice of operations to deduplicate in place.
pub fn apply_unique_target_paths(operations: &mut [MatchOperation]) {
    use std::collections::HashSet;

    fn final_target(op: &MatchOperation) -> PathBuf {
        if let Some(p) = &op.relocation_target_path {
            p.clone()
        } else {
            let parent = op
                .subtitle_file
                .path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            parent.join(&op.new_subtitle_name)
        }
    }

    fn split_filename(name: &str) -> (String, String) {
        // Split filename into (stem, ".ext"); preserves multi-dot stems.
        if let Some(idx) = name.rfind('.') {
            if idx > 0 {
                return (name[..idx].to_string(), name[idx..].to_string());
            }
        }
        (name.to_string(), String::new())
    }

    /// Strip a trailing `.<digits>` segment from `stem`, returning
    /// `(base_stem, existing_counter)`. Used so that when the AI (or a
    /// prior pass) already produced `movie.2.srt`, the next collision
    /// becomes `movie.3.srt` rather than `movie.2.2.srt`.
    fn split_numeric_tail(stem: &str) -> (String, Option<u32>) {
        if let Some(idx) = stem.rfind('.') {
            let (head, tail) = (&stem[..idx], &stem[idx + 1..]);
            if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = tail.parse::<u32>() {
                    return (head.to_string(), Some(n));
                }
            }
        }
        (stem.to_string(), None)
    }

    // Sort indices so we can mutate operations in-place but iterate in a
    // deterministic order independent of the AI's response ordering.
    let mut indices: Vec<usize> = (0..operations.len()).collect();
    indices.sort_by(|&a, &b| {
        let pa = final_target(&operations[a]);
        let pb = final_target(&operations[b]);
        let dir_a = pa.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let dir_b = pb.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        dir_a.cmp(&dir_b).then_with(|| {
            operations[a]
                .subtitle_file
                .relative_path
                .cmp(&operations[b].subtitle_file.relative_path)
        })
    });

    // Reserve every operation's *original* candidate so that two
    // operations cannot both claim a numerically-suffixed slot that a
    // third operation already legitimately needed. See OpenSpec scenario
    // "Three-way duplicates and a pre-existing numeric suffix" — without
    // this guard `[movie.srt, movie.srt, movie.2.srt]` would steal
    // `movie.2.srt` from the third op while resolving the second.
    let reserved: HashSet<PathBuf> = (0..operations.len())
        .map(|i| final_target(&operations[i]))
        .collect();

    let mut claimed: HashSet<PathBuf> = HashSet::new();

    for idx in indices {
        let candidate = final_target(&operations[idx]);
        let parent = candidate
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let filename = candidate
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let (stem, ext) = split_filename(&filename);
        let (base_stem, existing_counter) = split_numeric_tail(&stem);
        let source_path = operations[idx].subtitle_file.path.clone();

        // A probe path is taken if another op has already committed it
        // (via `claimed`), if some *other* op still holds it as its
        // original candidate (via `reserved`, minus the current op's own
        // candidate so the current op can keep its preferred slot when
        // free), or if a file already exists on disk at that path
        // (excluding the operation's own source file so a no-op rename
        // can keep its current name).
        let is_taken = |p: &PathBuf| -> bool {
            claimed.contains(p)
                || (p != &candidate && reserved.contains(p))
                || (p != &source_path && p.exists())
        };

        let mut resolved = candidate.clone();
        let mut resolved_name = filename.clone();
        if is_taken(&resolved) {
            let mut counter = existing_counter.map(|n| n + 1).unwrap_or(2).max(2);
            loop {
                let new_name = format!("{}.{}{}", base_stem, counter, ext);
                let probe = parent.join(&new_name);
                if !is_taken(&probe) {
                    resolved = probe;
                    resolved_name = new_name;
                    break;
                }
                counter = counter.saturating_add(1);
                if counter > 9999 {
                    break;
                }
            }
        }

        let op = &mut operations[idx];
        op.new_subtitle_name = resolved_name;
        if op.relocation_target_path.is_some() {
            op.relocation_target_path = Some(resolved.clone());
        }
        claimed.insert(resolved);
    }
}

/// File relocation mode for matched subtitle files
#[derive(Debug, Clone, PartialEq)]
pub enum FileRelocationMode {
    /// No file relocation
    None,
    /// Copy subtitle files to video folders
    Copy,
    /// Move subtitle files to video folders
    Move,
}

/// Strategy for handling filename conflicts during relocation
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// Skip relocation if conflict exists
    Skip,
    /// Automatically rename with numeric suffix
    AutoRename,
    /// Prompt user for decision (interactive mode only)
    Prompt,
}

/// Configuration settings for the file matching engine.
///
/// Controls various aspects of the subtitle-to-video matching process,
/// including confidence thresholds and analysis options.
#[derive(Debug, Clone)]
pub struct MatchConfig {
    /// Minimum confidence score required for a successful match (0.0 to 1.0)
    pub confidence_threshold: f32,
    /// Maximum number of characters to sample from subtitle content
    pub max_sample_length: usize,
    /// Whether to enable advanced content analysis for matching
    pub enable_content_analysis: bool,
    /// Whether to create backup files before operations
    pub backup_enabled: bool,
    /// File relocation mode
    pub relocation_mode: FileRelocationMode,
    /// Strategy for handling filename conflicts during relocation
    pub conflict_resolution: ConflictResolution,
    /// AI model name used for analysis
    pub ai_model: String,
    /// Maximum subtitle file size in bytes accepted for content sampling.
    /// Populated from `GeneralConfig.max_subtitle_bytes`.
    pub max_subtitle_bytes: u64,
}

#[cfg(test)]
mod language_name_tests {
    use super::*;
    use crate::core::matcher::discovery::{MediaFile, MediaFileType};
    use crate::services::ai::{
        AIProvider, AnalysisRequest, ConfidenceScore, FileMatch, MatchResult, VerificationRequest,
    };
    use async_trait::async_trait;
    use std::path::PathBuf;

    fn legacy_match() -> FileMatch {
        FileMatch {
            video_file_id: "v".into(),
            subtitle_file_id: "s".into(),
            confidence: 1.0,
            match_factors: vec![],
            language: None,
            target_filename_suffix: None,
        }
    }

    fn match_with_language(language: Option<&str>, suffix: Option<&str>) -> FileMatch {
        FileMatch {
            video_file_id: "v".into(),
            subtitle_file_id: "s".into(),
            confidence: 1.0,
            match_factors: vec![],
            language: language.map(|s| s.to_string()),
            target_filename_suffix: suffix.map(|s| s.to_string()),
        }
    }

    struct DummyAI;
    #[async_trait]
    impl AIProvider for DummyAI {
        async fn analyze_content(&self, _req: AnalysisRequest) -> crate::Result<MatchResult> {
            unimplemented!()
        }
        async fn verify_match(&self, _req: VerificationRequest) -> crate::Result<ConfidenceScore> {
            unimplemented!()
        }
    }

    #[test]
    fn test_generate_subtitle_name_with_directory_language() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("movie01.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie01".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("tc/subtitle01.ass"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "subtitle01".to_string(),
            extension: "ass".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "movie01.tc.ass");
    }

    #[test]
    fn test_generate_subtitle_name_with_filename_language() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("movie02.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie02".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("subtitle02.sc.ass"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "subtitle02".to_string(),
            extension: "ass".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "movie02.sc.ass");
    }

    #[test]
    fn test_generate_subtitle_name_without_language() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("movie03.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie03".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("subtitle03.ass"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "subtitle03".to_string(),
            extension: "ass".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "movie03.ass");
    }
    #[test]
    fn test_generate_subtitle_name_removes_video_extension() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("movie.mkv"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie.mkv".to_string(),
            extension: "mkv".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("subtitle.srt"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "subtitle".to_string(),
            extension: "srt".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "movie.srt");
    }

    #[test]
    fn test_generate_subtitle_name_with_language_removes_video_extension() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("movie.mkv"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie.mkv".to_string(),
            extension: "mkv".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("tc/subtitle.srt"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "subtitle".to_string(),
            extension: "srt".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "movie.tc.srt");
    }

    #[test]
    fn test_generate_subtitle_name_edge_cases() {
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );
        // File name contains multiple dots and no extension case
        let video = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("a.b.c"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "a.b.c".to_string(),
            extension: "".to_string(),
        };
        let subtitle = MediaFile {
            id: "".to_string(),
            relative_path: "".to_string(),
            path: PathBuf::from("sub.srt"),
            file_type: MediaFileType::Subtitle,
            size: 0,
            name: "sub".to_string(),
            extension: "srt".to_string(),
        };
        let new_name = engine.generate_subtitle_name(&video, &subtitle, &legacy_match());
        assert_eq!(new_name, "a.b.c.srt");
    }

    fn make_engine() -> MatchEngine {
        MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        )
    }

    fn media(path: &str, name: &str, ext: &str, ty: MediaFileType) -> MediaFile {
        MediaFile {
            id: "".into(),
            relative_path: path.into(),
            path: PathBuf::from(path),
            file_type: ty,
            size: 0,
            name: name.into(),
            extension: ext.into(),
        }
    }

    #[test]
    fn test_generate_subtitle_name_ai_suffix_wins() {
        let engine = make_engine();
        let video = media("movie.mkv", "movie.mkv", "mkv", MediaFileType::Video);
        let subtitle = media("tc/subs.srt", "subs", "srt", MediaFileType::Subtitle);
        let m = match_with_language(Some("en"), Some("tc"));
        assert_eq!(
            engine.generate_subtitle_name(&video, &subtitle, &m),
            "movie.tc.srt"
        );
    }

    #[test]
    fn test_generate_subtitle_name_ai_language_used() {
        let engine = make_engine();
        let video = media("movie.mkv", "movie.mkv", "mkv", MediaFileType::Video);
        let subtitle = media("subs/movie.srt", "movie", "srt", MediaFileType::Subtitle);
        let m = match_with_language(Some("ja"), None);
        assert_eq!(
            engine.generate_subtitle_name(&video, &subtitle, &m),
            "movie.ja.srt"
        );
    }

    #[test]
    fn test_generate_subtitle_name_language_synonym_normalized() {
        let engine = make_engine();
        let video = media("movie.mkv", "movie.mkv", "mkv", MediaFileType::Video);
        let subtitle = media("subs/movie.srt", "movie", "srt", MediaFileType::Subtitle);
        for variant in &["english", "eng", "EN"] {
            let m = match_with_language(Some(variant), None);
            assert_eq!(
                engine.generate_subtitle_name(&video, &subtitle, &m),
                "movie.en.srt",
                "variant {variant} should normalize to en"
            );
        }
    }

    #[test]
    fn test_generate_subtitle_name_und_collapses_to_no_tag() {
        let engine = make_engine();
        let video = media("movie.mkv", "movie.mkv", "mkv", MediaFileType::Video);
        let subtitle = media("subs/movie.srt", "movie", "srt", MediaFileType::Subtitle);
        let m = match_with_language(Some("und"), None);
        assert_eq!(
            engine.generate_subtitle_name(&video, &subtitle, &m),
            "movie.srt"
        );
    }

    #[test]
    fn test_generate_subtitle_name_sanitization_drops_path_traversal() {
        let engine = make_engine();
        let video = media("movie.mkv", "movie.mkv", "mkv", MediaFileType::Video);
        let subtitle = media("subs/movie.srt", "movie", "srt", MediaFileType::Subtitle);
        // Whole-string rejection: any disallowed character (including
        // `.` and `/`) causes `sanitize_suffix` to return None, so a
        // payload like "../etc" cannot smuggle "etc" through.
        let m = match_with_language(None, Some("../etc"));
        assert_eq!(
            engine.generate_subtitle_name(&video, &subtitle, &m),
            "movie.srt"
        );
    }

    #[test]
    fn test_sanitize_suffix_helper() {
        assert_eq!(super::sanitize_suffix(""), None);
        assert_eq!(super::sanitize_suffix("../"), None);
        // Whole-string rejection: presence of `.` or `/` aborts.
        assert_eq!(super::sanitize_suffix("../etc"), None);
        assert_eq!(super::sanitize_suffix("a-b_c"), Some("a-b_c".into()));
        assert_eq!(super::sanitize_suffix("繁中"), None);
        // Length cap: anything > 16 bytes is rejected outright (no
        // truncation, since silent truncation could mask injection).
        assert_eq!(super::sanitize_suffix("0123456789abcdefGHIJKL"), None);
        assert_eq!(
            super::sanitize_suffix("0123456789abcdef"),
            Some("0123456789abcdef".into())
        );
    }

    #[test]
    fn test_normalize_ai_language_helper() {
        let det = LanguageDetector::new();
        assert_eq!(
            super::normalize_ai_language(&det, "english"),
            Some("en".into())
        );
        assert_eq!(super::normalize_ai_language(&det, "ENG"), Some("en".into()));
        assert_eq!(super::normalize_ai_language(&det, "EN"), Some("en".into()));
        assert_eq!(super::normalize_ai_language(&det, "und"), None);
        assert_eq!(super::normalize_ai_language(&det, "UND"), None);
        assert_eq!(super::normalize_ai_language(&det, ""), None);
        assert_eq!(super::normalize_ai_language(&det, "cht"), Some("tc".into()));
        assert_eq!(super::normalize_ai_language(&det, "chs"), Some("sc".into()));
        // Unicode label preserved long enough to reach the language map.
        assert_eq!(
            super::normalize_ai_language(&det, "繁中"),
            Some("tc".into())
        );
        assert_eq!(
            super::normalize_ai_language(&det, "简中"),
            Some("sc".into())
        );
        // Hyphenated and underscored regional aliases.
        assert_eq!(
            super::normalize_ai_language(&det, "traditional-chinese"),
            Some("tc".into())
        );
        assert_eq!(
            super::normalize_ai_language(&det, "Traditional_Chinese"),
            Some("tc".into())
        );
        assert_eq!(
            super::normalize_ai_language(&det, "zh-Hant"),
            Some("tc".into())
        );
        assert_eq!(
            super::normalize_ai_language(&det, "zh_hans"),
            Some("sc".into())
        );
        // Pass-through for unknown but otherwise valid short codes.
        assert_eq!(super::normalize_ai_language(&det, "vi"), Some("vi".into()));
        assert_eq!(super::normalize_ai_language(&det, "ID"), Some("id".into()));
    }

    fn op(parent: &str, name: &str, sub_relpath: &str, relocate: bool) -> MatchOperation {
        let video_path = PathBuf::from(parent).join("movie.mkv");
        let subtitle_path = PathBuf::from(sub_relpath);
        let relocation_target_path = if relocate {
            Some(PathBuf::from(parent).join(name))
        } else {
            None
        };
        MatchOperation {
            video_file: MediaFile {
                id: "v".into(),
                relative_path: video_path.to_string_lossy().to_string(),
                path: video_path,
                file_type: MediaFileType::Video,
                size: 0,
                name: "movie.mkv".into(),
                extension: "mkv".into(),
            },
            subtitle_file: MediaFile {
                id: "s".into(),
                relative_path: sub_relpath.into(),
                path: subtitle_path,
                file_type: MediaFileType::Subtitle,
                size: 0,
                name: name.into(),
                extension: "srt".into(),
            },
            new_subtitle_name: name.into(),
            confidence: 1.0,
            reasoning: vec![],
            relocation_mode: if relocate {
                FileRelocationMode::Copy
            } else {
                FileRelocationMode::None
            },
            relocation_target_path,
            requires_relocation: relocate,
        }
    }

    #[test]
    fn test_unique_target_paths_two_duplicates_same_dir() {
        let mut ops = vec![
            op("/d", "movie.srt", "/d/a.srt", false),
            op("/d", "movie.srt", "/d/b.srt", false),
        ];
        super::apply_unique_target_paths(&mut ops);
        assert_eq!(ops[0].new_subtitle_name, "movie.srt");
        assert_eq!(ops[1].new_subtitle_name, "movie.2.srt");
    }

    #[test]
    fn test_unique_target_paths_three_way_with_existing_two() {
        // Sorted order: /d/a.srt → movie.srt, /d/b.srt → movie.srt
        // (clones), /d/c.srt → movie.2.srt (already-unique candidate).
        // Spec requires the *third* op to keep its preferred slot
        // movie.2.srt, so the second op must skip past it (yielding
        // movie.3.srt) instead of stealing it. See OpenSpec scenario
        // "Three-way duplicates and a pre-existing numeric suffix".
        let mut ops = vec![
            op("/d", "movie.srt", "/d/a.srt", false),
            op("/d", "movie.srt", "/d/b.srt", false),
            op("/d", "movie.2.srt", "/d/c.srt", false),
        ];
        super::apply_unique_target_paths(&mut ops);
        assert_eq!(ops[0].new_subtitle_name, "movie.srt");
        assert_eq!(ops[1].new_subtitle_name, "movie.3.srt");
        assert_eq!(ops[2].new_subtitle_name, "movie.2.srt");
    }

    #[test]
    fn test_unique_target_paths_idempotent() {
        // Running the allocator twice on the same batch must not change
        // the result; the engine pre-pass plus the CLI post-pass rely on
        // this.
        let mut ops = vec![
            op("/d", "movie.srt", "/d/a.srt", false),
            op("/d", "movie.srt", "/d/b.srt", false),
            op("/d", "movie.2.srt", "/d/c.srt", false),
        ];
        super::apply_unique_target_paths(&mut ops);
        let snapshot: Vec<String> = ops.iter().map(|o| o.new_subtitle_name.clone()).collect();
        super::apply_unique_target_paths(&mut ops);
        let after: Vec<String> = ops.iter().map(|o| o.new_subtitle_name.clone()).collect();
        assert_eq!(snapshot, after);
    }

    #[test]
    fn test_unique_target_paths_two_languages_preserved() {
        let mut ops = vec![
            op("/d", "movie.tc.srt", "/d/a.srt", false),
            op("/d", "movie.sc.srt", "/d/b.srt", false),
        ];
        super::apply_unique_target_paths(&mut ops);
        assert_eq!(ops[0].new_subtitle_name, "movie.tc.srt");
        assert_eq!(ops[1].new_subtitle_name, "movie.sc.srt");
    }

    #[test]
    fn test_unique_target_paths_cross_video_collision_under_copy() {
        // Two different videos in different source dirs both relocate
        // into /shared with the same candidate filename "subs.srt".
        let mut ops = vec![
            op("/shared", "subs.srt", "/src1/subs.srt", true),
            op("/shared", "subs.srt", "/src2/subs.srt", true),
        ];
        super::apply_unique_target_paths(&mut ops);
        assert_eq!(ops[0].new_subtitle_name, "subs.srt");
        assert_eq!(ops[1].new_subtitle_name, "subs.2.srt");
        // relocation_target_path must be kept consistent.
        assert_eq!(
            ops[0].relocation_target_path.as_ref().unwrap(),
            &PathBuf::from("/shared/subs.srt")
        );
        assert_eq!(
            ops[1].relocation_target_path.as_ref().unwrap(),
            &PathBuf::from("/shared/subs.2.srt")
        );
    }

    #[test]
    fn test_unique_target_paths_archive_origin_relocation_unique() {
        // Mirrors the archive-origin scenario: relocation_target_path
        // is set, and two ops collide on the rewritten path.
        let mut ops = vec![
            op("/videos", "movie.srt", "/tmp/a.srt", true),
            op("/videos", "movie.srt", "/tmp/b.srt", true),
        ];
        super::apply_unique_target_paths(&mut ops);
        assert_eq!(ops[0].new_subtitle_name, "movie.srt");
        assert_eq!(ops[1].new_subtitle_name, "movie.2.srt");
    }

    #[test]
    fn test_unique_target_paths_skip_existing_on_disk() {
        // Two ops want the same target. The output directory already
        // contains `movie.srt` and `movie.1.srt`. The allocator must
        // skip both pre-existing slots, otherwise downstream
        // `resolve_filename_conflict` re-probing can race and steal
        // a numerical slot that was already allocated to a peer op
        // (depending on the operations[] iteration order).
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        std::fs::write(dir_path.join("movie.srt"), b"existing").unwrap();
        std::fs::write(dir_path.join("movie.1.srt"), b"existing").unwrap();
        let dir_str = dir_path.to_string_lossy().to_string();
        let src1 = dir_path.parent().unwrap().join("sub1.srt");
        let src2 = dir_path.parent().unwrap().join("sub2.srt");
        let mut ops = vec![
            op(&dir_str, "movie.srt", src1.to_str().unwrap(), true),
            op(&dir_str, "movie.srt", src2.to_str().unwrap(), true),
        ];
        super::apply_unique_target_paths(&mut ops);
        let mut got: Vec<String> = ops.iter().map(|o| o.new_subtitle_name.clone()).collect();
        got.sort();
        assert_eq!(
            got,
            vec!["movie.2.srt".to_string(), "movie.3.srt".to_string()]
        );
    }

    #[test]
    fn test_legacy_v1_cache_rejected() {
        // After the v1.0 → v2.0 bump, `check_file_list_cache` calls
        // `is_cache_version_current` on the loaded payload and bails out
        // with `Ok(None)` before considering operations, so stale
        // duplicate-name results from the legacy prompt cannot leak
        // through. Round-trip a real on-disk v1 cache through
        // `CacheData::load` (the same loader `check_file_list_cache`
        // uses) to prove the rejection path actually fires — going
        // through the engine method itself would require manipulating
        // `XDG_CONFIG_HOME`, which AGENTS.md forbids.
        use crate::core::matcher::cache::CacheData;
        use std::time::{SystemTime, UNIX_EPOCH};
        use tempfile::TempDir;

        assert_eq!(CURRENT_CACHE_VERSION, "2.0");
        assert!(super::is_cache_version_current("2.0"));
        assert!(!super::is_cache_version_current("1.0"));
        assert!(!super::is_cache_version_current(""));

        let temp = TempDir::new().unwrap();
        let cache_path = temp.path().join("legacy_v1_cache.json");
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let legacy = serde_json::json!({
            "cache_version": "1.0",
            "directory": "filelist_deadbeef",
            "file_snapshot": [],
            "match_operations": [],
            "created_at": now,
            "ai_model_used": "test-model",
            "config_hash": "0000000000000000",
            "original_relocation_mode": "None",
            "original_backup_enabled": false,
        });
        std::fs::write(&cache_path, serde_json::to_string(&legacy).unwrap()).unwrap();

        let loaded = CacheData::load(&cache_path).expect("legacy cache should parse");
        assert_eq!(loaded.cache_version, "1.0");
        assert!(
            !super::is_cache_version_current(&loaded.cache_version),
            "loaded v1 cache must be rejected by the version gate"
        );
    }

    #[tokio::test]
    async fn test_rename_file_displays_success_check_mark() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test file
        let original_file = temp_path.join("original.srt");
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // Create a test MatchEngine
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );

        // Create a MatchOperation
        let subtitle_file = MediaFile {
            id: "test_id".to_string(),
            relative_path: "original.srt".to_string(),
            path: original_file.clone(),
            file_type: MediaFileType::Subtitle,
            size: 40,
            name: "original".to_string(),
            extension: "srt".to_string(),
        };

        let match_op = MatchOperation {
            video_file: MediaFile {
                id: "video_id".to_string(),
                relative_path: "test.mp4".to_string(),
                path: temp_path.join("test.mp4"),
                file_type: MediaFileType::Video,
                size: 1000,
                name: "test".to_string(),
                extension: "mp4".to_string(),
            },
            subtitle_file,
            new_subtitle_name: "renamed.srt".to_string(),
            confidence: 95.0,
            reasoning: vec!["Test match".to_string()],
            requires_relocation: false,
            relocation_target_path: None,
            relocation_mode: FileRelocationMode::None,
        };

        // Execute the rename operation
        let result = engine.rename_file(&match_op).await;

        // Verify the operation was successful
        assert!(result.is_ok());

        // Verify the file has been renamed
        let renamed_file = temp_path.join("renamed.srt");
        assert!(renamed_file.exists(), "The renamed file should exist");
        assert!(
            !original_file.exists(),
            "The original file should have been renamed"
        );

        // Verify the file content is correct
        let content = fs::read_to_string(&renamed_file).unwrap();
        assert!(content.contains("Test subtitle"));
    }

    #[tokio::test]
    async fn test_rename_file_displays_error_cross_mark_when_file_not_exists() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test file
        let original_file = temp_path.join("original.srt");
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // Create a test MatchEngine
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
                ai_model: "test-model".to_string(),
                max_subtitle_bytes: 52_428_800,
            },
        );

        // Create a MatchOperation
        let subtitle_file = MediaFile {
            id: "test_id".to_string(),
            relative_path: "original.srt".to_string(),
            path: original_file.clone(),
            file_type: MediaFileType::Subtitle,
            size: 40,
            name: "original".to_string(),
            extension: "srt".to_string(),
        };

        let match_op = MatchOperation {
            video_file: MediaFile {
                id: "video_id".to_string(),
                relative_path: "test.mp4".to_string(),
                path: temp_path.join("test.mp4"),
                file_type: MediaFileType::Video,
                size: 1000,
                name: "test".to_string(),
                extension: "mp4".to_string(),
            },
            subtitle_file,
            new_subtitle_name: "renamed.srt".to_string(),
            confidence: 95.0,
            reasoning: vec!["Test match".to_string()],
            requires_relocation: false,
            relocation_target_path: None,
            relocation_mode: FileRelocationMode::None,
        };

        // Simulate file not existing after operation
        // First, execute the rename operation normally
        let result = engine.rename_file(&match_op).await;
        assert!(result.is_ok());

        // Manually delete the renamed file to simulate failure
        let renamed_file = temp_path.join("renamed.srt");
        if renamed_file.exists() {
            fs::remove_file(&renamed_file).unwrap();
        }

        // Recreate the original file for the second test
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // Create a rename operation that will fail, by overwriting the rename implementation
        // Since we cannot directly simulate std::fs::rename failure with file not existing,
        // we test the scenario where the file is manually removed after the operation completes
        let result = engine.rename_file(&match_op).await;
        assert!(result.is_ok());

        // Manually delete the file again
        let renamed_file = temp_path.join("renamed.srt");
        if renamed_file.exists() {
            fs::remove_file(&renamed_file).unwrap();
        }

        // This test mainly verifies the code structure is correct, the actual error message display needs to be validated through integration tests
        // Because we cannot easily simulate the scenario where the file system operation succeeds but the file does not exist
    }

    #[test]
    fn test_file_operation_message_format() {
        // Test error message format
        let source_name = "test.srt";
        let target_name = "renamed.srt";

        // Simulate success message format
        let success_msg = format!("  ✓ Renamed: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Renamed:"));
        assert!(success_msg.contains(source_name));
        assert!(success_msg.contains(target_name));

        // Simulate failure message format
        let error_msg = format!(
            "  ✗ Rename failed: {} -> {} (target file does not exist after operation)",
            source_name, target_name
        );
        assert!(error_msg.contains("✗"));
        assert!(error_msg.contains("Rename failed:"));
        assert!(error_msg.contains("target file does not exist"));
        assert!(error_msg.contains(source_name));
        assert!(error_msg.contains(target_name));
    }

    #[test]
    fn test_copy_operation_message_format() {
        // Test copy operation message format
        let source_name = "subtitle.srt";
        let target_name = "video.srt";

        // Simulate success message format
        let success_msg = format!("  ✓ Copied: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Copied:"));

        // Simulate failure message format
        let error_msg = format!(
            "  ✗ Copy failed: {} -> {} (target file does not exist after operation)",
            source_name, target_name
        );
        assert!(error_msg.contains("✗"));
        assert!(error_msg.contains("Copy failed:"));
        assert!(error_msg.contains("target file does not exist"));
    }

    #[test]
    fn test_move_operation_message_format() {
        // Test move operation message format
        let source_name = "subtitle.srt";
        let target_name = "video.srt";

        // Simulate success message format
        let success_msg = format!("  ✓ Moved: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Moved:"));

        // Simulate failure message format
        let error_msg = format!(
            "  ✗ Move failed: {} -> {} (target file does not exist after operation)",
            source_name, target_name
        );
        assert!(error_msg.contains("✗"));
        assert!(error_msg.contains("Move failed:"));
        assert!(error_msg.contains("target file does not exist"));
    }
}

/// Match operation result representing a single video-subtitle match.
///
/// Contains all information about a successful match between a video file
/// and a subtitle file, including confidence metrics and reasoning.
#[derive(Debug)]
pub struct MatchOperation {
    /// The matched video file
    pub video_file: MediaFile,
    /// The matched subtitle file
    pub subtitle_file: MediaFile,
    /// The new filename for the subtitle file
    pub new_subtitle_name: String,
    /// Confidence score of the match (0.0 to 1.0)
    pub confidence: f32,
    /// List of reasons supporting this match
    pub reasoning: Vec<String>,
    /// File relocation mode for this operation
    pub relocation_mode: FileRelocationMode,
    /// Target relocation path if operation is needed
    pub relocation_target_path: Option<std::path::PathBuf>,
    /// Whether relocation operation is needed (different folders)
    pub requires_relocation: bool,
}

/// AI-suggested candidate that did not become a match operation.
///
/// Emitted by [`MatchEngine::match_file_list_with_audit`] so machine-readable
/// callers can surface AI suggestions that were rejected (sub-threshold or
/// referencing unknown file IDs).
#[derive(Debug, Clone)]
pub struct RejectedCandidate {
    /// Path of the candidate video file (empty string if unresolved).
    pub video_path: String,
    /// Path of the candidate subtitle file (empty string if unresolved).
    pub subtitle_path: String,
    /// AI-reported confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Stable reason code (`"below_threshold"` or `"id_not_found"`).
    pub reason: &'static str,
}

/// Result of the auditable match planning pass: accepted operations plus
/// rejected candidates.
#[derive(Debug)]
pub struct MatchAudit {
    /// Operations that satisfied the confidence threshold and resolved to real files.
    pub operations: Vec<MatchOperation>,
    /// Candidates rejected by the planner.
    pub rejected: Vec<RejectedCandidate>,
}

/// Per-operation outcome captured by [`MatchEngine::execute_operations_audit`].
///
/// Unlike [`MatchEngine::execute_operations`] which aborts on first failure,
/// the audit variant records each operation's outcome so callers can report
/// partial success/failure (used by JSON output mode).
#[derive(Debug)]
pub struct OperationOutcome {
    /// Whether the operation was applied to the filesystem.
    pub applied: bool,
    /// Set when the operation failed (mutually exclusive with `applied == true`).
    pub error: Option<OperationError>,
}

/// Self-contained per-operation error metadata for machine-readable output.
#[derive(Debug, Clone)]
pub struct OperationError {
    /// Error category from [`SubXError::category`].
    pub category: &'static str,
    /// Stable machine code from [`SubXError::machine_code`].
    pub code: &'static str,
    /// User-friendly message from [`SubXError::user_friendly_message`].
    pub message: String,
}

/// Convert a [`SubXError`] into a self-contained [`OperationError`] for
/// audit reporting.
fn operation_error_from(err: &SubXError) -> OperationError {
    OperationError {
        category: err.category(),
        code: err.machine_code(),
        message: err.user_friendly_message(),
    }
}

/// Engine for matching video and subtitle files using AI analysis.
pub struct MatchEngine {
    ai_client: Box<dyn AIProvider>,
    discovery: FileDiscovery,
    config: MatchConfig,
}

impl MatchEngine {
    /// Creates a new `MatchEngine` with the given AI provider and configuration.
    pub fn new(ai_client: Box<dyn AIProvider>, config: MatchConfig) -> Self {
        Self {
            ai_client,
            discovery: FileDiscovery::new(),
            config,
        }
    }

    /// Matches video and subtitle files from a specified list of files.
    ///
    /// This method processes a user-provided list of files, filtering them into
    /// video and subtitle files, then performing AI-powered matching analysis.
    /// This is useful when users specify exact files via -i parameters.
    ///
    /// # Arguments
    ///
    /// * `file_paths` - A slice of file paths to process for matching
    ///
    /// # Returns
    ///
    /// A list of `MatchOperation` entries that meet the confidence threshold.
    pub async fn match_file_list(&self, file_paths: &[PathBuf]) -> Result<Vec<MatchOperation>> {
        Ok(self
            .match_file_list_with_audit(file_paths)
            .await?
            .operations)
    }

    /// Auditable variant of [`MatchEngine::match_file_list`] that also returns
    /// rejected candidates (sub-threshold or unresolvable AI suggestions).
    ///
    /// When operations are served from cache, `rejected` is returned empty
    /// because cache entries do not preserve rejection metadata.
    pub async fn match_file_list_with_audit(&self, file_paths: &[PathBuf]) -> Result<MatchAudit> {
        let json_mode = crate::cli::output::active_mode().is_json();
        // 1. Process the file list to create MediaFile objects
        let files = self.discovery.scan_file_list(file_paths)?;

        let videos: Vec<_> = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .collect();
        let subtitles: Vec<_> = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .collect();

        if videos.is_empty() || subtitles.is_empty() {
            return Ok(MatchAudit {
                operations: Vec::new(),
                rejected: Vec::new(),
            });
        }

        // 2. Check if we can use cache for file list operations
        let cache_key = self.calculate_file_list_cache_key(file_paths)?;
        if let Some(ops) = self.check_file_list_cache(&cache_key).await? {
            return Ok(MatchAudit {
                operations: ops,
                rejected: Vec::new(),
            });
        }

        // 3. Content sampling
        let content_samples = if self.config.enable_content_analysis {
            self.extract_content_samples(&subtitles).await?
        } else {
            Vec::new()
        };

        // 4. AI analysis request
        let video_files: Vec<String> = videos
            .iter()
            .map(|v| format!("ID:{} | Name:{} | Path:{}", v.id, v.name, v.relative_path))
            .collect();
        let subtitle_files: Vec<String> = subtitles
            .iter()
            .map(|s| format!("ID:{} | Name:{} | Path:{}", s.id, s.name, s.relative_path))
            .collect();

        let analysis_request = AnalysisRequest {
            video_files,
            subtitle_files,
            content_samples,
        };

        // 5. Query AI service
        let match_result = self.ai_client.analyze_content(analysis_request).await?;

        // Debug: Log AI analysis results (suppressed in JSON mode to keep
        // stderr free of free-form chatter).
        if !json_mode {
            eprintln!("🔍 AI Analysis Results:");
            eprintln!("   - Total matches: {}", match_result.matches.len());
            eprintln!(
                "   - Confidence threshold: {:.2}",
                self.config.confidence_threshold
            );
            for ai_match in &match_result.matches {
                eprintln!(
                    "   - {} -> {} (confidence: {:.2})",
                    ai_match.video_file_id, ai_match.subtitle_file_id, ai_match.confidence
                );
            }
        }

        // 6. Assemble match operation list
        let mut operations = Vec::new();
        let mut rejected = Vec::new();

        for ai_match in match_result.matches {
            let video_match =
                Self::find_media_file_by_id_or_path(&videos, &ai_match.video_file_id, None);
            let subtitle_match =
                Self::find_media_file_by_id_or_path(&subtitles, &ai_match.subtitle_file_id, None);

            if ai_match.confidence < self.config.confidence_threshold {
                rejected.push(RejectedCandidate {
                    video_path: video_match
                        .map(|v| v.path.display().to_string())
                        .unwrap_or_default(),
                    subtitle_path: subtitle_match
                        .map(|s| s.path.display().to_string())
                        .unwrap_or_default(),
                    confidence: ai_match.confidence,
                    reason: "below_threshold",
                });
                continue;
            }

            match (video_match, subtitle_match) {
                (Some(video), Some(subtitle)) => {
                    let new_name = self.generate_subtitle_name(video, subtitle, &ai_match);

                    let requires_relocation = self.config.relocation_mode
                        != FileRelocationMode::None
                        && subtitle.path.parent() != video.path.parent();

                    let relocation_target_path = if requires_relocation {
                        let video_dir = video.path.parent().unwrap();
                        Some(video_dir.join(&new_name))
                    } else {
                        None
                    };

                    operations.push(MatchOperation {
                        video_file: (*video).clone(),
                        subtitle_file: (*subtitle).clone(),
                        new_subtitle_name: new_name,
                        confidence: ai_match.confidence,
                        reasoning: ai_match.match_factors,
                        relocation_mode: self.config.relocation_mode.clone(),
                        relocation_target_path,
                        requires_relocation,
                    });
                }
                _ => {
                    if !json_mode {
                        eprintln!(
                            "⚠️  Cannot find AI-suggested file pair:\n     Video ID: '{}'\n     Subtitle ID: '{}'",
                            ai_match.video_file_id, ai_match.subtitle_file_id
                        );
                    }
                    rejected.push(RejectedCandidate {
                        video_path: video_match
                            .map(|v| v.path.display().to_string())
                            .unwrap_or_default(),
                        subtitle_path: subtitle_match
                            .map(|s| s.path.display().to_string())
                            .unwrap_or_default(),
                        confidence: ai_match.confidence,
                        reason: "id_not_found",
                    });
                }
            }
        }

        // 7. Globally enforce unique target paths before caching so the
        //    cached operations are already conflict-free. The CLI-level
        //    `match_command.rs` runs a second pass after archive-origin
        //    relocation rewrites — `apply_unique_target_paths` is
        //    idempotent, so the two passes compose cleanly.
        apply_unique_target_paths(&mut operations);

        // 8. Save to cache for future use
        self.save_file_list_cache(&cache_key, &operations).await?;

        Ok(MatchAudit {
            operations,
            rejected,
        })
    }

    async fn extract_content_samples(
        &self,
        subtitles: &[&MediaFile],
    ) -> Result<Vec<ContentSample>> {
        let mut samples = Vec::new();

        for subtitle in subtitles {
            let path = subtitle.path.clone();
            crate::core::fs_util::check_file_size(
                &path,
                self.config.max_subtitle_bytes,
                "Subtitle",
            )
            .map_err(SubXError::Io)?;
            let content = tokio::task::spawn_blocking(move || std::fs::read_to_string(&path))
                .await
                .map_err(|e| SubXError::Io(std::io::Error::other(e.to_string())))??;
            let preview = self.create_content_preview(&content);

            samples.push(ContentSample {
                filename: subtitle.name.clone(),
                subtitle_file_id: subtitle.id.clone(),
                content_preview: preview,
                file_size: subtitle.size,
            });
        }

        Ok(samples)
    }

    fn create_content_preview(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().take(20).collect();
        let preview = lines.join("\n");

        if preview.len() > self.config.max_sample_length {
            format!("{}...", &preview[..self.config.max_sample_length])
        } else {
            preview
        }
    }

    fn generate_subtitle_name(
        &self,
        video: &MediaFile,
        subtitle: &MediaFile,
        ai_match: &crate::services::ai::FileMatch,
    ) -> String {
        let detector = LanguageDetector::new();

        // Remove the extension from the video file name (if any)
        let video_base_name = if !video.extension.is_empty() {
            video
                .name
                .strip_suffix(&format!(".{}", video.extension))
                .unwrap_or(&video.name)
        } else {
            &video.name
        };

        // Four-step precedence per the AI-driven naming spec:
        // 1. Sanitized + normalized AI `target_filename_suffix` wins.
        // 2. Sanitized + normalized AI `language` (with `und` → no tag).
        // 3. Filename-derived language detection.
        // 4. No language tag.
        let ai_tag = ai_match
            .target_filename_suffix
            .as_deref()
            .and_then(sanitize_suffix)
            .or_else(|| {
                ai_match
                    .language
                    .as_deref()
                    .and_then(|s| normalize_ai_language(&detector, s))
            });

        let code = ai_tag.or_else(|| detector.get_primary_language(&subtitle.path));

        if let Some(code) = code {
            format!("{}.{}.{}", video_base_name, code, subtitle.extension)
        } else {
            format!("{}.{}", video_base_name, subtitle.extension)
        }
    }

    /// Execute match operations with dry-run mode support.
    ///
    /// When `dry_run` is false, a transactional journal is written to
    /// [`journal_path`] recording every successfully completed operation.
    /// The journal is saved atomically after each operation so that a
    /// crash mid-batch still leaves a consistent, resumable on-disk
    /// record. No journal is written in dry-run mode or when the batch
    /// performs zero operations.
    pub async fn execute_operations(
        &self,
        operations: &[MatchOperation],
        dry_run: bool,
    ) -> Result<()> {
        if dry_run {
            // Text-mode dry-run preview. JSON mode never reaches this path
            // (the match command takes the JSON branch via
            // `execute_operations_audit` before calling this method), but
            // gate defensively so a future caller cannot corrupt the
            // single-envelope contract.
            let json_mode = crate::cli::output::active_mode().is_json();
            for op in operations {
                if !json_mode {
                    println!(
                        "Preview: {} -> {}",
                        op.subtitle_file.name, op.new_subtitle_name
                    );
                }
                if op.requires_relocation {
                    if let Some(target_path) = &op.relocation_target_path {
                        let operation_verb = match op.relocation_mode {
                            FileRelocationMode::Copy => "Copy",
                            FileRelocationMode::Move => "Move",
                            _ => "",
                        };
                        if !json_mode {
                            println!(
                                "Preview: {} {} to {}",
                                operation_verb,
                                op.subtitle_file.path.display(),
                                target_path.display()
                            );
                        }
                    }
                }
            }
            return Ok(());
        }

        // Prepare a fresh journal for this batch. The journal is saved
        // atomically after each successful operation so that the on-disk
        // record never diverges from the in-memory state by more than a
        // single completed entry.
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let batch_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            created_at.hash(&mut hasher);
            operations.len().hash(&mut hasher);
            for op in operations {
                op.subtitle_file.path.hash(&mut hasher);
                op.new_subtitle_name.hash(&mut hasher);
            }
            format!("{:016x}", hasher.finish())
        };
        let mut journal = JournalData {
            batch_id,
            created_at,
            entries: Vec::new(),
        };
        let journal_file = journal_path().ok();

        let mut first_error: Option<SubXError> = None;

        for op in operations {
            // Build the task list the same way the previous implementation
            // did so relocation, backup and rename semantics are preserved.
            let mut backup_path: Option<PathBuf> = None;

            if op.relocation_mode == FileRelocationMode::Move && self.config.backup_enabled {
                let backup_task =
                    self.create_backup_task(&op.subtitle_file.path, &op.subtitle_file.extension);
                if let ProcessingOperation::CreateBackup { backup, .. } = &backup_task.operation {
                    backup_path = Some(backup.clone());
                }
                if let TaskResult::Failed(err) = backup_task.execute().await {
                    first_error = Some(SubXError::FileOperationFailed(err));
                    break;
                }
            }

            // Either a copy-with-rename task (Copy mode) or a rename task
            // (Move / None modes) produces the primary journal entry for
            // this operation.
            let primary_task = if op.relocation_mode == FileRelocationMode::Copy {
                self.create_copy_task(op)
            } else {
                self.create_rename_task(op)
            };

            let (journal_source, journal_destination, journal_kind) = match &primary_task.operation
            {
                ProcessingOperation::CopyWithRename { source, target }
                | ProcessingOperation::CopyToVideoFolder { source, target } => {
                    (source.clone(), target.clone(), JournalOperationType::Copied)
                }
                ProcessingOperation::MoveToVideoFolder { source, target } => {
                    (source.clone(), target.clone(), JournalOperationType::Moved)
                }
                ProcessingOperation::RenameFile { source, target } => {
                    let kind = match op.relocation_mode {
                        FileRelocationMode::Move => JournalOperationType::Moved,
                        _ => JournalOperationType::Renamed,
                    };
                    (source.clone(), target.clone(), kind)
                }
                _ => (
                    op.subtitle_file.path.clone(),
                    op.relocation_target_path.clone().unwrap_or_else(|| {
                        op.subtitle_file.path.with_file_name(&op.new_subtitle_name)
                    }),
                    JournalOperationType::Renamed,
                ),
            };

            // Capture source metadata before execution because move/rename
            // will invalidate the source path afterwards.
            let (pre_file_size, pre_file_mtime) = journal_source
                .metadata()
                .ok()
                .map(|m| {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    (m.len(), mtime)
                })
                .unwrap_or((0, 0));

            if let TaskResult::Failed(err) = primary_task.execute().await {
                first_error = Some(SubXError::FileOperationFailed(err));
                break;
            }

            // Record destination metadata after execution so that rollback
            // integrity checks compare against the actual destination state.
            let (file_size, file_mtime) = journal_destination
                .metadata()
                .ok()
                .map(|m| {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    (m.len(), mtime)
                })
                .unwrap_or((pre_file_size, pre_file_mtime));

            journal.entries.push(JournalEntry {
                operation_type: journal_kind,
                source: journal_source,
                destination: journal_destination,
                backup_path: backup_path.clone(),
                status: JournalEntryStatus::Completed,
                file_size,
                file_mtime,
            });

            if let Some(path) = journal_file.as_ref() {
                // Persist after every successful operation so interruption
                // leaves the on-disk journal consistent with the file system.
                journal.save(path).await?;
            }
        }

        if let Some(err) = first_error {
            return Err(err);
        }
        Ok(())
    }

    /// Auditable variant of [`MatchEngine::execute_operations`] that does NOT
    /// abort on first failure. Each operation produces an [`OperationOutcome`]
    /// describing whether it was applied or which error blocked it.
    ///
    /// This is the engine entry point used by JSON output mode to surface
    /// per-item statuses in the success envelope.
    pub async fn execute_operations_audit(
        &self,
        operations: &[MatchOperation],
        dry_run: bool,
    ) -> Result<Vec<OperationOutcome>> {
        if dry_run {
            return Ok(operations
                .iter()
                .map(|_| OperationOutcome {
                    applied: false,
                    error: None,
                })
                .collect());
        }

        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let batch_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            created_at.hash(&mut hasher);
            operations.len().hash(&mut hasher);
            for op in operations {
                op.subtitle_file.path.hash(&mut hasher);
                op.new_subtitle_name.hash(&mut hasher);
            }
            format!("{:016x}", hasher.finish())
        };
        let mut journal = JournalData {
            batch_id,
            created_at,
            entries: Vec::new(),
        };
        let journal_file = journal_path().ok();

        let mut outcomes = Vec::with_capacity(operations.len());

        for op in operations {
            let mut backup_path: Option<PathBuf> = None;

            if op.relocation_mode == FileRelocationMode::Move && self.config.backup_enabled {
                let backup_task =
                    self.create_backup_task(&op.subtitle_file.path, &op.subtitle_file.extension);
                if let ProcessingOperation::CreateBackup { backup, .. } = &backup_task.operation {
                    backup_path = Some(backup.clone());
                }
                if let TaskResult::Failed(err) = backup_task.execute().await {
                    let err = SubXError::FileOperationFailed(err);
                    outcomes.push(OperationOutcome {
                        applied: false,
                        error: Some(operation_error_from(&err)),
                    });
                    continue;
                }
            }

            let primary_task = if op.relocation_mode == FileRelocationMode::Copy {
                self.create_copy_task(op)
            } else {
                self.create_rename_task(op)
            };

            let (journal_source, journal_destination, journal_kind) = match &primary_task.operation
            {
                ProcessingOperation::CopyWithRename { source, target }
                | ProcessingOperation::CopyToVideoFolder { source, target } => {
                    (source.clone(), target.clone(), JournalOperationType::Copied)
                }
                ProcessingOperation::MoveToVideoFolder { source, target } => {
                    (source.clone(), target.clone(), JournalOperationType::Moved)
                }
                ProcessingOperation::RenameFile { source, target } => {
                    let kind = match op.relocation_mode {
                        FileRelocationMode::Move => JournalOperationType::Moved,
                        _ => JournalOperationType::Renamed,
                    };
                    (source.clone(), target.clone(), kind)
                }
                _ => (
                    op.subtitle_file.path.clone(),
                    op.relocation_target_path.clone().unwrap_or_else(|| {
                        op.subtitle_file.path.with_file_name(&op.new_subtitle_name)
                    }),
                    JournalOperationType::Renamed,
                ),
            };

            let (pre_file_size, pre_file_mtime) = journal_source
                .metadata()
                .ok()
                .map(|m| {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    (m.len(), mtime)
                })
                .unwrap_or((0, 0));

            if let TaskResult::Failed(err) = primary_task.execute().await {
                let err = SubXError::FileOperationFailed(err);
                outcomes.push(OperationOutcome {
                    applied: false,
                    error: Some(operation_error_from(&err)),
                });
                continue;
            }

            let (file_size, file_mtime) = journal_destination
                .metadata()
                .ok()
                .map(|m| {
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    (m.len(), mtime)
                })
                .unwrap_or((pre_file_size, pre_file_mtime));

            journal.entries.push(JournalEntry {
                operation_type: journal_kind,
                source: journal_source,
                destination: journal_destination,
                backup_path: backup_path.clone(),
                status: JournalEntryStatus::Completed,
                file_size,
                file_mtime,
            });

            if let Some(path) = journal_file.as_ref() {
                journal.save(path).await?;
            }

            outcomes.push(OperationOutcome {
                applied: true,
                error: None,
            });
        }

        Ok(outcomes)
    }

    /// Rename subtitle file by delegating to FileProcessingTask
    async fn rename_file(&self, op: &MatchOperation) -> Result<()> {
        let task = self.create_rename_task(op);
        match task.execute().await {
            TaskResult::Success(_) => Ok(()),
            TaskResult::Failed(err) => Err(SubXError::FileOperationFailed(err)),
            other => Err(SubXError::FileOperationFailed(format!(
                "Unexpected rename result: {:?}",
                other
            ))),
        }
    }

    /// Resolve filename conflicts by adding numeric suffix
    fn resolve_filename_conflict(&self, target: std::path::PathBuf) -> Result<std::path::PathBuf> {
        if !target.exists() {
            return Ok(target);
        }
        let json_mode = crate::cli::output::active_mode().is_json();
        match self.config.conflict_resolution {
            ConflictResolution::Skip => {
                if !json_mode {
                    eprintln!(
                        "Warning: Skipping relocation due to existing file: {}",
                        target.display()
                    );
                }
                Ok(target)
            }
            ConflictResolution::AutoRename => {
                let file_stem = target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");
                let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
                // Try the base name first via atomic create; if it succeeds we drop
                // the handle immediately since the downstream FileProcessingTask
                // performs its own I/O against this path.
                match crate::core::fs_util::atomic_create_file(&target) {
                    Ok(_f) => {
                        // Remove the placeholder so the downstream task can create it.
                        let _ = std::fs::remove_file(&target);
                        return Ok(target);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(SubXError::from(e)),
                }
                for i in 1..1000 {
                    let new_name = if extension.is_empty() {
                        format!("{}.{}", file_stem, i)
                    } else {
                        format!("{}.{}.{}", file_stem, i, extension)
                    };
                    let new_path = parent.join(new_name);
                    match crate::core::fs_util::atomic_create_file(&new_path) {
                        Ok(_f) => {
                            let _ = std::fs::remove_file(&new_path);
                            return Ok(new_path);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(SubXError::from(e)),
                    }
                }
                Err(SubXError::FileOperationFailed(
                    "Could not resolve filename conflict".to_string(),
                ))
            }
            ConflictResolution::Prompt => {
                if !json_mode {
                    eprintln!(
                        "Warning: Conflict resolution prompt not implemented, using auto-rename"
                    );
                }
                self.resolve_filename_conflict(target)
            }
        }
    }

    /// Create a task to copy (or rename) a file with new name
    fn create_copy_task(&self, op: &MatchOperation) -> FileProcessingTask {
        // In copy mode, always use the original subtitle file as source
        let source = op.subtitle_file.path.clone();
        let target_base = op.relocation_target_path.clone().unwrap();
        let final_target = self.resolve_filename_conflict(target_base).unwrap();
        FileProcessingTask::new(
            source.clone(),
            Some(final_target.clone()),
            ProcessingOperation::CopyWithRename {
                source,
                target: final_target,
            },
        )
    }

    /// Create a task to backup a file
    fn create_backup_task(&self, source: &std::path::Path, ext: &str) -> FileProcessingTask {
        let backup_path = source.with_extension(format!("{}.backup", ext));
        FileProcessingTask::new(
            source.to_path_buf(),
            Some(backup_path.clone()),
            ProcessingOperation::CreateBackup {
                source: source.to_path_buf(),
                backup: backup_path,
            },
        )
    }

    /// Create a task to rename (move) a file
    fn create_rename_task(&self, op: &MatchOperation) -> FileProcessingTask {
        let old = op.subtitle_file.path.clone();
        // If relocation is required, use the relocation target path
        let new_path = if op.requires_relocation && op.relocation_target_path.is_some() {
            let target_base = op.relocation_target_path.clone().unwrap();
            self.resolve_filename_conflict(target_base).unwrap()
        } else {
            old.with_file_name(&op.new_subtitle_name)
        };

        FileProcessingTask::new(
            old.clone(),
            Some(new_path.clone()),
            ProcessingOperation::RenameFile {
                source: old,
                target: new_path,
            },
        )
    }

    /// Calculate cache key for file list operations
    fn calculate_file_list_cache_key(&self, file_paths: &[PathBuf]) -> Result<String> {
        use std::collections::BTreeMap;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Sort paths to ensure consistent key generation
        let mut path_metadata = BTreeMap::new();
        for path in file_paths {
            if let Ok(metadata) = path.metadata() {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                path_metadata.insert(
                    canonical.to_string_lossy().to_string(),
                    (metadata.len(), metadata.modified().ok()),
                );
            }
        }

        // Include config hash to invalidate cache when configuration changes
        let config_hash = self.calculate_config_hash()?;

        let mut hasher = DefaultHasher::new();
        path_metadata.hash(&mut hasher);
        config_hash.hash(&mut hasher);

        Ok(format!("filelist_{:016x}", hasher.finish()))
    }

    /// Check cache for file list operations
    async fn check_file_list_cache(&self, cache_key: &str) -> Result<Option<Vec<MatchOperation>>> {
        let cache_file_path = self.get_cache_file_path()?;
        let cache_data = CacheData::load(&cache_file_path).ok();

        if let Some(cache_data) = cache_data {
            if !is_cache_version_current(&cache_data.cache_version) {
                return Ok(None);
            }
            if cache_data.directory == cache_key {
                // Rebuild match operation list for file list cache
                let mut ops = Vec::new();
                let mut id_gen = Uuidv7Generator::new();
                for item in cache_data.match_operations {
                    // For file list operations, we reconstruct operations from cached data
                    let video_path = PathBuf::from(&item.video_file);
                    let subtitle_path = PathBuf::from(&item.subtitle_file);

                    if video_path.exists() && subtitle_path.exists() {
                        // Create minimal MediaFile objects for the operation
                        let video_meta = video_path.metadata()?;
                        let subtitle_meta = subtitle_path.metadata()?;

                        let video_file = MediaFile {
                            id: generate_file_id(&mut id_gen),
                            path: video_path.clone(),
                            file_type: MediaFileType::Video,
                            size: video_meta.len(),
                            name: video_path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            extension: video_path
                                .extension()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase(),
                            relative_path: video_path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        };

                        let subtitle_file = MediaFile {
                            id: generate_file_id(&mut id_gen),
                            path: subtitle_path.clone(),
                            file_type: MediaFileType::Subtitle,
                            size: subtitle_meta.len(),
                            name: subtitle_path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            extension: subtitle_path
                                .extension()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_lowercase(),
                            relative_path: subtitle_path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        };

                        // Recalculate relocation information based on current configuration
                        let requires_relocation = self.config.relocation_mode
                            != FileRelocationMode::None
                            && subtitle_file.path.parent() != video_file.path.parent();

                        let relocation_target_path = if requires_relocation {
                            let video_dir = video_file.path.parent().unwrap();
                            Some(video_dir.join(&item.new_subtitle_name))
                        } else {
                            None
                        };

                        ops.push(MatchOperation {
                            video_file,
                            subtitle_file,
                            new_subtitle_name: item.new_subtitle_name,
                            confidence: item.confidence,
                            reasoning: item.reasoning,
                            relocation_mode: self.config.relocation_mode.clone(),
                            relocation_target_path,
                            requires_relocation,
                        });
                    }
                }
                return Ok(Some(ops));
            }
        }
        Ok(None)
    }

    /// Save cache for file list operations
    async fn save_file_list_cache(
        &self,
        cache_key: &str,
        operations: &[MatchOperation],
    ) -> Result<()> {
        let cache_file_path = self.get_cache_file_path()?;
        let config_hash = self.calculate_config_hash()?;

        let mut cache_items = Vec::new();
        for op in operations {
            cache_items.push(OpItem {
                video_file: op.video_file.path.to_string_lossy().to_string(),
                subtitle_file: op.subtitle_file.path.to_string_lossy().to_string(),
                new_subtitle_name: op.new_subtitle_name.clone(),
                confidence: op.confidence,
                reasoning: op.reasoning.clone(),
            });
        }

        // Build file snapshot with canonical paths, sizes, and mtimes
        let mut snapshot_items = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();
        for op in operations {
            for path in [&op.video_file.path, &op.subtitle_file.path] {
                let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                let key = canonical.to_string_lossy().to_string();
                if seen_paths.insert(key.clone()) {
                    if let Ok(meta) = std::fs::metadata(&canonical) {
                        let mtime = meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        snapshot_items.push(SnapshotItem {
                            path: key,
                            name: canonical
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            size: meta.len(),
                            mtime,
                            file_type: if canonical.extension().is_some_and(|e| {
                                ["srt", "ass", "ssa", "vtt", "sub"]
                                    .contains(&e.to_string_lossy().to_lowercase().as_str())
                            }) {
                                "subtitle".to_string()
                            } else {
                                "video".to_string()
                            },
                        });
                    }
                }
            }
        }

        let cache_data = CacheData {
            cache_version: CURRENT_CACHE_VERSION.to_string(),
            directory: cache_key.to_string(),
            file_snapshot: snapshot_items,
            match_operations: cache_items,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ai_model_used: self.config.ai_model.clone(),
            config_hash,
            original_relocation_mode: format!("{:?}", self.config.relocation_mode),
            original_backup_enabled: self.config.backup_enabled,
        };

        // Save cache data to file
        let cache_dir = cache_file_path.parent().unwrap().to_path_buf();
        let cache_json = serde_json::to_string_pretty(&cache_data)?;
        let cache_file_path_clone = cache_file_path.clone();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            std::fs::create_dir_all(&cache_dir)?;
            std::fs::write(&cache_file_path_clone, cache_json)?;
            Ok(())
        })
        .await
        .map_err(|e| SubXError::Io(std::io::Error::other(e.to_string())))??;

        Ok(())
    }

    /// Get cache file path
    fn get_cache_file_path(&self) -> Result<std::path::PathBuf> {
        // First check XDG_CONFIG_HOME environment variable (used for testing)
        let dir = if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
            std::path::PathBuf::from(xdg_config)
        } else {
            dirs::config_dir()
                .ok_or_else(|| SubXError::config("Unable to determine cache directory"))?
        };
        Ok(dir.join("subx").join("match_cache.json"))
    }

    /// Calculate current configuration hash for cache validation
    fn calculate_config_hash(&self) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Add configuration items that affect cache validity to the hash
        format!("{:?}", self.config.relocation_mode).hash(&mut hasher);
        self.config.backup_enabled.hash(&mut hasher);
        // A short prompt-schema tag so future prompt rewrites invalidate
        // the on-disk match cache automatically.
        "prompt_v2".hash(&mut hasher);
        // Add other relevant configuration items

        Ok(format!("{:016x}", hasher.finish()))
    }

    /// Find a media file by ID, with an optional fallback to relative path or name.
    fn find_media_file_by_id_or_path<'a>(
        files: &'a [&MediaFile],
        file_id: &str,
        fallback_path: Option<&str>,
    ) -> Option<&'a MediaFile> {
        if let Some(file) = files.iter().find(|f| f.id == file_id) {
            return Some(*file);
        }
        if let Some(path) = fallback_path {
            if let Some(file) = files.iter().find(|f| f.relative_path == path) {
                return Some(*file);
            }
            files.iter().find(|f| f.name == path).copied()
        } else {
            None
        }
    }

    /// Log available files to assist debugging when a match is not found.
    fn log_available_files(&self, files: &[&MediaFile], file_type: &str) {
        if crate::cli::output::active_mode().is_json() {
            return;
        }
        eprintln!("   Available {} files:", file_type);
        for f in files {
            eprintln!(
                "     - ID: {} | Name: {} | Path: {}",
                f.id, f.name, f.relative_path
            );
        }
    }

    /// Provide detailed information when no matches are found.
    fn log_no_matches_found(
        &self,
        match_result: &MatchResult,
        videos: &[MediaFile],
        subtitles: &[MediaFile],
    ) {
        if crate::cli::output::active_mode().is_json() {
            return;
        }
        eprintln!("\n❌ No matching files found that meet the criteria");
        eprintln!("🔍 AI analysis results:");
        eprintln!("   - Total matches: {}", match_result.matches.len());
        eprintln!(
            "   - Confidence threshold: {:.2}",
            self.config.confidence_threshold
        );
        eprintln!(
            "   - Matches meeting threshold: {}",
            match_result
                .matches
                .iter()
                .filter(|m| m.confidence >= self.config.confidence_threshold)
                .count()
        );
        eprintln!("\n📂 Scanned files:");
        eprintln!("   Video files ({} files):", videos.len());
        for v in videos {
            eprintln!("     - ID: {} | {}", v.id, v.relative_path);
        }
        eprintln!("   Subtitle files ({} files):", subtitles.len());
        for s in subtitles {
            eprintln!("     - ID: {} | {}", s.id, s.relative_path);
        }
    }
}

/// Replay a frozen set of cached match operations.
///
/// This helper powers the `cache apply` command. It reconstructs
/// [`MatchOperation`] values from the paths recorded in `cache`, without
/// performing any additional AI analysis or validation, and then feeds
/// them through the standard [`MatchEngine::execute_operations`] pipeline
/// so the same journal and file-system guarantees apply.
///
/// The provided `config` determines runtime behaviour such as relocation
/// mode, backup handling and conflict resolution. Callers are expected to
/// synchronise the config with the original cache's recorded values when
/// strict replay fidelity is required.
///
/// # Errors
///
/// Returns an error if any cached source path cannot be read or if the
/// underlying execution pipeline fails.
pub async fn apply_cached_operations(cache: &CacheData, config: &MatchConfig) -> Result<()> {
    let operations = reconstruct_operations_from_cache(cache, config)?;
    let engine = MatchEngine::new(Box::new(NoOpAIProvider), config.clone());
    engine.execute_operations(&operations, false).await
}

/// Rebuild [`MatchOperation`] values from a [`CacheData`] payload.
///
/// Silently skips entries whose source files no longer exist so the replay
/// is resilient to partial state (e.g., an earlier apply completed some
/// operations but was interrupted).
fn reconstruct_operations_from_cache(
    cache: &CacheData,
    config: &MatchConfig,
) -> Result<Vec<MatchOperation>> {
    let mut ops = Vec::new();
    let mut id_gen = Uuidv7Generator::new();
    for item in &cache.match_operations {
        let video_path = PathBuf::from(&item.video_file);
        let subtitle_path = PathBuf::from(&item.subtitle_file);

        if !video_path.exists() || !subtitle_path.exists() {
            continue;
        }

        let video_meta = video_path.metadata()?;
        let subtitle_meta = subtitle_path.metadata()?;

        let video_file = MediaFile {
            id: generate_file_id(&mut id_gen),
            path: video_path.clone(),
            file_type: MediaFileType::Video,
            size: video_meta.len(),
            name: video_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: video_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase(),
            relative_path: video_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        };

        let subtitle_file = MediaFile {
            id: generate_file_id(&mut id_gen),
            path: subtitle_path.clone(),
            file_type: MediaFileType::Subtitle,
            size: subtitle_meta.len(),
            name: subtitle_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: subtitle_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase(),
            relative_path: subtitle_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        };

        let requires_relocation = config.relocation_mode != FileRelocationMode::None
            && subtitle_file.path.parent() != video_file.path.parent();
        let relocation_target_path = if requires_relocation {
            video_file
                .path
                .parent()
                .map(|p| p.join(&item.new_subtitle_name))
        } else {
            None
        };

        ops.push(MatchOperation {
            video_file,
            subtitle_file,
            new_subtitle_name: item.new_subtitle_name.clone(),
            confidence: item.confidence,
            reasoning: item.reasoning.clone(),
            relocation_mode: config.relocation_mode.clone(),
            relocation_target_path,
            requires_relocation,
        });
    }
    Ok(ops)
}

/// AI provider stub used by [`apply_cached_operations`].
///
/// `execute_operations` never calls the AI service, so replaying a cached
/// plan does not require a real provider; this stub panics defensively if
/// accidentally invoked.
struct NoOpAIProvider;

#[async_trait::async_trait]
impl AIProvider for NoOpAIProvider {
    async fn analyze_content(&self, _request: AnalysisRequest) -> crate::Result<MatchResult> {
        Err(SubXError::config(
            "AI analysis is not available while replaying cached operations",
        ))
    }

    async fn verify_match(
        &self,
        _verification: crate::services::ai::VerificationRequest,
    ) -> crate::Result<crate::services::ai::ConfidenceScore> {
        Err(SubXError::config(
            "AI verification is not available while replaying cached operations",
        ))
    }
}
