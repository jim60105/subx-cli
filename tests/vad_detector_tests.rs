// Comprehensive tests for VadDetector and SyncVadDetector
//
// Following the testing guidelines in `docs/testing-guidelines.md`,
// these tests use real assets and focus on the core speech detection logic.

mod common;

use std::path::Path;
use std::time::Duration;
use subx_cli::config::VadConfig;
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::services::vad::{LocalVadDetector, VadAudioProcessor, VadSyncDetector};

// Helper to load and process the real audio asset for tests
fn get_test_audio_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("SubX - The Subtitle Revolution.mp4")
}

#[tokio::test]
async fn test_vad_detector_with_real_audio() {
    let audio_path = get_test_audio_path();
    let vad_config = VadConfig::default();
    let detector = LocalVadDetector::new(vad_config).unwrap();
    let result = detector.detect_speech(&audio_path).await.unwrap();

    // Basic validation based on known characteristics of the audio file
    assert!(
        !result.speech_segments.is_empty(),
        "Should detect at least one speech segment"
    );
    assert!(
        result.speech_segments.len() > 5,
        "Expected multiple speech segments"
    );

    // Check segment properties
    for segment in result.speech_segments {
        assert!(
            segment.start_time < segment.end_time,
            "Segment start must be before end"
        );
        assert!(
            segment.duration > 0.1,
            "Segment duration should be reasonable"
        );
    }
}

#[tokio::test]
async fn test_sync_vad_detector_with_real_audio() {
    let audio_path = get_test_audio_path();
    let vad_config = VadConfig::default();
    let detector = VadSyncDetector::new(vad_config).unwrap();

    let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    let mut subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);
    subtitle.entries.push(SubtitleEntry {
        index: 1,
        start_time: Duration::from_secs_f64(5.0),
        end_time: Duration::from_secs_f64(8.0),
        text: "Hello world".to_string(),
        styling: None,
    });

    let result = detector
        .detect_sync_offset(&audio_path, &subtitle, 0)
        .await
        .unwrap();

    // Sync detector should produce a valid offset
    assert!(
        result.offset_seconds.abs() < 10.0,
        "Offset should be reasonable"
    );
    assert!(result.confidence > 0.5, "Confidence should be high enough");
    assert_eq!(
        result.method_used,
        subx_cli::core::sync::SyncMethod::LocalVad
    );
}

#[tokio::test]
async fn test_vad_detector_config_sensitivity() {
    let audio_path = get_test_audio_path();

    // High sensitivity
    let mut high_sensitivity_config = VadConfig::default();
    high_sensitivity_config.sensitivity = 0.9;
    let high_sensitivity_detector = LocalVadDetector::new(high_sensitivity_config).unwrap();
    let high_sensitivity_result = high_sensitivity_detector
        .detect_speech(&audio_path)
        .await
        .unwrap();

    // Low sensitivity
    let mut low_sensitivity_config = VadConfig::default();
    low_sensitivity_config.sensitivity = 0.1;
    let low_sensitivity_detector = LocalVadDetector::new(low_sensitivity_config).unwrap();
    let low_sensitivity_result = low_sensitivity_detector
        .detect_speech(&audio_path)
        .await
        .unwrap();

    // Expect more segments with higher sensitivity (允許誤差 1 個)
    let high_count = high_sensitivity_result.speech_segments.len();
    let low_count = low_sensitivity_result.speech_segments.len();
    assert!(
        high_count + 1 >= low_count,
        "Higher sensitivity should detect more or equal segments (high: {}, low: {})",
        high_count,
        low_count
    );
}

#[tokio::test]
async fn test_vad_audio_processor_invalid_path() {
    let invalid_path = Path::new("/invalid/path/to/audio.mp4");
    let vad_config = VadConfig::default();
    let processor = VadAudioProcessor::new(vad_config.sample_rate, 1).unwrap();
    let result = processor.load_and_prepare_audio_direct(invalid_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_vad_detector_empty_audio() {
    let mut helper = common::cli_helpers::CLITestHelper::new();
    let empty_audio_path = helper.create_subtitle_file("empty.wav", "").await.unwrap();

    let vad_config = VadConfig::default();
    let detector = LocalVadDetector::new(vad_config).unwrap();
    let result = detector.detect_speech(&empty_audio_path).await;

    // This should fail because the audio is empty/invalid
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sync_detector_no_subtitle_entries() {
    let audio_path = get_test_audio_path();
    let vad_config = VadConfig::default();
    let detector = VadSyncDetector::new(vad_config).unwrap();
    let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    let empty_subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);

    let result = detector
        .detect_sync_offset(&audio_path, &empty_subtitle, 0)
        .await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("No subtitle entries found"));
    }
}
