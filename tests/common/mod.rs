use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use toml;

/// Test media file generator
pub struct TestMediaGenerator {
    pub temp_dir: TempDir,
}

impl TestMediaGenerator {
    /// Create a new temporary directory as test workspace
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
        }
    }

    /// Get the temporary directory path
    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create test SRT subtitle file
    pub fn create_srt_file(&self, name: &str, entries: &[(&str, &str, &str)]) -> PathBuf {
        let mut content = String::new();
        for (i, (start, end, text)) in entries.iter().enumerate() {
            content.push_str(&format!("{}\n{} --> {}\n{}\n\n", i + 1, start, end, text));
        }
        let path = self.path().join(format!("{}.srt", name));
        fs::write(&path, content).unwrap();
        path
    }

    /// Create test video file (empty file)
    pub fn create_video_file(&self, name: &str, extension: &str) -> PathBuf {
        let path = self.path().join(format!("{}.{}", name, extension));
        fs::write(&path, b"").unwrap();
        path
    }

    /// Create test configuration file
    pub fn create_config_file(&self, config: &subx_cli::config::Config) -> PathBuf {
        let content = toml::to_string_pretty(config).unwrap();
        let path = self.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }
}

/// Test AI response simulator
pub struct MockAIResponses;

impl MockAIResponses {
    pub fn successful_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.95,
                    "match_factors": ["filename similarity", "content match"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "filename pattern highly similar"
        })
    }

    pub fn low_confidence_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [
                {
                    "video_file": "video.mp4",
                    "subtitle_file": "subtitle.srt",
                    "confidence": 0.3,
                    "match_factors": ["partial filename similarity"]
                }
            ],
            "confidence": 0.3,
            "reasoning": "low match confidence"
        })
    }

    pub fn no_match_response() -> serde_json::Value {
        serde_json::json!({
            "matches": [],
            "confidence": 0.0,
            "reasoning": "no suitable match found"
        })
    }
}

/// Assertion helper macros
#[macro_export]
macro_rules! assert_subtitle_entry {
    ($entry:expr, $index:expr, $start:expr, $end:expr, $text:expr) => {
        assert_eq!($entry.index, $index);
        assert_eq!($entry.start_time, std::time::Duration::from_millis($start));
        assert_eq!($entry.end_time, std::time::Duration::from_millis($end));
        assert_eq!($entry.text, $text);
    };
}

pub mod parallel_helpers;
pub mod sync_helpers;

// CLI testing helpers with dependency injection support
pub mod cli_helpers;
