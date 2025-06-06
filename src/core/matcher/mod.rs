//! 檔案匹配引擎模組
#![allow(dead_code)]

pub mod discovery;
pub mod engine;
// 已移除檔名分析器，簡化匹配邏輯

pub use discovery::{FileDiscovery, MediaFile, MediaFileType};
pub use engine::{MatchConfig, MatchEngine, MatchOperation};
// pub use filename_analyzer::{FilenameAnalyzer, ParsedFilename};
pub mod cache;
