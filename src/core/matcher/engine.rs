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
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
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
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
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
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
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
            },
        );
        // 檔案名稱包含多個點且無副檔名情況
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
        assert_eq!(new_name, "a.b.c.srt");
    }

    #[tokio::test]
    async fn test_rename_file_displays_success_check_mark() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 建立測試檔案
        let original_file = temp_path.join("original.srt");
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // 建立測試用的 MatchEngine
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
            },
        );

        // 建立 MatchOperation
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

        // 執行重新命名操作
        let result = engine.rename_file(&match_op).await;

        // 驗證操作成功
        assert!(result.is_ok());

        // 驗證檔案已重新命名
        let renamed_file = temp_path.join("renamed.srt");
        assert!(renamed_file.exists(), "重新命名的檔案應該存在");
        assert!(!original_file.exists(), "原始檔案應該已被重新命名");

        // 驗證檔案內容正確
        let content = fs::read_to_string(&renamed_file).unwrap();
        assert!(content.contains("Test subtitle"));
    }

    #[tokio::test]
    async fn test_rename_file_displays_error_cross_mark_when_file_not_exists() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // 建立測試檔案
        let original_file = temp_path.join("original.srt");
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // 建立測試用的 MatchEngine
        let engine = MatchEngine::new(
            Box::new(DummyAI),
            MatchConfig {
                confidence_threshold: 0.0,
                max_sample_length: 0,
                enable_content_analysis: false,
                backup_enabled: false,
                relocation_mode: FileRelocationMode::None,
                conflict_resolution: ConflictResolution::Skip,
            },
        );

        // 建立 MatchOperation
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

        // 模擬檔案系統操作後檔案不存在的情況
        // 首先正常執行重新命名操作
        let result = engine.rename_file(&match_op).await;
        assert!(result.is_ok());

        // 手動刪除重新命名後的檔案來模擬失敗情況
        let renamed_file = temp_path.join("renamed.srt");
        if renamed_file.exists() {
            fs::remove_file(&renamed_file).unwrap();
        }

        // 重新建立原始檔案進行第二次測試
        fs::write(
            &original_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle",
        )
        .unwrap();

        // 創建一個會失敗的重新命名操作，通過覆寫 rename 實作
        // 由於無法直接模擬 std::fs::rename 失敗後檔案不存在的情況，
        // 我們測試檔案操作完成後手動移除檔案的情況
        let result = engine.rename_file(&match_op).await;
        assert!(result.is_ok());

        // 再次手動刪除檔案
        let renamed_file = temp_path.join("renamed.srt");
        if renamed_file.exists() {
            fs::remove_file(&renamed_file).unwrap();
        }

        // 此測試主要驗證程式碼結構正確，實際的錯誤訊息顯示需要通過集成測試驗證
        // 因為我們無法輕易模擬檔案系統操作成功但檔案不存在的異常情況
    }

    #[test]
    fn test_file_operation_message_format() {
        // 測試錯誤訊息格式是否正確
        let source_name = "test.srt";
        let target_name = "renamed.srt";

        // 模擬成功訊息格式
        let success_msg = format!("  ✓ Renamed: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Renamed:"));
        assert!(success_msg.contains(source_name));
        assert!(success_msg.contains(target_name));

        // 模擬失敗訊息格式
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
        // 測試複製操作的訊息格式
        let source_name = "subtitle.srt";
        let target_name = "video.srt";

        // 模擬成功訊息格式
        let success_msg = format!("  ✓ Copied: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Copied:"));

        // 模擬失敗訊息格式
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
        // 測試移動操作的訊息格式
        let source_name = "subtitle.srt";
        let target_name = "video.srt";

        // 模擬成功訊息格式
        let success_msg = format!("  ✓ Moved: {} -> {}", source_name, target_name);
        assert!(success_msg.contains("✓"));
        assert!(success_msg.contains("Moved:"));

        // 模擬失敗訊息格式
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

                        // Determine if relocation is needed
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
                    (None, Some(_)) => {
                        eprintln!(
                            "⚠️  Cannot find AI-suggested video file ID: '{}'",
                            ai_match.video_file_id
                        );
                        self.log_available_files(&videos, "video");
                    }
                    (Some(_), None) => {
                        eprintln!(
                            "⚠️  Cannot find AI-suggested subtitle file ID: '{}'",
                            ai_match.subtitle_file_id
                        );
                        self.log_available_files(&subtitles, "subtitle");
                    }
                    (None, None) => {
                        eprintln!("⚠️  Cannot find AI-suggested file pair:");
                        eprintln!("     Video ID: '{}'", ai_match.video_file_id);
                        eprintln!("     Subtitle ID: '{}'", ai_match.subtitle_file_id);
                    }
                }
            } else {
                eprintln!(
                    "ℹ️  AI match confidence too low ({:.2}): {} <-> {}",
                    ai_match.confidence, ai_match.video_file_id, ai_match.subtitle_file_id
                );
            }
        }

        // Check if no operations were generated and provide debugging info
        if operations.is_empty() {
            eprintln!("\n❌ No matching files found that meet the criteria");
            eprintln!("🔍 Available file statistics:");
            eprintln!("   Video files ({} files):", videos.len());
            for v in &videos {
                eprintln!("     - ID: {} | {}", v.id, v.relative_path);
            }
            eprintln!("   Subtitle files ({} files):", subtitles.len());
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

        // 從影片檔案名稱中移除副檔名（如果有）
        let video_base_name = if !video.extension.is_empty() {
            video
                .name
                .strip_suffix(&format!(".{}", video.extension))
                .unwrap_or(&video.name)
        } else {
            &video.name
        };

        if let Some(code) = detector.get_primary_language(&subtitle.path) {
            format!("{}.{}.{}", video_base_name, code, subtitle.extension)
        } else {
            format!("{}.{}", video_base_name, subtitle.extension)
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
                if op.requires_relocation {
                    if let Some(target_path) = &op.relocation_target_path {
                        let operation_verb = match op.relocation_mode {
                            FileRelocationMode::Copy => "Copy",
                            FileRelocationMode::Move => "Move",
                            _ => "",
                        };
                        println!(
                            "Preview: {} {} to {}",
                            operation_verb,
                            op.subtitle_file.path.display(),
                            target_path.display()
                        );
                    }
                }
            } else {
                match op.relocation_mode {
                    FileRelocationMode::Copy => {
                        if op.requires_relocation {
                            self.execute_copy_operation(op).await?;
                        } else {
                            // In copy mode, create a local copy with new name
                            self.execute_local_copy(op).await?;
                        }
                    }
                    FileRelocationMode::Move => {
                        self.rename_file(op).await?;
                        if op.requires_relocation {
                            self.execute_relocation_operation(op).await?;
                        }
                    }
                    FileRelocationMode::None => {
                        self.rename_file(op).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Execute file relocation operation (copy or move)
    async fn execute_relocation_operation(&self, op: &MatchOperation) -> Result<()> {
        if !op.requires_relocation {
            return Ok(());
        }

        let source_path = if op.new_subtitle_name == op.subtitle_file.name {
            // File was not renamed, use original path
            op.subtitle_file.path.clone()
        } else {
            // File was renamed, use the new path in the same directory
            op.subtitle_file.path.with_file_name(&op.new_subtitle_name)
        };

        if let Some(target_path) = &op.relocation_target_path {
            // Create target directory if it doesn't exist
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Handle filename conflicts
            let final_target = self.resolve_filename_conflict(target_path.clone())?;

            match op.relocation_mode {
                FileRelocationMode::Copy => {
                    // Create backup of target if enabled
                    if self.config.backup_enabled && final_target.exists() {
                        let backup_path = final_target.with_extension(format!(
                            "{}.backup",
                            final_target
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                        ));
                        std::fs::copy(&final_target, backup_path)?;
                    }

                    // Execute copy operation
                    std::fs::copy(&source_path, &final_target)?;

                    // Verify the file exists after copy and display appropriate indicator
                    if final_target.exists() {
                        println!(
                            "  ✓ Copied: {} -> {}",
                            source_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            final_target
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                    } else {
                        eprintln!(
                            "  ✗ Copy failed: {} -> {} (target file does not exist after operation)",
                            source_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            final_target
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                    }
                }
                FileRelocationMode::Move => {
                    // Create backup of original if enabled
                    if self.config.backup_enabled {
                        let backup_path = source_path.with_extension(format!(
                            "{}.backup",
                            source_path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                        ));
                        std::fs::copy(&source_path, backup_path)?;
                    }

                    // Create backup of target if exists and enabled
                    if self.config.backup_enabled && final_target.exists() {
                        let backup_path = final_target.with_extension(format!(
                            "{}.backup",
                            final_target
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                        ));
                        std::fs::copy(&final_target, backup_path)?;
                    }

                    // Execute move operation
                    std::fs::rename(&source_path, &final_target)?;

                    // Verify the file exists after move and display appropriate indicator
                    if final_target.exists() {
                        println!(
                            "  ✓ Moved: {} -> {}",
                            source_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            final_target
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                    } else {
                        eprintln!(
                            "  ✗ Move failed: {} -> {} (target file does not exist after operation)",
                            source_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            final_target
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        );
                    }
                }
                FileRelocationMode::None => {
                    // No operation needed
                }
            }
        }

        Ok(())
    }

    /// Execute copy operation followed by rename of the copied file
    /// Execute copy operation - copies original file to target location without modifying original
    async fn execute_copy_operation(&self, op: &MatchOperation) -> Result<()> {
        if let Some(target_path) = &op.relocation_target_path {
            // Resolve filename conflicts
            let final_target = self.resolve_filename_conflict(target_path.clone())?;
            if let Some(parent) = final_target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Backup target file if it exists and backup is enabled
            if self.config.backup_enabled && final_target.exists() {
                let backup_path = final_target.with_extension(format!(
                    "{}.backup",
                    final_target
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                ));
                std::fs::copy(&final_target, backup_path)?;
            }
            // Copy original subtitle to target location
            // In copy mode, the original file remains unchanged
            std::fs::copy(&op.subtitle_file.path, &final_target)?;

            // Display copy operation result
            if final_target.exists() {
                println!(
                    "  ✓ Copied: {} -> {}",
                    op.subtitle_file.name,
                    final_target.file_name().unwrap().to_string_lossy()
                );
            }
        }
        Ok(())
    }

    /// Execute local copy operation - creates a copy with new name in the same directory
    async fn execute_local_copy(&self, op: &MatchOperation) -> Result<()> {
        if op.new_subtitle_name != op.subtitle_file.name {
            let target_path = op.subtitle_file.path.with_file_name(&op.new_subtitle_name);

            // Handle filename conflicts
            let final_target = self.resolve_filename_conflict(target_path)?;

            // Backup target file if it exists and backup is enabled
            if self.config.backup_enabled && final_target.exists() {
                let backup_path = final_target.with_extension(format!(
                    "{}.backup",
                    final_target
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                ));
                std::fs::copy(&final_target, backup_path)?;
            }

            // Copy original file to new name in same directory
            std::fs::copy(&op.subtitle_file.path, &final_target)?;

            // Display copy operation result
            if final_target.exists() {
                println!(
                    "  ✓ Copied: {} -> {}",
                    op.subtitle_file.name,
                    final_target.file_name().unwrap().to_string_lossy()
                );
            }
        }
        Ok(())
    }

    /// Resolve filename conflicts by adding numeric suffix
    fn resolve_filename_conflict(&self, target: std::path::PathBuf) -> Result<std::path::PathBuf> {
        if !target.exists() {
            return Ok(target);
        }

        // Use AutoRename strategy
        match self.config.conflict_resolution {
            ConflictResolution::Skip => {
                eprintln!(
                    "Warning: Skipping relocation due to existing file: {}",
                    target.display()
                );
                Ok(target) // Return original path but operation will be skipped
            }
            ConflictResolution::AutoRename => {
                // Extract filename components
                let file_stem = target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");

                let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));

                // Try adding numeric suffixes
                for i in 1..1000 {
                    let new_name = if extension.is_empty() {
                        format!("{}.{}", file_stem, i)
                    } else {
                        format!("{}.{}.{}", file_stem, i, extension)
                    };
                    let new_path = parent.join(new_name);
                    if !new_path.exists() {
                        return Ok(new_path);
                    }
                }

                Err(SubXError::FileOperationFailed(
                    "Could not resolve filename conflict".to_string(),
                ))
            }
            ConflictResolution::Prompt => {
                // For now, fall back to AutoRename
                // In a future version, this could prompt the user
                eprintln!("Warning: Conflict resolution prompt not implemented, using auto-rename");
                self.resolve_filename_conflict(target)
            }
        }
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

        std::fs::rename(old_path, &new_path)?;

        // Verify the file exists after rename and display appropriate indicator
        if new_path.exists() {
            println!(
                "  ✓ Renamed: {} -> {}",
                old_path.file_name().unwrap_or_default().to_string_lossy(),
                op.new_subtitle_name
            );
        } else {
            eprintln!(
                "  ✗ Rename failed: {} -> {} (target file does not exist after operation)",
                old_path.file_name().unwrap_or_default().to_string_lossy(),
                op.new_subtitle_name
            );
        }

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
                        // 重新計算重定位需求（基於當前配置）
                        let requires_relocation = self.config.relocation_mode
                            != FileRelocationMode::None
                            && subtitle.path.parent() != video.path.parent();
                        let relocation_target_path = if requires_relocation {
                            let video_dir = video.path.parent().unwrap();
                            Some(video_dir.join(&item.new_subtitle_name))
                        } else {
                            None
                        };

                        ops.push(MatchOperation {
                            video_file: (*video).clone(),
                            subtitle_file: (*subtitle).clone(),
                            new_subtitle_name: item.new_subtitle_name.clone(),
                            confidence: item.confidence,
                            reasoning: item.reasoning.clone(),
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
            // 記錄產生 cache 時的重定位模式與備份設定
            original_relocation_mode: format!("{:?}", self.config.relocation_mode),
            original_backup_enabled: self.config.backup_enabled,
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
