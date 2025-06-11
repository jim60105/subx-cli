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
use std::path::Path;

use crate::Result;
use crate::core::language::LanguageDetector;
use crate::core::matcher::cache::{CacheData, OpItem, SnapshotItem};
use crate::core::matcher::{FileDiscovery, MediaFile, MediaFileType};

use crate::error::SubXError;
use dirs;
use serde_json;

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
}

#[cfg(test)]
mod language_name_tests {
    use super::*;
    use crate::core::matcher::discovery::{MediaFile, MediaFileType};
    use crate::services::ai::{
        AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
    };
    use async_trait::async_trait;
    use std::path::PathBuf;

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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
        assert_eq!(new_name, "movie03.ass");
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

    /// Matches video and subtitle files under the given directory.
    ///
    /// # Arguments
    ///
    /// * `path` - Directory to scan for media files.
    /// * `recursive` - Whether to include subdirectories.
    ///
    /// # Returns
    ///
    /// A list of `MatchOperation` entries that meet the confidence threshold.
    pub async fn match_files(&self, path: &Path, recursive: bool) -> Result<Vec<MatchOperation>> {
        // 1. Explore files
        let files = self.discovery.scan_directory(path, recursive)?;

        let videos: Vec<_> = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .collect();
        let subtitles: Vec<_> = files
            .iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
            .collect();

        if videos.is_empty() || subtitles.is_empty() {
            return Ok(Vec::new());
        }

        // 2. Try to reuse results from Dry-run cache
        if let Some(ops) = self.check_cache(path, recursive).await? {
            return Ok(ops);
        }
        // 3. Content sampling
        let content_samples = if self.config.enable_content_analysis {
            self.extract_content_samples(&subtitles).await?
        } else {
            Vec::new()
        };

        // 4. AI analysis request
        // Generate AI analysis request: include file IDs for precise matching
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

        let match_result = self.ai_client.analyze_content(analysis_request).await?;

        // Debug: Log AI analysis results
        eprintln!("🔍 AI 分析結果:");
        eprintln!("   - 總匹配數: {}", match_result.matches.len());
        eprintln!("   - 信心度閾值: {:.2}", self.config.confidence_threshold);
        for ai_match in &match_result.matches {
            eprintln!(
                "   - {} -> {} (信心度: {:.2})",
                ai_match.video_file_id, ai_match.subtitle_file_id, ai_match.confidence
            );
        }

        // 4. Assemble match operation list
        let mut operations = Vec::new();

        for ai_match in match_result.matches {
            if ai_match.confidence >= self.config.confidence_threshold {
                let video_match =
                    Self::find_media_file_by_id_or_path(&videos, &ai_match.video_file_id, None);
                let subtitle_match = Self::find_media_file_by_id_or_path(
                    &subtitles,
                    &ai_match.subtitle_file_id,
                    None,
                );
                match (video_match, subtitle_match) {
                    (Some(video), Some(subtitle)) => {
                        let new_name = self.generate_subtitle_name(video, subtitle);
                        operations.push(MatchOperation {
                            video_file: (*video).clone(),
                            subtitle_file: (*subtitle).clone(),
                            new_subtitle_name: new_name,
                            confidence: ai_match.confidence,
                            reasoning: ai_match.match_factors,
                        });
                    }
                    (None, Some(_)) => {
                        eprintln!(
                            "⚠️  找不到 AI 建議的影片檔案 ID: '{}'",
                            ai_match.video_file_id
                        );
                        self.log_available_files(&videos, "影片");
                    }
                    (Some(_), None) => {
                        eprintln!(
                            "⚠️  找不到 AI 建議的字幕檔案 ID: '{}'",
                            ai_match.subtitle_file_id
                        );
                        self.log_available_files(&subtitles, "字幕");
                    }
                    (None, None) => {
                        eprintln!("⚠️  找不到 AI 建議的檔案對:");
                        eprintln!("     影片 ID: '{}'", ai_match.video_file_id);
                        eprintln!("     字幕 ID: '{}'", ai_match.subtitle_file_id);
                    }
                }
            } else {
                eprintln!(
                    "ℹ️  AI 匹配信心度過低 ({:.2}): {} <-> {}",
                    ai_match.confidence, ai_match.video_file_id, ai_match.subtitle_file_id
                );
            }
        }

        // Check if no operations were generated and provide debugging info
        if operations.is_empty() {
            eprintln!("\n❌ 沒有找到符合條件的檔案匹配");
            eprintln!("🔍 可用檔案統計:");
            eprintln!("   影片檔案 ({} 個):", videos.len());
            for v in &videos {
                eprintln!("     - ID: {} | {}", v.id, v.relative_path);
            }
            eprintln!("   字幕檔案 ({} 個):", subtitles.len());
            for s in &subtitles {
                eprintln!("     - ID: {} | {}", s.id, s.relative_path);
            }
        }

        Ok(operations)
    }

    async fn extract_content_samples(
        &self,
        subtitles: &[&MediaFile],
    ) -> Result<Vec<ContentSample>> {
        let mut samples = Vec::new();

        for subtitle in subtitles {
            let content = std::fs::read_to_string(&subtitle.path)?;
            let preview = self.create_content_preview(&content);

            samples.push(ContentSample {
                filename: subtitle.name.clone(),
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

    fn generate_subtitle_name(&self, video: &MediaFile, subtitle: &MediaFile) -> String {
        let detector = LanguageDetector::new();
        if let Some(code) = detector.get_primary_language(&subtitle.path) {
            format!("{}.{}.{}", video.name, code, subtitle.extension)
        } else {
            format!("{}.{}", video.name, subtitle.extension)
        }
    }

    /// Execute match operations with dry-run mode support
    pub async fn execute_operations(
        &self,
        operations: &[MatchOperation],
        dry_run: bool,
    ) -> Result<()> {
        for op in operations {
            if dry_run {
                println!(
                    "Preview: {} -> {}",
                    op.subtitle_file.name, op.new_subtitle_name
                );
            } else {
                self.rename_file(op).await?;
            }
        }
        Ok(())
    }

    async fn rename_file(&self, op: &MatchOperation) -> Result<()> {
        let old_path = &op.subtitle_file.path;
        let new_path = old_path.with_file_name(&op.new_subtitle_name);

        // Backup file
        if self.config.backup_enabled {
            let backup_path =
                old_path.with_extension(format!("{}.backup", op.subtitle_file.extension));
            std::fs::copy(old_path, backup_path)?;
        }

        std::fs::rename(old_path, new_path)?;
        Ok(())
    }
    /// Calculate file snapshot for specified directory for cache comparison
    fn calculate_file_snapshot(
        &self,
        directory: &Path,
        recursive: bool,
    ) -> Result<Vec<SnapshotItem>> {
        let files = self.discovery.scan_directory(directory, recursive)?;
        let mut snapshot = Vec::new();
        for f in files {
            let metadata = std::fs::metadata(&f.path)?;
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            snapshot.push(SnapshotItem {
                name: f.name.clone(),
                size: f.size,
                mtime,
                file_type: match f.file_type {
                    MediaFileType::Video => "video".to_string(),
                    MediaFileType::Subtitle => "subtitle".to_string(),
                },
            });
        }
        Ok(snapshot)
    }

    /// Check dry-run cache, return previous calculated match operations if hit
    pub async fn check_cache(
        &self,
        directory: &Path,
        recursive: bool,
    ) -> Result<Option<Vec<MatchOperation>>> {
        let current_snapshot = self.calculate_file_snapshot(directory, recursive)?;
        let cache_data = CacheData::load(&self.get_cache_file_path()?).ok();
        if let Some(cache_data) = cache_data {
            if cache_data.directory == directory.to_string_lossy()
                && cache_data.file_snapshot == current_snapshot
                && cache_data.ai_model_used == self.calculate_config_hash()?
                && cache_data.config_hash == self.calculate_config_hash()?
            {
                // Rebuild match operation list
                let files = self.discovery.scan_directory(directory, recursive)?;
                let mut ops = Vec::new();
                for item in cache_data.match_operations {
                    if let (Some(video), Some(subtitle)) = (
                        files.iter().find(|f| {
                            f.name == item.video_file && matches!(f.file_type, MediaFileType::Video)
                        }),
                        files.iter().find(|f| {
                            f.name == item.subtitle_file
                                && matches!(f.file_type, MediaFileType::Subtitle)
                        }),
                    ) {
                        ops.push(MatchOperation {
                            video_file: (*video).clone(),
                            subtitle_file: (*subtitle).clone(),
                            new_subtitle_name: item.new_subtitle_name.clone(),
                            confidence: item.confidence,
                            reasoning: item.reasoning.clone(),
                        });
                    }
                }
                return Ok(Some(ops));
            }
        }
        Ok(None)
    }

    /// Save dry-run cache results
    pub async fn save_cache(
        &self,
        directory: &Path,
        recursive: bool,
        operations: &[MatchOperation],
    ) -> Result<()> {
        let cache_data = CacheData {
            cache_version: "1.0".to_string(),
            directory: directory.to_string_lossy().to_string(),
            file_snapshot: self.calculate_file_snapshot(directory, recursive)?,
            match_operations: operations
                .iter()
                .map(|op| OpItem {
                    video_file: op.video_file.name.clone(),
                    subtitle_file: op.subtitle_file.name.clone(),
                    new_subtitle_name: op.new_subtitle_name.clone(),
                    confidence: op.confidence,
                    reasoning: op.reasoning.clone(),
                })
                .collect(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            ai_model_used: self.calculate_config_hash()?,
            config_hash: self.calculate_config_hash()?,
        };
        let path = self.get_cache_file_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            serde_json::to_string_pretty(&cache_data).map_err(|e| SubXError::Other(e.into()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get cache file path
    fn get_cache_file_path(&self) -> Result<std::path::PathBuf> {
        let dir = dirs::config_dir()
            .ok_or_else(|| SubXError::config("Unable to determine cache directory"))?;
        Ok(dir.join("subx").join("match_cache.json"))
    }

    /// Calculate current configuration hash for cache validation
    fn calculate_config_hash(&self) -> Result<String> {
        // Use a fixed hash for now since we don't have access to global config
        // This will be improved when cache validation is refactored
        Ok("default_config_hash".to_string())
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
        eprintln!("   可用的{}檔案:", file_type);
        for f in files {
            eprintln!(
                "     - ID: {} | 名稱: {} | 路徑: {}",
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
        eprintln!("\n❌ 沒有找到符合條件的檔案匹配");
        eprintln!("🔍 AI 分析結果:");
        eprintln!("   - 總匹配數: {}", match_result.matches.len());
        eprintln!("   - 信心度閾值: {:.2}", self.config.confidence_threshold);
        eprintln!(
            "   - 符合閾值的匹配: {}",
            match_result
                .matches
                .iter()
                .filter(|m| m.confidence >= self.config.confidence_threshold)
                .count()
        );
        eprintln!("\n📂 掃描到的檔案:");
        eprintln!("   影片檔案 ({} 個):", videos.len());
        for v in videos {
            eprintln!("     - ID: {} | {}", v.id, v.relative_path);
        }
        eprintln!("   字幕檔案 ({} 個):", subtitles.len());
        for s in subtitles {
            eprintln!("     - ID: {} | {}", s.id, s.relative_path);
        }
    }
}
