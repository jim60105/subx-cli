use tempfile::TempDir;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json;
use toml;

/// 測試用的媒體檔案生成器
pub struct TestMediaGenerator {
    pub temp_dir: TempDir,
}

impl TestMediaGenerator {
    /// 建立新的臨時目錄作為測試工作區
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
        }
    }

    /// 取得臨時目錄路徑
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// 建立測試用的 SRT 字幕檔案
    pub fn create_srt_file(&self, name: &str, entries: &[(&str, &str, &str)]) -> PathBuf {
        let mut content = String::new();
        for (i, (start, end, text)) in entries.iter().enumerate() {
            content.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, start, end, text));
        }
        let path = self.path().join(format!("{}.srt", name));
        fs::write(&path, content).unwrap();
        path
    }

    /// 建立測試用的影片檔案（空檔案）
    pub fn create_video_file(&self, name: &str, extension: &str) -> PathBuf {
        let path = self.path().join(format!("{}.{}", name, extension));
        fs::write(&path, b"").unwrap();
        path
    }

    /// 建立測試用的配置檔案
    pub fn create_config_file(&self, config: &subx_cli::config::Config) -> PathBuf {
        let content = toml::to_string_pretty(config).unwrap();
        let path = self.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }
}

/// 測試用的 AI 回應模擬器
pub struct MockAIResponses;

impl MockAIResponses {
    pub fn successful_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.95,
                    "match_factors": ["檔名相似", "內容匹配"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "檔名模式高度相似"
        })
    }

    pub fn low_confidence_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.3,
                    "match_factors": ["部分檔名相似"]
                }
            ],
            "confidence": 0.3,
            "reasoning": "匹配度較低"
        })
    }

    pub fn no_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [],
            "confidence": 0.0,
            "reasoning": "找不到合適的匹配"
        })
    }
}

/// 斷言輔助巨集
#[macro_export]
macro_rules! assert_subtitle_entry {
    ($entry:expr, $index:expr, $start:expr, $end:expr, $text:expr) => {
        assert_eq!($entry.index, $index);
        assert_eq!($entry.start_time, std::time::Duration::from_millis($start));
        assert_eq!($entry.end_time, std::time::Duration::from_millis($end));
        assert_eq!($entry.text, $text);
    };
}

pub mod sync_helpers;
