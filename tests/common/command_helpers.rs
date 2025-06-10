use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
use subx_cli::config::Config;

/// 建立測試快取檔案，回傳 match_cache.json 路徑
pub async fn create_test_cache_files(dir: &TempDir) -> PathBuf {
    let cache_dir = dir.path().join("subx");
    fs::create_dir_all(&cache_dir).unwrap();
    let path = cache_dir.join("match_cache.json");
    fs::write(&path, "{}").unwrap();
    path
}

/// 建立測試配置檔案，回傳 config.toml 路徑
pub async fn create_test_config(dir: &TempDir) -> PathBuf {
    let config = Config::default();
    let toml = toml::to_string_pretty(&config).unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, toml).unwrap();
    path
}

/// 建立測試 UTF-8 編碼的 SRT 檔案
pub async fn create_utf8_subtitle_file(dir: &TempDir) -> PathBuf {
    let path = dir.path().join("test.srt");
    let content = "1\n00:00:01,000 --> 00:00:02,000\nHello World";
    fs::write(&path, content).unwrap();
    path
}
