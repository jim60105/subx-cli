# Product Backlog #06: 檔案匹配引擎

## 領域範圍
檔案發現、AI 智慧匹配、檔案重命名、Dry-run 模式

## 完成項目

### 1. 檔案發現系統
- [ ] 遞歸掃描資料夾結構
- [ ] 影片檔案類型識別 (mp4, mkv, avi, etc.)
- [ ] 字幕檔案類型識別 (srt, ass, vtt, sub)
- [ ] 檔案過濾和排除規則

### 2. 檔名分析器
- [ ] 季集資訊提取 (S01E01, 1x01, etc.)
- [ ] 劇名和標題解析
- [ ] 年份和版本資訊識別
- [ ] 檔名標準化處理

### 3. 內容採樣器
- [ ] 字幕內容預覽提取
- [ ] 語言檢測
- [ ] 內容長度和品質評估
- [ ] 採樣策略優化

### 4. AI 匹配協調器
- [ ] 批次請求組織
- [ ] AI 服務呼叫管理
- [ ] 結果聚合和排序
- [ ] 信心度閾值過濾

### 5. 檔案操作管理
- [ ] Dry-run 模式預覽
- [ ] 安全的檔案重命名
- [ ] 備份機制
- [ ] 衝突解決策略

### 6. 進度和日誌
- [ ] 操作進度追蹤
- [ ] 詳細日誌記錄
- [ ] 錯誤報告和恢復
- [ ] 統計資訊收集

## 技術設計

### 檔案發現系統
```rust
// src/core/matcher/discovery.rs
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct MediaFile {
    pub path: PathBuf,
    pub file_type: MediaFileType,
    pub size: u64,
    pub name: String,
    pub extension: String,
}

#[derive(Debug, Clone)]
pub enum MediaFileType {
    Video,
    Subtitle,
}

pub struct FileDiscovery {
    video_extensions: Vec<String>,
    subtitle_extensions: Vec<String>,
}

impl FileDiscovery {
    pub fn new() -> Self {
        Self {
            video_extensions: vec![
                "mp4".to_string(), "mkv".to_string(), "avi".to_string(),
                "mov".to_string(), "wmv".to_string(), "flv".to_string(),
                "m4v".to_string(), "webm".to_string(),
            ],
            subtitle_extensions: vec![
                "srt".to_string(), "ass".to_string(), "vtt".to_string(),
                "sub".to_string(), "ssa".to_string(), "idx".to_string(),
            ],
        }
    }
    
    pub fn scan_directory(&self, path: &Path, recursive: bool) -> crate::Result<Vec<MediaFile>> {
        let mut files = Vec::new();
        
        let walker = if recursive {
            WalkDir::new(path).into_iter()
        } else {
            WalkDir::new(path).max_depth(1).into_iter()
        };
        
        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(media_file) = self.classify_file(path)? {
                    files.push(media_file);
                }
            }
        }
        
        Ok(files)
    }
    
    fn classify_file(&self, path: &Path) -> crate::Result<Option<MediaFile>> {
        let extension = path.extension()
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
        let name = path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        
        Ok(Some(MediaFile {
            path: path.to_path_buf(),
            file_type,
            size: metadata.len(),
            name,
            extension,
        }))
    }
}
```

### 檔名分析器
```rust
// src/core/matcher/filename_analyzer.rs
use regex::Regex;

#[derive(Debug, Clone)]
pub struct ParsedFilename {
    pub title: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub year: Option<u32>,
    pub quality: Option<String>,
    pub language: Option<String>,
    pub group: Option<String>,
}

pub struct FilenameAnalyzer {
    season_episode_patterns: Vec<Regex>,
    year_pattern: Regex,
    quality_pattern: Regex,
}

impl FilenameAnalyzer {
    pub fn new() -> Self {
        Self {
            season_episode_patterns: vec![
                Regex::new(r"[Ss](\d{1,2})[Ee](\d{1,3})").unwrap(),
                Regex::new(r"(\d{1,2})x(\d{1,3})").unwrap(),
                Regex::new(r"Season\s*(\d{1,2}).*Episode\s*(\d{1,3})").unwrap(),
                Regex::new(r"第(\d{1,2})季.*第(\d{1,3})集").unwrap(),
            ],
            year_pattern: Regex::new(r"\b(19|20)\d{2}\b").unwrap(),
            quality_pattern: Regex::new(r"\b(720p|1080p|4K|2160p|BluRay|WEB-DL|HDRip)\b").unwrap(),
        }
    }
    
    pub fn parse(&self, filename: &str) -> ParsedFilename {
        let mut parsed = ParsedFilename {
            title: String::new(),
            season: None,
            episode: None,
            year: None,
            quality: None,
            language: None,
            group: None,
        };
        
        // 提取季集資訊
        for pattern in &self.season_episode_patterns {
            if let Some(captures) = pattern.captures(filename) {
                parsed.season = captures.get(1).and_then(|m| m.as_str().parse().ok());
                parsed.episode = captures.get(2).and_then(|m| m.as_str().parse().ok());
                break;
            }
        }
        
        // 提取年份
        if let Some(year_match) = self.year_pattern.find(filename) {
            parsed.year = year_match.as_str().parse().ok();
        }
        
        // 提取品質資訊
        if let Some(quality_match) = self.quality_pattern.find(filename) {
            parsed.quality = Some(quality_match.as_str().to_string());
        }
        
        // 提取標題（移除季集資訊後的主要部分）
        parsed.title = self.extract_title(filename, &parsed);
        
        parsed
    }
    
    fn extract_title(&self, filename: &str, parsed: &ParsedFilename) -> String {
        let mut title = filename.to_string();
        
        // 移除季集模式
        for pattern in &self.season_episode_patterns {
            title = pattern.replace(&title, "").to_string();
        }
        
        // 移除年份、品質等資訊
        title = self.year_pattern.replace(&title, "").to_string();
        title = self.quality_pattern.replace(&title, "").to_string();
        
        // 清理和標準化
        title = title.replace(&['.', '_', '-'], " ");
        title = title.trim().to_string();
        
        // 移除多餘空格
        while title.contains("  ") {
            title = title.replace("  ", " ");
        }
        
        title
    }
}
```

### 匹配引擎主體
```rust
// src/core/matcher/engine.rs
use crate::services::ai::AIProvider;
use std::path::Path;

pub struct MatchEngine {
    ai_client: Box<dyn AIProvider>,
    discovery: FileDiscovery,
    analyzer: FilenameAnalyzer,
    config: MatchConfig,
}

#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub confidence_threshold: f32,
    pub max_sample_length: usize,
    pub enable_content_analysis: bool,
    pub backup_enabled: bool,
}

#[derive(Debug)]
pub struct MatchOperation {
    pub video_file: MediaFile,
    pub subtitle_file: MediaFile,
    pub new_subtitle_name: String,
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

impl MatchEngine {
    pub fn new(ai_client: Box<dyn AIProvider>, config: MatchConfig) -> Self {
        Self {
            ai_client,
            discovery: FileDiscovery::new(),
            analyzer: FilenameAnalyzer::new(),
            config,
        }
    }
    
    pub async fn match_files(&self, path: &Path, recursive: bool) -> crate::Result<Vec<MatchOperation>> {
        // 1. 檔案發現
        let files = self.discovery.scan_directory(path, recursive)?;
        
        let videos: Vec<_> = files.iter()
            .filter(|f| matches!(f.file_type, MediaFileType::Video))
            .collect();
        
        let subtitles: Vec<_> = files.iter()
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
        
        // 3. AI 分析
        let analysis_request = crate::services::ai::AnalysisRequest {
            video_files: videos.iter().map(|v| v.name.clone()).collect(),
            subtitle_files: subtitles.iter().map(|s| s.name.clone()).collect(),
            content_samples,
        };
        
        let match_result = self.ai_client.analyze_content(analysis_request).await?;
        
        // 4. 轉換為操作列表
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
    
    async fn extract_content_samples(&self, subtitles: &[&MediaFile]) -> crate::Result<Vec<crate::services::ai::ContentSample>> {
        let mut samples = Vec::new();
        
        for subtitle in subtitles {
            let content = std::fs::read_to_string(&subtitle.path)?;
            let preview = self.create_content_preview(&content);
            
            samples.push(crate::services::ai::ContentSample {
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
    
    pub async fn execute_operations(&self, operations: &[MatchOperation], dry_run: bool) -> crate::Result<()> {
        for operation in operations {
            if dry_run {
                println!("預覽: {} -> {}", 
                    operation.subtitle_file.name, 
                    operation.new_subtitle_name
                );
            } else {
                self.rename_file(operation).await?;
            }
        }
        
        Ok(())
    }
    
    async fn rename_file(&self, operation: &MatchOperation) -> crate::Result<()> {
        let old_path = &operation.subtitle_file.path;
        let new_path = old_path.with_file_name(&operation.new_subtitle_name);
        
        // 備份
        if self.config.backup_enabled {
            let backup_path = old_path.with_extension(
                format!("{}.backup", operation.subtitle_file.extension)
            );
            std::fs::copy(old_path, backup_path)?;
        }
        
        // 重命名
        std::fs::rename(old_path, new_path)?;
        
        Ok(())
    }
}
```

## 驗收標準
1. 檔案發現功能準確且高效
2. 檔名分析覆蓋常見格式
3. AI 匹配結果符合預期
4. Dry-run 模式正確預覽
5. 檔案操作安全可靠

## 估計工時
5-6 天

## 相依性
- 依賴 Backlog #04 (字幕格式解析引擎)
- 依賴 Backlog #05 (AI 服務整合)

## 風險評估
- 中風險：檔名解析複雜度高
- 注意事項：檔案操作安全性、邊界情況處理
