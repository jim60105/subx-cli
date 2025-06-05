use crate::services::ai::{AIProvider, AnalysisRequest, ContentSample};
use std::path::Path;

use crate::core::matcher::{FileDiscovery, FilenameAnalyzer, MediaFile, MediaFileType};
use crate::Result;

/// 檔案匹配引擎配置
#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub confidence_threshold: f32,
    pub max_sample_length: usize,
    pub enable_content_analysis: bool,
    pub backup_enabled: bool,
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
    analyzer: FilenameAnalyzer,
    config: MatchConfig,
}

impl MatchEngine {
    /// 建立匹配引擎，注入 AI 提供者與設定
    pub fn new(ai_client: Box<dyn AIProvider>, config: MatchConfig) -> Self {
        Self {
            ai_client,
            discovery: FileDiscovery::new(),
            analyzer: FilenameAnalyzer::new(),
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

        // 2. 內容採樣
        let content_samples = if self.config.enable_content_analysis {
            self.extract_content_samples(&subtitles).await?
        } else {
            Vec::new()
        };

        // 3. AI 分析請求
        let analysis_request = AnalysisRequest {
            video_files: videos.iter().map(|v| v.name.clone()).collect(),
            subtitle_files: subtitles.iter().map(|s| s.name.clone()).collect(),
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
                language_hint: None, // TODO: 實作語言檢測
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
        format!("{}.{}", video.name, subtitle.extension)
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
}
