use std::fs;
use std::sync::Arc;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;
use subx_core::config::{Config, TestConfigService};
use tempfile::TempDir;

/// Build [`SyncArgs`] for a single-file sync with the fields this suite never
/// exercises pinned to their clap defaults, so the literal tracks the current
/// struct shape.
fn base_args(
    subtitle: Option<std::path::PathBuf>,
    video: Option<std::path::PathBuf>,
    offset: Option<f32>,
) -> SyncArgs {
    SyncArgs {
        positional_paths: vec![],
        video,
        subtitle,
        input_paths: vec![],
        recursive: false,
        offset,
        method: None,
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: None,
        no_extract: false,
    }
}

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

    let args = base_args(Some(subtitle_path.clone()), None, Some(2.5));
    let config_service = Arc::new(TestConfigService::new(Config::default()));
    let result = sync_command::execute_with_config(args, config_service).await;

    assert!(result.is_ok(), "Manual sync should execute successfully");

    // Characterised (B4 revival): the sync command never rewrites the input
    // subtitle in place — with no `--output` it saves to the default output
    // path `<stem>_synced.<ext>`. Verify the timeline there instead.
    let synced_path = temp.path().join("test_synced.srt");
    assert!(
        synced_path.exists(),
        "synced subtitle should be written to the default output path"
    );
    let updated_content = fs::read_to_string(&synced_path).unwrap();
    assert!(updated_content.contains("00:00:03,500")); // 1s + 2.5s
    assert!(updated_content.contains("00:00:06,500")); // 4s + 2.5s
}

#[tokio::test]
async fn test_auto_sync_requires_video_file() {
    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");

    let args = base_args(Some(subtitle_path), None, None);

    let result = args.validate();
    assert!(
        result.is_err(),
        "Auto mode missing video file should produce error"
    );
}

#[tokio::test]
async fn test_backward_compatibility() {
    let temp = TempDir::new().unwrap();
    let video_path = temp.path().join("video.mp4");
    let subtitle_path = temp.path().join("test.srt");

    // Create empty test files
    fs::write(&video_path, b"").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    let args = base_args(Some(subtitle_path), Some(video_path), Some(1.5));

    let result = args.validate();
    assert!(
        result.is_ok(),
        "Backward compatibility should be maintained"
    );
}
