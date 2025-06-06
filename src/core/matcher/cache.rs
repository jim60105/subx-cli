use serde::{Deserialize, Serialize};

/// 檔案快照項目，用於比對目錄檔案狀態
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct SnapshotItem {
    pub name: String,
    pub size: u64,
    pub mtime: u64,
    pub file_type: String,
}

/// 單筆匹配操作快取結構
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpItem {
    pub video_file: String,
    pub subtitle_file: String,
    pub new_subtitle_name: String,
    pub confidence: f32,
    pub reasoning: Vec<String>,
}

/// Dry-run 快取資料結構
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheData {
    pub cache_version: String,
    pub directory: String,
    pub file_snapshot: Vec<SnapshotItem>,
    pub match_operations: Vec<OpItem>,
    pub created_at: u64,
    pub ai_model_used: String,
    pub config_hash: String,
}

impl CacheData {
    /// 從檔案載入快取資料
    pub fn load(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }
}
