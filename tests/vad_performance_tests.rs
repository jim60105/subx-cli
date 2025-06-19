use std::time::Instant;
use subx_cli::config::VadConfig;
use subx_cli::services::vad::LocalVadDetector;
use tempfile::TempDir;

#[tokio::test]
#[ignore] // Marked as performance test, not executed normally
async fn test_vad_performance_large_audio() {
    let temp_dir = TempDir::new().unwrap();
    let audio_path = temp_dir.path().join("large_audio.wav");

    // Create 10 minutes of test audio
    create_long_audio_file(&audio_path, 600);

    let config = VadConfig::default();
    let detector = LocalVadDetector::new(config).unwrap();
    let processor = subx_cli::services::vad::VadAudioProcessor::new().unwrap();
    let audio_data = processor
        .load_and_prepare_audio_direct(&audio_path)
        .await
        .unwrap();

    let start_time = Instant::now();
    let result = detector.detect_speech_from_data(audio_data).await.unwrap();
    let processing_time = start_time.elapsed();

    // Verify performance requirements
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

        // Create speech segment every 10 seconds
        let is_speech_time = (t % 10.0) >= 2.0 && (t % 10.0) <= 5.0;

        let sample = if is_speech_time {
            // Speech signal
            (0.3 * (2.0 * std::f32::consts::PI * 400.0 * t).sin()
                + 0.2 * (2.0 * std::f32::consts::PI * 800.0 * t).sin())
                * 32767.0
        } else {
            // Background noise
            ((t * 9973.0).sin() * 0.01) * 32767.0
        };

        if i % (16000 * 30) == 0 {
            println!("Generated {} seconds of audio", i / 16000);
        }

        writer.write_sample(sample as i16).unwrap();
    }

    writer.finalize().unwrap();
}
