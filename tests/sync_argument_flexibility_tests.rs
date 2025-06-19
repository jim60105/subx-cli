//! Comprehensive tests for sync command argument flexibility and intuitive usage.

use std::fs;
use std::sync::Arc;
use subx_cli::cli::{SyncArgs, SyncMethodArg, SyncMode};
use subx_cli::commands::sync_command;
use subx_cli::config::TestConfigService;
use tempfile::TempDir;

#[tokio::test]
async fn test_sync_batch_with_directory_argument() {
    // Create a temporary directory
    let tmp = TempDir::new().unwrap();
    let test_dir = tmp.path().to_path_buf();

    // Test batch mode with directory argument: subx sync -b test_dir
    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: None,
        subtitle: None,
        input_paths: Vec::new(),
        recursive: false,
        offset: None,
        method: None,
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: Some(Some(test_dir)),
    };

    assert!(args.validate().is_ok());
    let sync_mode = args.get_sync_mode();
    if let Err(ref e) = sync_mode {
        println!("Error: {:?}", e);
    }
    assert!(sync_mode.is_ok());
}

#[tokio::test]
async fn test_sync_single_video_positional() {
    // Create temporary files
    let tmp = TempDir::new().unwrap();
    let video_path = tmp.path().join("movie.mp4");
    let subtitle_path = tmp.path().join("movie.srt");
    fs::write(&video_path, b"fake video").unwrap();
    fs::write(
        &subtitle_path,
        b"1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n\n",
    )
    .unwrap();

    // Test single video file with auto-pairing: subx sync movie.mp4
    let args = SyncArgs {
        positional_paths: vec![video_path.clone()],
        video: None,
        subtitle: None,
        input_paths: Vec::new(),
        recursive: false,
        offset: None,
        method: None,
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: None,
    };

    assert!(args.validate().is_ok());
    let sync_mode = args.get_sync_mode();
    assert!(sync_mode.is_ok());

    if let Ok(SyncMode::Single { video, subtitle }) = sync_mode {
        assert_eq!(video, video_path);
        assert_eq!(subtitle, subtitle_path);
    } else {
        panic!("Expected Single sync mode");
    }
}

#[tokio::test]
async fn test_sync_manual_offset_with_positional() {
    let config_service = Arc::new(TestConfigService::with_sync_settings(0.5, 30.0));

    // Create temporary subtitle file
    let tmp = TempDir::new().unwrap();
    let subtitle_path = tmp.path().join("subtitle.srt");
    fs::write(
        &subtitle_path,
        b"1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n\n",
    )
    .unwrap();

    // Test manual offset with positional subtitle: subx sync --offset 2.5 subtitle.srt
    let args = SyncArgs {
        positional_paths: vec![subtitle_path.clone()],
        video: None,
        subtitle: None,
        input_paths: Vec::new(),
        recursive: false,
        offset: Some(2.5),
        method: Some(SyncMethodArg::Manual),
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: None,
    };

    assert!(args.validate().is_ok());

    // Execute the command
    let result = sync_command::execute(args, config_service.as_ref()).await;
    if let Err(ref e) = result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sync_validation_errors() {
    // Test validation fails when no inputs provided for auto mode
    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: None,
        subtitle: None,
        input_paths: Vec::new(),
        recursive: false,
        offset: None,
        method: None,
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: None,
    };

    assert!(args.validate().is_err());
}
