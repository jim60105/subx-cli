use std::time::Duration;
use subx_cli::config::{TestConfigBuilder, VadConfig};
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::services::vad::{LocalVadDetector, VadSyncDetector};
use tempfile::TempDir;

#[tokio::test]
async fn test_vad_sync_detection_integration() {
    let temp_dir = TempDir::new().unwrap();

    // 建立測試音訊檔案
    let audio_path = temp_dir.path().join("test_video.wav");
    create_test_audio_with_timed_speech(&audio_path);

    // 建立測試字幕
    let subtitle = create_test_subtitle_with_known_timing();

    // 建立 VAD 同步檢測器
    let config = VadConfig::default();
    let detector = VadSyncDetector::new(config).unwrap();

    // 執行同步檢測
    let result = detector
        .detect_sync_offset(
            &audio_path,
            &subtitle,
            30, // 30 秒分析窗口
        )
        .await
        .unwrap();

    // 驗證結果
    assert!(result.confidence > 0.5);
    assert_eq!(
        result.method_used,
        subx_cli::core::sync::SyncMethod::LocalVad
    );
    assert!(result.additional_info.is_some());
}

#[tokio::test]
async fn test_vad_audio_format_compatibility() {
    let temp_dir = TempDir::new().unwrap();

    // 測試不同的音訊格式和參數
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

        let result = detector.detect_speech(&audio_path).await;
        assert!(
            result.is_ok(),
            "Failed for format: {}Hz, {} channels",
            sample_rate,
            channels
        );
    }
}

fn create_test_audio_with_timed_speech(path: &std::path::Path) {
    // 建立包含已知時間點語音的測試音訊
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let duration_seconds = 60; // 1 分鐘
    let total_samples = 16000 * duration_seconds;

    for i in 0..total_samples {
        let t = i as f32 / 16000.0;

        // 在第 30 秒附近（分析窗口中心）建立語音
        let sample = if t >= 29.5 && t <= 32.0 {
            // 語音信號
            (0.4 * (2.0 * std::f32::consts::PI * 300.0 * t).sin()
                + 0.3 * (2.0 * std::f32::consts::PI * 600.0 * t).sin())
                * 32767.0
        } else {
            // 背景雜音
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
            Duration::from_secs(30), // 第一句在第 30 秒
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
                // 語音信號
                (0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) * 32767.0
            } else {
                // 靜音
                0.0
            };

            writer.write_sample(sample as i16).unwrap();
        }
    }

    writer.finalize().unwrap();
}
