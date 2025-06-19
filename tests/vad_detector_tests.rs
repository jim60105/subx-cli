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
    let processor = VadAudioProcessor::new().unwrap();
    let audio_data = processor
        .load_and_prepare_audio_direct(&audio_path)
        .await
        .unwrap();
    let result = detector.detect_speech_from_data(audio_data).await.unwrap();

    // Basic validation based on known characteristics of the audio file
    assert!(
        !result.speech_segments.is_empty(),
        "Should detect at least one speech segment"
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
    let mut vad_config = VadConfig::default();
    vad_config.sensitivity = 0.5;
    let detector = VadSyncDetector::new(vad_config).unwrap();

    let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    let mut subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);
    subtitle.entries.push(SubtitleEntry {
        index: 1,
        start_time: Duration::from_secs_f64(9.797),
        end_time: Duration::from_secs_f64(12.093),
        text: "Files scattered everywhere".to_string(),
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
    let processor = VadAudioProcessor::new().unwrap();
    let audio_data = processor
        .load_and_prepare_audio_direct(&audio_path)
        .await
        .unwrap();
    let audio_data_high = audio_data.clone();
    let audio_data_low = audio_data;

    // High sensitivity
    let mut high_sensitivity_config = VadConfig::default();
    high_sensitivity_config.sensitivity = 0.9;
    let high_sensitivity_detector = LocalVadDetector::new(high_sensitivity_config).unwrap();
    let high_sensitivity_result = high_sensitivity_detector
        .detect_speech_from_data(audio_data_high)
        .await
        .unwrap();

    // Low sensitivity
    let mut low_sensitivity_config = VadConfig::default();
    low_sensitivity_config.sensitivity = 0.1;
    let low_sensitivity_detector = LocalVadDetector::new(low_sensitivity_config).unwrap();
    let low_sensitivity_result = low_sensitivity_detector
        .detect_speech_from_data(audio_data_low)
        .await
        .unwrap();

    // Expect fewer or equal segments with lower sensitivity (允許誤差 1 個)
    let high_count = high_sensitivity_result.speech_segments.len();
    let low_count = low_sensitivity_result.speech_segments.len();
    assert!(
        low_count <= high_count + 1,
        "Lower sensitivity should detect fewer or equal segments (low: {}, high: {})",
        low_count,
        high_count
    );
}

#[tokio::test]
async fn test_vad_audio_processor_invalid_path() {
    let invalid_path = Path::new("/invalid/path/to/audio.mp4");
    let processor = VadAudioProcessor::new().unwrap();
    let result = processor.load_and_prepare_audio_direct(invalid_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_vad_detector_empty_audio() {
    let mut helper = common::cli_helpers::CLITestHelper::new();
    let empty_audio_path = helper.create_subtitle_file("empty.wav", "").await.unwrap();

    let vad_config = VadConfig::default();
    let detector = LocalVadDetector::new(vad_config).unwrap();
    let processor = VadAudioProcessor::new().unwrap();
    let audio_data_result = processor
        .load_and_prepare_audio_direct(&empty_audio_path)
        .await;
    assert!(
        audio_data_result.is_ok(),
        "Audio processor should return Ok (even if empty)"
    );
    let audio_data = audio_data_result.unwrap();
    let result = detector.detect_speech_from_data(audio_data).await;

    // This should fail because the audio is empty/invalid
    assert!(result.is_err());
}

#[test]
fn test_chunk_size_calculation() {
    let vad_config = VadConfig::default();
    let detector = LocalVadDetector::new(vad_config).unwrap();
    assert_eq!(detector.calculate_chunk_size(8000), 256);
    assert_eq!(detector.calculate_chunk_size(16000), 512);
    let result = std::panic::catch_unwind(|| {
        detector.calculate_chunk_size(48000);
    });
    assert!(result.is_err(), "calculate_chunk_size(48000) should panic");
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
