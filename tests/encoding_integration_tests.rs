use serial_test::serial;
use std::fs;
use subx_cli::{
    config::reset_global_config_manager,
    core::formats::encoding::{Charset, EncodingConverter, EncodingDetector},
    init_config_manager,
};
use tempfile::tempdir;

#[test]
#[serial]
fn test_subtitle_file_encoding_detection() {
    reset_global_config_manager();
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
    init_config_manager().unwrap();
    let detector = EncodingDetector::new().unwrap();
    let info = detector
        .detect_file_encoding(path.to_str().unwrap())
        .unwrap();
    assert_eq!(info.charset, Charset::Utf8);
    assert!(info.confidence > 0.7);
    reset_global_config_manager();
}

#[test]
#[serial]
fn test_end_to_end_encoding_conversion() {
    reset_global_config_manager();
    let dir = tempdir().unwrap();
    let path = dir.path().join("input.srt");
    let content = "Hello, 世界! Bonjour, monde!";
    fs::write(&path, content).unwrap();
    init_config_manager().unwrap();
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
    reset_global_config_manager();
}
