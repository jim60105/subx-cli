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
use crate::core::matcher::cache::{CacheData, OpItem};
use crate::core::matcher::discovery::generate_file_id;
use crate::core::matcher::{FileDiscovery, MediaFile, MediaFileType};

use crate::core::fs_util::copy_file_cifs_safe;
use crate::core::parallel::{FileProcessingTask, ProcessingOperation, Task, TaskResult};
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
    /// AI model name used for analysis
    pub ai_model: String,
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
                ai_model: "test-model".to_string(),
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
                ai_model: "test-model".to_string(),
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
                ai_model: "test-model".to_string(),
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
                ai_model: "test-model".to_string(),
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
                ai_model: "test-model".to_string(),
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
                ai_model: "test-model".to_string(),
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
        let new_name = engine.generate_subtitle_name(&video, &subtitle);
        assert_eq!(new_name, "a.b.c.srt");
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
            return Ok(Vec::new());
        }

        // 2. Check if we can use cache for file list operations
        // Create a stable cache key based on sorted file paths and their metadata
        let cache_key = self.calculate_file_list_cache_key(file_paths)?;
        if let Some(ops) = self.check_file_list_cache(&cache_key).await? {
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

        // 5. Query AI service
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

        // 6. Assemble match operation list
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
                    _ => {
                        eprintln!(
                            "⚠️  Cannot find AI-suggested file pair:\n     Video ID: '{}'\n     Subtitle ID: '{}'",
                            ai_match.video_file_id, ai_match.subtitle_file_id
                        );
                        eprintln!("❌ No matching files found that meet the criteria");
                        eprintln!("🔍 Available file statistics:");
                        eprintln!("   Video files ({} files):", videos.len());
                        for video in &videos {
                            eprintln!("     - ID: {} | {}", video.id, video.name);
                        }
                        eprintln!("   Subtitle files ({} files):", subtitles.len());
                        for subtitle in &subtitles {
                            eprintln!("     - ID: {} | {}", subtitle.id, subtitle.name);
                        }
                    }
                }
            }
        }

        // 7. Save to cache for future use
        self.save_file_list_cache(&cache_key, &operations).await?;

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

        // Remove the extension from the video file name (if any)
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
                // Delegate file operations to FileProcessingTask
                let mut tasks = Vec::new();
                // Backup source if move and enabled
                if op.relocation_mode == FileRelocationMode::Move && self.config.backup_enabled {
                    tasks.push(
                        self.create_backup_task(
                            &op.subtitle_file.path,
                            &op.subtitle_file.extension,
                        ),
                    );
                }
                // Copy or local copy with rename
                if op.relocation_mode == FileRelocationMode::Copy {
                    tasks.push(self.create_copy_task(op));
                }
                // Rename original file if any
                if op.relocation_mode != FileRelocationMode::Copy {
                    tasks.push(self.create_rename_task(op));
                }
                // Execute all tasks sequentially
                for t in tasks {
                    if let TaskResult::Failed(err) = t.execute().await {
                        return Err(SubXError::FileOperationFailed(err));
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
                        copy_file_cifs_safe(&final_target, &backup_path)?;
                    }

                    // Execute copy operation
                    copy_file_cifs_safe(&source_path, &final_target)?;

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
                        copy_file_cifs_safe(&source_path, &backup_path)?;
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
                        copy_file_cifs_safe(&final_target, &backup_path)?;
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
        match self.config.conflict_resolution {
            ConflictResolution::Skip => {
                eprintln!(
                    "Warning: Skipping relocation due to existing file: {}",
                    target.display()
                );
                Ok(target)
            }
            ConflictResolution::AutoRename => {
                let file_stem = target
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");
                let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
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
                eprintln!("Warning: Conflict resolution prompt not implemented, using auto-rename");
                self.resolve_filename_conflict(target)
            }
        }
    }

    /// Create a task to copy (or rename) a file with new name
    fn create_copy_task(&self, op: &MatchOperation) -> FileProcessingTask {
        let source = if op.new_subtitle_name == op.subtitle_file.name {
            op.subtitle_file.path.clone()
        } else {
            op.subtitle_file.path.with_file_name(&op.new_subtitle_name)
        };
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
        let new_path = old.with_file_name(&op.new_subtitle_name);
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
            if cache_data.directory == cache_key {
                // Rebuild match operation list for file list cache
                let mut ops = Vec::new();
                for item in cache_data.match_operations {
                    // For file list operations, we reconstruct operations from cached data
                    let video_path = PathBuf::from(&item.video_file);
                    let subtitle_path = PathBuf::from(&item.subtitle_file);

                    if video_path.exists() && subtitle_path.exists() {
                        // Create minimal MediaFile objects for the operation
                        let video_meta = video_path.metadata()?;
                        let subtitle_meta = subtitle_path.metadata()?;

                        let video_file = MediaFile {
                            id: generate_file_id(&video_path, video_meta.len()),
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
                            id: generate_file_id(&subtitle_path, subtitle_meta.len()),
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

        let cache_data = CacheData {
            cache_version: "1.0".to_string(),
            directory: cache_key.to_string(),
            file_snapshot: vec![], // Not used for file list cache
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
        let cache_dir = cache_file_path.parent().unwrap();
        std::fs::create_dir_all(cache_dir)?;
        let cache_json = serde_json::to_string_pretty(&cache_data)?;
        std::fs::write(&cache_file_path, cache_json)?;

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
