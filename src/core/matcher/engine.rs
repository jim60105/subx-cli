use crate::services::ai::{AIProvider, AnalysisRequest, ContentSample};
use std::path::Path;

use crate::core::language::LanguageDetector;
use crate::core::matcher::cache::{CacheData, OpItem, SnapshotItem};
use crate::core::matcher::{FileDiscovery, MediaFile, MediaFileType};
use crate::Result;

use crate::config::Config;
use crate::error::SubXError;
use dirs;
use md5;
use serde_json;
use toml;

/// 檔案匹配引擎配置
#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub confidence_threshold: f32,
    pub max_sample_length: usize,
    pub enable_content_analysis: bool,
    pub backup_enabled: bool,
}

#[cfg(test)]
mod language_name_tests {
    use super::*;
    use crate::core::language::LanguageSource;
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
            path: PathBuf::from("movie01.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie01".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
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
            path: PathBuf::from("movie02.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie02".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
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
            path: PathBuf::from("movie03.mp4"),
            file_type: MediaFileType::Video,
            size: 0,
            name: "movie03".to_string(),
            extension: "mp4".to_string(),
        };
        let subtitle = MediaFile {
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

/// 單次匹配操作結果
#[derive(Debug)]
pub struct MatchOperation {
    pub video_file: MediaFile,
    pub subtitle_file: MediaFile,
    pub new_subtitle_name: String,
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

/// 檔案匹配引擎
pub struct MatchEngine {
    ai_client: Box<dyn AIProvider>,
    discovery: FileDiscovery,
    config: MatchConfig,
}

impl MatchEngine {
    /// 建立匹配引擎，注入 AI 提供者與設定
    // analyzer 已移除
    pub fn new(ai_client: Box<dyn AIProvider>, config: MatchConfig) -> Self {
        Self {
            ai_client,
            discovery: FileDiscovery::new(),
            config,
        }
    }

    /// 匹配指定路徑下的影片與字幕檔案，回傳符合閾值的匹配操作
    pub async fn match_files(&self, path: &Path, recursive: bool) -> Result<Vec<MatchOperation>> {
        // 1. 探索檔案
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

        // 2. 嘗試從 Dry-run 快取重用結果
        if let Some(ops) = self.check_cache(path, recursive).await? {
            return Ok(ops);
        }
        // 3. 內容採樣
        let content_samples = if self.config.enable_content_analysis {
            self.extract_content_samples(&subtitles).await?
        } else {
            Vec::new()
        };

        // 4. AI 分析請求
        // 生成 AI 分析請求：在檔名中包含相對路徑與目錄資訊，提升遞迴匹配的準確度
        let video_files: Vec<String> = videos
            .iter()
            .map(|v| {
                let rel = v
                    .path
                    .strip_prefix(path)
                    .unwrap_or(&v.path)
                    .to_string_lossy();
                let dir = v
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                format!("{} (路徑: {}, 目錄: {})", v.name, rel, dir)
            })
            .collect();
        let subtitle_files: Vec<String> = subtitles
            .iter()
            .map(|s| {
                let rel = s
                    .path
                    .strip_prefix(path)
                    .unwrap_or(&s.path)
                    .to_string_lossy();
                let dir = s
                    .path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                format!("{} (路徑: {}, 目錄: {})", s.name, rel, dir)
            })
            .collect();
        let analysis_request = AnalysisRequest {
            video_files,
            subtitle_files,
            content_samples,
        };

        let match_result = self.ai_client.analyze_content(analysis_request).await?;

        // 4. 組裝匹配操作列表
        let mut operations = Vec::new();

        for ai_match in match_result.matches {
            if ai_match.confidence >= self.config.confidence_threshold {
                if let (Some(video), Some(subtitle)) = (
                    videos.iter().find(|v| v.name == ai_match.video_file),
                    subtitles.iter().find(|s| s.name == ai_match.subtitle_file),
                ) {
                    let new_name = self.generate_subtitle_name(video, subtitle);

                    operations.push(MatchOperation {
                        video_file: (*video).clone(),
                        subtitle_file: (*subtitle).clone(),
                        new_subtitle_name: new_name,
                        confidence: ai_match.confidence,
                        reasoning: ai_match.match_factors,
                    });
                }
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

    /// 執行匹配操作，支援 Dry-run 模式
    pub async fn execute_operations(
        &self,
        operations: &[MatchOperation],
        dry_run: bool,
    ) -> Result<()> {
        for op in operations {
            if dry_run {
                println!(
                    "預覽: {} -> {}",
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

        // 備份檔案
        if self.config.backup_enabled {
            let backup_path =
                old_path.with_extension(format!("{}.backup", op.subtitle_file.extension));
            std::fs::copy(old_path, backup_path)?;
        }

        std::fs::rename(old_path, new_path)?;
        Ok(())
    }
    /// 計算指定目錄的檔案快照，用於快取比對
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

    /// 檢查 Dry-run 快取，命中則回傳先前計算的匹配操作
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
                // 重建匹配操作列表
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

    /// 儲存 Dry-run 快取結果
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

    /// 取得快取檔案路徑
    fn get_cache_file_path(&self) -> Result<std::path::PathBuf> {
        let dir = dirs::config_dir().ok_or_else(|| SubXError::config("無法確定快取目錄"))?;
        Ok(dir.join("subx").join("match_cache.json"))
    }

    /// 計算目前配置雜湊，用於快取驗證
    fn calculate_config_hash(&self) -> Result<String> {
        let config = Config::load()?;
        let toml = toml::to_string(&config)
            .map_err(|e| SubXError::config(format!("TOML 序列化錯誤: {}", e)))?;
        Ok(format!("{:x}", md5::compute(toml)))
    }
}
