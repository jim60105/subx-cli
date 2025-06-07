//! 檔案匹配引擎模組
#![allow(dead_code)]

pub mod discovery;
pub mod engine;
// 已移除檔名分析器，簡化匹配邏輯

pub use discovery::{FileDiscovery, MediaFile, MediaFileType};
pub use engine::{MatchConfig, MatchEngine, MatchOperation};
// pub use filename_analyzer::{FilenameAnalyzer, ParsedFilename};
pub mod cache;
use crate::core::language::{LanguageDetector, LanguageInfo};
use crate::error::SubXError;
use crate::Result;
use std::path::{Path, PathBuf};

/// 增強的檔案資訊結構，包含相對路徑與目錄上下文
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 檔案名稱（不含路徑）
    pub name: String,
    /// 相對於搜尋根目錄的路徑
    pub relative_path: String,
    /// 完整的絕對路徑
    pub full_path: PathBuf,
    /// 所在目錄名稱
    pub directory: String,
    /// 目錄深度（相對於根目錄）
    pub depth: usize,
    /// 偵測出的語言編碼（如 tc、sc、en）
    pub language: Option<LanguageInfo>,
}

impl FileInfo {
    /// 建立 FileInfo，root_path 為搜尋根目錄
    pub fn new(full_path: PathBuf, root_path: &Path) -> Result<Self> {
        let relative_path = full_path
            .strip_prefix(root_path)
            .map_err(|e| SubXError::Other(e.into()))?
            .to_string_lossy()
            .to_string();
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
        let depth = relative_path.matches(std::path::MAIN_SEPARATOR).count();
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
}
