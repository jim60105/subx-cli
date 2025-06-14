use std::fs;
use std::path::Path;
use subx_cli::services::whisper::AudioSegmentExtractor;
use tempfile::TempDir;

#[tokio::test]
#[ignore = "需要音訊處理環境，在某些 CI 環境中可能失敗"]
async fn test_audio_segment_extraction() {
    let temp = TempDir::new().unwrap();
    let audio_path = temp.path().join("test.wav");
    // 建立簡易 WAV 檔頭模擬
    fs::write(&audio_path, b"RIFF....WAVE").unwrap();

    let extractor = AudioSegmentExtractor::new().unwrap();
    let seg = extractor
        .extract_segment(&audio_path, std::time::Duration::from_secs(10), 30)
        .await
        .unwrap();
    assert!(seg.exists());
}

#[tokio::test]
#[ignore]
async fn test_whisper_sync_detection_integration() {
    // 需提供 OPENAI_API_KEY 與測試檔案
    let api_key = std::env::var("OPENAI_API_KEY").expect("API key missing");
    let detector = subx_cli::services::whisper::WhisperSyncDetector::new(
        api_key,
        "https://api.openai.com/v1".to_string(),
        subx_cli::config::WhisperConfig::default(),
    )
    .unwrap();

    let format_manager = subx_cli::core::formats::manager::FormatManager::new();
    let subtitle = format_manager
        .load_subtitle(Path::new("tests/data/test.srt"))
        .unwrap();

    let result = detector
        .detect_sync_offset(Path::new("tests/data/test.wav"), &subtitle, 30)
        .await;
    assert!(result.is_ok());
}
