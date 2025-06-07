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

// 單元測試: FileDiscovery 檔案匹配邏輯
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
        assert!(!files.iter().any(|f| f.name == "episode1"));
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
        assert!(files.iter().any(|f| f.name == "episode1"));
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
        let vf = disco.classify_file(&v).unwrap().unwrap();
        assert!(matches!(vf.file_type, MediaFileType::Video));
        assert_eq!(vf.name, "t");
        let sf = disco.classify_file(&s).unwrap().unwrap();
        assert!(matches!(sf.file_type, MediaFileType::Subtitle));
        assert_eq!(sf.name, "t");
        let none = disco.classify_file(&x).unwrap();
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
