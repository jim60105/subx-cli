use std::time::Duration;
use subx_cli::config::VadConfig;
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::services::vad::{LocalVadDetector, VadAudioProcessor, VadSyncDetector};
use tempfile::TempDir;

#[tokio::test]
#[ignore = "Requires audio processing environment, may fail in some CI environments"]
async fn test_vad_sync_detection_integration() {
    let temp_dir = TempDir::new().unwrap();

    // Create test audio file
    let audio_path = temp_dir.path().join("test_video.wav");
    create_test_audio_with_timed_speech(&audio_path);

    // Create test subtitle
    let subtitle = create_test_subtitle_with_known_timing();

    // Create VAD sync detector
    let config = VadConfig::default();
    let detector = VadSyncDetector::new(config).unwrap();

    // Execute sync detection
    let result = detector
        .detect_sync_offset(
            &audio_path,
            &subtitle,
            30, // 30-second analysis window
        )
        .await
        .unwrap();

    // Verify results
    assert!(result.confidence > 0.5);
    assert_eq!(
        result.method_used,
        subx_cli::core::sync::SyncMethod::LocalVad
    );
    assert!(result.additional_info.is_some());
}

#[tokio::test]
#[ignore = "Requires audio processing environment, may fail in some CI environments"]
async fn test_vad_audio_format_compatibility() {
    let temp_dir = TempDir::new().unwrap();

    // Test different audio formats and parameters
    let test_cases = vec![
        (8000, 1),  // 8kHz mono
        (16000, 1), // 16kHz mono
        (44100, 1), // 44.1kHz mono
        (44100, 2), // 44.1kHz stereo
    ];

    let config = VadConfig::default();
    let detector = LocalVadDetector::new(config).unwrap();

    for (sample_rate, channels) in test_cases {
        let audio_path = temp_dir
            .path()
            .join(&format!("test_{}_{}.wav", sample_rate, channels));
        create_test_audio_with_format(&audio_path, sample_rate, channels);

        let processor = VadAudioProcessor::new().unwrap();
        let audio_data = processor
            .load_and_prepare_audio_direct(&audio_path)
            .await
            .unwrap();
        let result = detector.detect_speech_from_data(audio_data).await;
        assert!(
            result.is_ok(),
            "Failed for format: {}Hz, {} channels",
            sample_rate,
            channels
        );
        let vad_result = result.unwrap();
        // Verify that the original sample rate is preserved and audio is converted to mono
        assert_eq!(vad_result.audio_info.sample_rate, sample_rate);
        assert_eq!(vad_result.audio_info.channels, 1);
    }
}

fn create_test_audio_with_timed_speech(path: &std::path::Path) {
    // Create test audio with speech at known time points
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let duration_seconds = 60; // 1 minute
    let total_samples = 16000 * duration_seconds;

    for i in 0..total_samples {
        let t = i as f32 / 16000.0;

        // Create speech around 30 seconds (analysis window center)
        let sample = if t >= 29.5 && t <= 32.0 {
            // Speech signal
            (0.4 * (2.0 * std::f32::consts::PI * 300.0 * t).sin()
                + 0.3 * (2.0 * std::f32::consts::PI * 600.0 * t).sin())
                * 32767.0
        } else {
            // Background noise
            ((t * 7919.0).sin() * 0.005) * 32767.0
        };

        writer.write_sample(sample as i16).unwrap();
    }

    writer.finalize().unwrap();
}

fn create_test_subtitle_with_known_timing() -> Subtitle {
    Subtitle {
        entries: vec![SubtitleEntry::new(
            1,
            Duration::from_secs(30), // First sentence at 30 seconds
            Duration::from_secs(32),
            "Test dialogue".to_string(),
        )],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    }
}

fn create_test_audio_with_format(path: &std::path::Path, sample_rate: u32, channels: u16) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let duration_seconds = 2;
    let total_samples = sample_rate * duration_seconds;

    for i in 0..total_samples {
        let t = i as f32 / sample_rate as f32;

        for _ch in 0..channels {
            let sample = if t >= 0.5 && t <= 1.5 {
                // Speech signal
                (0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) * 32767.0
            } else {
                // Silence
                0.0
            };

            writer.write_sample(sample as i16).unwrap();
        }
    }

    writer.finalize().unwrap();
}
