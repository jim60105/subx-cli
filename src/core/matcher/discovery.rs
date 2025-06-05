use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::Result;

/// 媒體檔案類型
#[derive(Debug, Clone)]
pub struct MediaFile {
    pub path: PathBuf,
    pub file_type: MediaFileType,
    pub size: u64,
    pub name: String,
    pub extension: String,
}

impl Default for FileDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// 媒體檔案類型枚舉
#[derive(Debug, Clone)]
pub enum MediaFileType {
    Video,
    Subtitle,
}

/// 檔案探索器
pub struct FileDiscovery {
    video_extensions: Vec<String>,
    subtitle_extensions: Vec<String>,
}

impl FileDiscovery {
    /// 建立新的檔案探索器，預設辨識常見影片與字幕副檔名
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

    /// 掃描指定目錄，並回傳所有符合媒體類型的檔案清單
    pub fn scan_directory(&self, path: &Path, recursive: bool) -> Result<Vec<MediaFile>> {
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

    /// 根據副檔名判別媒體檔案類型，並擷取基本屬性
    fn classify_file(&self, path: &Path) -> Result<Option<MediaFile>> {
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
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
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
