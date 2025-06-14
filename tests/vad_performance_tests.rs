use std::time::Instant;
use subx_cli::config::VadConfig;
use subx_cli::services::vad::LocalVadDetector;
use tempfile::TempDir;

#[tokio::test]
#[ignore] // 標記為性能測試，平時不執行
async fn test_vad_performance_large_audio() {
    let temp_dir = TempDir::new().unwrap();
    let audio_path = temp_dir.path().join("large_audio.wav");

    // 建立 10 分鐘的測試音訊
    create_long_audio_file(&audio_path, 600);

    let config = VadConfig::default();
    let detector = LocalVadDetector::new(config).unwrap();

    let start_time = Instant::now();
    let result = detector.detect_speech(&audio_path).await.unwrap();
    let processing_time = start_time.elapsed();

    // 驗證性能要求
    assert!(
        processing_time.as_secs() < 30,
        "Processing took too long: {:?}",
        processing_time
    );
    assert!(!result.speech_segments.is_empty());

    println!("Processed 10 minutes of audio in {:?}", processing_time);
    println!("Found {} speech segments", result.speech_segments.len());
}

fn create_long_audio_file(path: &std::path::Path, duration_seconds: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let total_samples = 16000 * duration_seconds;

    for i in 0..total_samples {
        let t = i as f32 / 16000.0;

        // 每 10 秒建立一個語音片段
        let is_speech_time = (t % 10.0) >= 2.0 && (t % 10.0) <= 5.0;

        let sample = if is_speech_time {
            // 語音信號
            (0.3 * (2.0 * std::f32::consts::PI * 400.0 * t).sin()
                + 0.2 * (2.0 * std::f32::consts::PI * 800.0 * t).sin())
                * 32767.0
        } else {
            // 背景雜音
            ((t * 9973.0).sin() * 0.01) * 32767.0
        };

        if i % (16000 * 30) == 0 {
            println!("Generated {} seconds of audio", i / 16000);
        }

        writer.write_sample(sample as i16).unwrap();
    }

    writer.finalize().unwrap();
}
