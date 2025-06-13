use std::fs;
use subx_cli::Result;
use subx_cli::services::audio::AusAudioAnalyzer;
use tempfile::TempDir;

/// Test that we handle the case where aus returns AudioFile with empty samples
/// This simulates the original panic condition
#[tokio::test]
async fn test_aus_empty_samples_handling() {
    // This test verifies that our fix prevents the original panic
    // The original error was: index out of bounds: the len is 0 but the index is 0
    // which occurred when code tried to access samples[0] on an empty samples array

    let analyzer = AusAudioAnalyzer::new(44100);
    let temp_dir = TempDir::new().unwrap();

    // Try different scenarios that might cause aus to return empty samples
    let scenarios = vec![
        ("tiny.wav", "RIFF....WAVE"),   // Minimal invalid WAV
        ("corrupt.mkv", "EBML"),        // Minimal invalid MKV
        ("empty.mp3", ""),              // Empty file
        ("random.avi", "RIFF....AVI "), // Invalid AVI
    ];

    for (filename, content) in scenarios {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).unwrap();

        // This should not panic, but should return a meaningful error
        let result = analyzer.load_audio_file(&file_path).await;

        // Verify we get an error instead of a panic
        assert!(result.is_err(), "Expected error for file: {}", filename);

        if let Err(error) = result {
            let error_msg = format!("{}", error);
            println!("File: {}, Error: {}", filename, error_msg);

            // Should be some kind of audio processing error
            assert!(
                error_msg.contains("audio processing")
                    || error_msg.contains("FileCorrupt")
                    || error_msg.contains("no samples"),
                "Unexpected error format for {}: {}",
                filename,
                error_msg
            );
        }
    }
}

/// Test the original problematic command scenario
#[tokio::test]
async fn test_original_panic_scenario() -> Result<()> {
    // This simulates the original command that caused the panic:
    // subx-cli sync "[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].mkv"
    //              "[Noumin Kanren no Skill][01][BDRIP][1080P][H264_FLACx2].sc.srts"

    let temp_dir = TempDir::new().unwrap();

    // Create files with similar names to the original problem
    let video_path = temp_dir
        .path()
        .join("[Test][01][BDRIP][1080P][H264_FLACx2].mkv");
    let subtitle_path = temp_dir
        .path()
        .join("[Test][01][BDRIP][1080P][H264_FLACx2].sc.srt");

    // Create problematic video file (likely to cause aus issues)
    fs::write(&video_path, "This is not a valid MKV file").unwrap();

    // Create valid subtitle
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
Test subtitle

"#;
    fs::write(&subtitle_path, srt_content).unwrap();

    // Test audio analysis (the part that was failing)
    let analyzer = AusAudioAnalyzer::new(44100);
    let result = analyzer.load_audio_file(&video_path).await;

    // Should get error, not panic
    assert!(result.is_err());

    if let Err(error) = result {
        let error_msg = format!("{}", error);
        println!("Original scenario error: {}", error_msg);

        // Should be a meaningful error message
        assert!(
            error_msg.contains("audio processing")
                || error_msg.contains("FileCorrupt")
                || error_msg.contains("no samples")
        );
    }

    Ok(())
}
