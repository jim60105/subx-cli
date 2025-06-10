//! 編碼檢測整合測試

use std::fs;
use subx_cli::{
    config::TestConfigBuilder,
    core::formats::encoding::{Charset, EncodingConverter, EncodingDetector},
};
use tempfile::tempdir;

mod common;
use common::{SubtitleFormat, SubtitleGenerator, TestFileManager};

#[test]
fn test_subtitle_file_encoding_detection() {
    let _config = TestConfigBuilder::new().build_config();

    let dir = tempdir().unwrap();
    let srt = r#"1
00:00:01,000 --> 00:00:03,000
Hello, World!

2
00:00:04,000 --> 00:00:06,000
測試字幕
"#;
    let path = dir.path().join("test_utf8.srt");
    fs::write(&path, srt).unwrap();

    let detector = EncodingDetector::new().unwrap();
    let info = detector
        .detect_file_encoding(path.to_str().unwrap())
        .unwrap();
    assert_eq!(info.charset, Charset::Utf8);
    assert!(info.confidence > 0.7);
}

#[test]
fn test_end_to_end_encoding_conversion() {
    let _config = TestConfigBuilder::new().build_config();

    let dir = tempdir().unwrap();
    let path = dir.path().join("input.srt");
    let content = "Hello, 世界! Bonjour, monde!";
    fs::write(&path, content).unwrap();

    let detector = EncodingDetector::new().unwrap();
    let info = detector
        .detect_file_encoding(path.to_str().unwrap())
        .unwrap();
    let converter = EncodingConverter::new();
    let result = converter
        .convert_file_to_utf8(path.to_str().unwrap(), &info)
        .unwrap();
    assert_eq!(result.converted_text, content);
    assert!(!result.had_errors);
}

#[tokio::test]
async fn test_encoding_detection_with_generated_files() {
    // 使用新的測試工具產生測試檔案
    let mut file_manager = TestFileManager::new();
    let test_dir = file_manager
        .create_isolated_test_directory("encoding_test")
        .await
        .unwrap();
    let test_dir_path = test_dir.to_path_buf();

    // 產生不同類型的測試檔案
    let generator = SubtitleGenerator::new(SubtitleFormat::Srt)
        .add_entry(1.0, 3.0, "測試字幕 1")
        .add_entry(4.0, 6.0, "Test subtitle 2");

    let subtitle_path = test_dir_path.join("test.srt");
    generator.save_to_file(&subtitle_path).await.unwrap();

    // 測試編碼檢測
    let detector = EncodingDetector::new().unwrap();
    let info = detector
        .detect_file_encoding(subtitle_path.to_str().unwrap())
        .unwrap();

    assert_eq!(info.charset, Charset::Utf8);
    assert!(info.confidence > 0.5);
}

#[test]
fn test_encoding_detection_with_custom_config() {
    // 測試自訂配置的編碼檢測
    let _config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .build_config();

    let dir = tempdir().unwrap();
    let path = dir.path().join("test.srt");
    let content = "Simple test content";
    fs::write(&path, content).unwrap();

    // 使用配置進行編碼檢測
    let detector = EncodingDetector::new().unwrap();
    let info = detector
        .detect_file_encoding(path.to_str().unwrap())
        .unwrap();

    assert_eq!(info.charset, Charset::Utf8);
}
