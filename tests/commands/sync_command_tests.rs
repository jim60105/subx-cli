use tempfile::TempDir;
use std::fs;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;
use crate::common::command_helpers::create_utf8_subtitle_file;

#[tokio::test]
async fn test_audio_sync_workflow_manual_offset() {
    let temp = TempDir::new().unwrap();
    let video = temp.path().join("video.mp4");
    fs::write(&video, b"").unwrap();
    let subtitle = create_utf8_subtitle_file(&temp).await;
    let args = SyncArgs {
        video: video.clone(),
        subtitle: subtitle.clone(),
        offset: Some(1.5),
        batch: false,
        range: None,
        threshold: None,
    };
    let result = sync_command::execute(args).await;
    assert!(result.is_ok());
}
