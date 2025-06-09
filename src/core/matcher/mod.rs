//! File matching engine module.
#![allow(dead_code)]

pub mod discovery;
pub mod engine;
// 已移除檔名分析器，簡化匹配邏輯

pub use discovery::{FileDiscovery, MediaFile, MediaFileType};
pub use engine::{MatchConfig, MatchEngine, MatchOperation};
// pub use filename_analyzer::{FilenameAnalyzer, ParsedFilename};
pub mod cache;
use crate::Result;
use crate::core::language::{LanguageDetector, LanguageInfo};
use crate::error::SubXError;
use std::path::{Path, PathBuf};

/// Extended file information structure with relative paths and context metadata.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File name without directory path.
    pub name: String,
    /// Path relative to the search root directory.
    pub relative_path: String,
    /// Absolute file system path.
    pub full_path: PathBuf,
    /// Name of the parent directory containing the file.
    pub directory: String,
    /// Directory depth relative to the root search path.
    pub depth: usize,
    /// Detected language code information (e.g., "tc", "sc", "en").
    pub language: Option<LanguageInfo>,
}

impl FileInfo {
    /// Construct a new `FileInfo`, given the full path and the search root.
    pub fn new(full_path: PathBuf, root_path: &Path) -> Result<Self> {
        // 統一使用 Unix 風格分隔符，確保跨平台一致性
        let relative_path = full_path
            .strip_prefix(root_path)
            .map_err(|e| SubXError::Other(e.into()))?
            .to_string_lossy()
            .replace('\\', "/");
        let name = full_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let directory = full_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        // 使用 '/' 作為分隔符計算深度
        let depth = relative_path.matches('/').count();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_info_creation() -> Result<()> {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let file_path = root.join("season1").join("episode1.mp4");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, b"").unwrap();

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

        // 測試多層目錄
        let file_path = root
            .join("series")
            .join("season1")
            .join("episodes")
            .join("ep01.mp4");
        std::fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        std::fs::write(&file_path, b"").unwrap();

        let info = FileInfo::new(file_path.clone(), root)?;
        assert_eq!(info.relative_path, "series/season1/episodes/ep01.mp4");
        assert_eq!(info.depth, 3);

        Ok(())
    }
}
