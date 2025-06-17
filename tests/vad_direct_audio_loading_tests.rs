//! 測試：VAD 直接音訊載入能力，支援多種格式（暫時標記為忽略）
use subx_cli::services::vad::audio_loader::DirectAudioLoader;

#[tokio::test]
#[ignore]
async fn test_direct_mp4_loading() {
    // TODO: 實作 MP4 直接載入測試
    let loader = DirectAudioLoader::new().unwrap();
    let result = loader.load_audio_samples("tests/fixtures/test.mp4");
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_direct_mkv_loading() {
    // TODO: 實作 MKV 直接載入測試
    let loader = DirectAudioLoader::new().unwrap();
    let result = loader.load_audio_samples("tests/fixtures/test.mkv");
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_format_comparison() {
    // TODO: 比較直接載入與轉碼載入的結果一致性
    assert!(true);
}

#[tokio::test]
#[ignore]
async fn test_performance_improvement() {
    // TODO: 性能比較測試
    assert!(true);
}
