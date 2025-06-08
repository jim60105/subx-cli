//! aus 整合測試

use std::path::PathBuf;
use subx_cli::services::audio::AusAudioAnalyzer;

#[tokio::test]
async fn test_aus_audio_loading() {
    let analyzer = AusAudioAnalyzer::new(16000);
    let test_audio = PathBuf::from("tests/fixtures/test_audio.wav");
    if test_audio.exists() {
        let audio_file = analyzer.load_audio_file(&test_audio).await.unwrap();
        assert!(audio_file.sample_rate() > 0);
        assert!(audio_file.duration_seconds() > 0.0);
    }
}

#[tokio::test]
async fn test_aus_envelope_extraction() {
    let analyzer = AusAudioAnalyzer::new(16000);
    let test_audio = PathBuf::from("tests/fixtures/test_audio.wav");
    if test_audio.exists() {
        let envelope = analyzer.extract_envelope_v2(&test_audio).await.unwrap();
        assert!(!envelope.samples.is_empty());
        assert!(envelope.duration > 0.0);
    }
}

#[tokio::test]
async fn test_aus_feature_analysis() {
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
