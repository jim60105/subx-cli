use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;
use subx_cli::config::test_service::TestConfigService;

#[tokio::test]
async fn test_manual_sync_without_video_file() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");

    // Create test subtitle file
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
測試字幕 1

2
00:00:04,000 --> 00:00:06,000
測試字幕 2
"#;
    fs::write(&subtitle_path, srt_content).unwrap();

    let args = SyncArgs {
        video: None,
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: Some(2.5),
        method: None,
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        batch: false,
        range: None,
        threshold: None,
    };
    let config_service = Arc::new(TestConfigService::new());
    let result = sync_command::execute_with_config(args, config_service).await;

    assert!(result.is_ok(), "Manual sync should execute successfully");

    // Verify subtitle timeline has been adjusted
    let updated_content = fs::read_to_string(&subtitle_path).unwrap();
    assert!(updated_content.contains("00:00:03,500")); // 1s + 2.5s
    assert!(updated_content.contains("00:00:06,500")); // 4s + 2.5s
}

#[tokio::test]
async fn test_auto_sync_requires_video_file() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");

    let args = SyncArgs {
        video: None,
        subtitle: Some(subtitle_path),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: None,
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        batch: false,
        range: None,
        threshold: None,
    };

    let result = args.validate();
    assert!(result.is_err(), "Auto mode missing video file should produce error");
}

#[tokio::test]
async fn test_backward_compatibility() {
    let temp = TempDir::new().unwrap();
    let video_path = temp.path().join("video.mp4");
    let subtitle_path = temp.path().join("test.srt");

    // Create empty test files
    fs::write(&video_path, b"").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    let args = SyncArgs {
        video: Some(video_path),
        subtitle: Some(subtitle_path),
        input_paths: vec![],
        recursive: false,
        offset: Some(1.5),
        method: None,
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        batch: false,
        range: None,
        threshold: None,
    };

    let result = args.validate();
    assert!(result.is_ok(), "Backward compatibility should be maintained");
}
