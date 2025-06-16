use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;
use std::time::{Duration, Instant};
use subx_cli::config::{TestConfigBuilder, TestConfigService};
use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use subx_cli::core::sync::{SyncEngine, SyncMethod};
use tempfile::TempDir;

#[tokio::test]
#[ignore] // Performance test, not executed normally
async fn test_sync_engine_performance_comparison() {
    let temp_dir = TempDir::new().unwrap();
    let audio_path = temp_dir.path().join("large_test.wav");
    let subtitle = create_large_test_subtitle();
    create_large_test_audio(&audio_path, 300);

    let config = TestConfigBuilder::new()
        .with_vad_enabled(true)
        .build_config();

    let config_service = TestConfigService::new(config);
    let engine = SyncEngine::new(config_service.config().sync.clone()).unwrap();

    let start_time = Instant::now();
    let result = engine
        .detect_sync_offset(&audio_path, &subtitle, Some(SyncMethod::LocalVad))
        .await
        .unwrap();
    let vad_duration = start_time.elapsed();

    println!("VAD processing time: {:?}", vad_duration);
    println!("VAD confidence: {:.2}%", result.confidence * 100.0);

    assert!(vad_duration.as_secs() < 30, "VAD processing took too long");
    assert!(result.confidence > 0.0);
}

fn create_large_test_subtitle() -> Subtitle {
    Subtitle {
        entries: vec![SubtitleEntry::new(
            1,
            Duration::from_secs(0),
            Duration::from_secs(300),
            "Test dialogue".to_string(),
        )],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    }
}

fn create_large_test_audio(path: &Path, duration_seconds: u32) {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).unwrap();
    let total_samples = 16000 * duration_seconds as usize;
    for _ in 0..total_samples {
        writer.write_sample(0_i16).unwrap();
    }
    writer.finalize().unwrap();
}
