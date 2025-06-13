use std::fs;
use subx_cli::Result;
use subx_cli::services::audio::AusAudioAnalyzer;
use tempfile::TempDir;

/// Test handling of corrupted or invalid audio files
#[tokio::test]
async fn test_invalid_audio_file_handling() {
    let analyzer = AusAudioAnalyzer::new(44100);
    let temp_dir = TempDir::new().unwrap();

    // Create invalid audio file (not a real audio file)
    let invalid_path = temp_dir.path().join("invalid.mkv");
    fs::write(&invalid_path, b"This is not audio data").unwrap();

    let result = analyzer.load_audio_file(&invalid_path).await;
    assert!(result.is_err());

    // The error should be descriptive
    if let Err(error) = result {
        let error_msg = format!("{}", error);
        println!("Error message: {}", error_msg);
        // Should contain "FileCorrupt" or similar aus error
        assert!(error_msg.contains("FileCorrupt") || error_msg.contains("audio processing"));
    }
}

/// Test handling of empty audio files
#[tokio::test]
async fn test_empty_audio_file_handling() {
    let analyzer = AusAudioAnalyzer::new(44100);
    let temp_dir = TempDir::new().unwrap();

    // Create empty file
    let empty_path = temp_dir.path().join("empty.wav");
    fs::write(&empty_path, b"").unwrap();

    let result = analyzer.load_audio_file(&empty_path).await;
    assert!(result.is_err());

    // The error should mention that the file is corrupt or invalid
    if let Err(error) = result {
        let error_msg = format!("{}", error);
        println!("Error message: {}", error_msg);
        assert!(error_msg.contains("FileCorrupt") || error_msg.contains("audio processing"));
    }
}

/// Test sync command with problematic audio file
#[tokio::test]
async fn test_sync_with_problematic_audio() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();

    // Create a problematic video file (should trigger the aus error)
    let video_path = temp_dir.path().join("problematic.mkv");
    fs::write(&video_path, b"RIFF....WAVE").unwrap(); // Invalid but structured data

    // Create a valid subtitle file
    let subtitle_path = temp_dir.path().join("test.srt");
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
Test subtitle

"#;
    fs::write(&subtitle_path, srt_content).unwrap();

    let analyzer = AusAudioAnalyzer::new(44100);
    let result = analyzer.load_audio_file(&video_path).await;

    // Should get a meaningful error instead of a panic
    assert!(result.is_err());
    if let Err(error) = result {
        let error_msg = format!("{}", error);
        println!("Error message: {}", error_msg);
        assert!(
            error_msg.contains("FileCorrupt")
                || error_msg.contains("no samples")
                || error_msg.contains("audio processing")
        );
    }

    Ok(())
}
