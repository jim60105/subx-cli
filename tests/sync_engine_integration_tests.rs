use hound::{SampleFormat, WavSpec, WavWriter};
use std::time::Duration;
use subx_cli::config::{TestConfigBuilder, TestConfigService};
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::core::sync::{SyncEngine, SyncMethod};
use tempfile::TempDir;

#[tokio::test]
async fn test_sync_engine_with_vad_only() {
    let temp_dir = TempDir::new().unwrap();

    let audio_path = temp_dir.path().join("test.wav");
    let subtitle = create_test_subtitle_with_offset();
    create_test_audio(&audio_path);

    let config = TestConfigBuilder::new()
        .with_whisper_enabled(false)
        .with_vad_enabled(true)
        .build_config();

    let config_service = TestConfigService::new(config);
    let engine = SyncEngine::new(config_service.get_config().unwrap().sync, &config_service)
        .await
        .unwrap();

    let result = engine
        .detect_sync_offset(&audio_path, &subtitle, Some(SyncMethod::LocalVad))
        .await
        .unwrap();

    assert_eq!(result.method_used, SyncMethod::LocalVad);
    assert!(result.confidence > 0.0);
}

#[tokio::test]
async fn test_auto_method_selection_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let audio_path = temp_dir.path().join("test.wav");
    let subtitle = create_test_subtitle_with_offset();
    create_test_audio(&audio_path);

    let config = TestConfigBuilder::new()
        .with_whisper_enabled(false)
        .with_vad_enabled(true)
        .build_config();

    let config_service = TestConfigService::new(config);
    let engine = SyncEngine::new(config_service.get_config().unwrap().sync, &config_service)
        .await
        .unwrap();

    let result = engine
        .detect_sync_offset(&audio_path, &subtitle, Some(SyncMethod::Auto))
        .await
        .unwrap();

    assert_eq!(result.method_used, SyncMethod::LocalVad);
}

#[tokio::test]
async fn test_no_methods_available_error() {
    let config = TestConfigBuilder::new()
        .with_whisper_enabled(false)
        .with_vad_enabled(false)
        .build_config();

    let config_service = TestConfigService::new(config);
    let result = SyncEngine::new(config_service.get_config().unwrap().sync, &config_service).await;

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No synchronization methods available")
    );
}

fn create_test_subtitle_with_offset() -> Subtitle {
    Subtitle {
        entries: vec![SubtitleEntry::new(
            1,
            Duration::from_secs(30),
            Duration::from_secs(32),
            "Test dialogue".to_string(),
        )],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    }
}

fn create_test_audio(path: &std::path::Path) {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();
    let duration_seconds = 60;
    let total_samples = 16000 * duration_seconds;
    for i in 0..total_samples {
        let t = i as f32 / 16000.0;
        let sample = if t >= 29.5 && t <= 32.0 {
            (0.3 * (2.0 * std::f32::consts::PI * t).sin()) * 32767.0
        } else {
            0.0
        };
        writer.write_sample(sample as i16).unwrap();
    }
    writer.finalize().unwrap();
}
