//! aus 整合測試

use std::path::PathBuf;
use subx_cli::config::TestConfigBuilder;
use subx_cli::services::audio::AusAudioAnalyzer;

mod common;
use common::{file_managers::TestFileManager, mock_generators::AudioMockGenerator};

#[tokio::test]
async fn test_aus_audio_loading() {
    let _config = TestConfigBuilder::new()
        .with_audio_sample_rate(16000)
        .build_config();

    let analyzer = AusAudioAnalyzer::new(16000);

    // 使用 AudioMockGenerator 建立測試音訊
    let audio_gen = AudioMockGenerator::new(44100).with_duration(5.0);
    let mut file_manager = TestFileManager::new();
    let test_dir = file_manager
        .create_isolated_test_directory("audio_test")
        .await
        .unwrap();
    let test_audio_path = test_dir.join("test_audio.wav");

    let _metadata = audio_gen
        .generate_dialogue_audio(&test_audio_path)
        .await
        .unwrap();

    let audio_file = analyzer.load_audio_file(&test_audio_path).await.unwrap();
    assert!(audio_file.sample_rate > 0);
    assert!(audio_file.duration > 0.0);
}

#[tokio::test]
async fn test_aus_envelope_extraction() {
    let _config = TestConfigBuilder::new()
        .with_audio_sample_rate(16000)
        .build_config();

    let analyzer = AusAudioAnalyzer::new(16000);

    // 使用 AudioMockGenerator 建立測試音訊
    let audio_gen = AudioMockGenerator::new(44100).with_duration(3.0);
    let mut file_manager = TestFileManager::new();
    let test_dir = file_manager
        .create_isolated_test_directory("envelope_test")
        .await
        .unwrap();
    let test_audio_path = test_dir.join("test_audio.wav");

    let _metadata = audio_gen
        .generate_dialogue_audio(&test_audio_path)
        .await
        .unwrap();

    let envelope = analyzer.extract_envelope(&test_audio_path).await.unwrap();
    assert!(!envelope.samples.is_empty());
    assert!(envelope.duration > 0.0);
}

#[tokio::test]
async fn test_aus_feature_analysis() {
    let _config = TestConfigBuilder::new()
        .with_audio_sample_rate(16000)
        .build_config();

    let analyzer = AusAudioAnalyzer::new(16000);
    let test_audio = PathBuf::from("tests/fixtures/test_audio.wav");
    if test_audio.exists() {
        let audio_file = analyzer.load_audio_file(&test_audio).await.unwrap();
        let features = analyzer.analyze_audio_features(&audio_file).await.unwrap();
        assert!(!features.frames.is_empty());
        for frame in &features.frames {
            assert!(frame.spectral_centroid >= 0.0);
            assert!(frame.spectral_entropy >= 0.0);
            assert!(frame.zero_crossing_rate >= 0.0);
        }
    }
}
