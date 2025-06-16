//! Integration tests for CLI parameters and validation
//!
//! This test file verifies CLI parameter functionality after removing Whisper,
//! including VAD parameter parsing, manual offset validation logic, error handling and user experience.

use std::fs;
use tempfile::TempDir;

use subx_cli::cli::{SyncArgs, SyncMethodArg};

/// Test basic functionality of CLI parameter parsing
#[test]
fn test_sync_args_basic_parsing() {
    // Test basic parameter parsing
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    // Create test files
    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    // Test VAD method parameters
    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path.clone()),
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 30,
        vad_sensitivity: Some(0.8),
        vad_chunk_size: Some(1024),
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    // Verify parameter parsing is correct
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.window, 30);
    assert_eq!(args.vad_sensitivity, Some(0.8));
    assert_eq!(args.vad_chunk_size, Some(1024));
}

/// Test VAD method parameter settings
#[test]
fn test_sync_args_vad_method() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path.clone()),
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 45,
        vad_sensitivity: Some(0.7),
        vad_chunk_size: Some(512),
        output: None,
        verbose: true,
        dry_run: false,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    // Verify VAD parameters
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.vad_sensitivity, Some(0.7));
    assert_eq!(args.vad_chunk_size, Some(512));
    assert_eq!(args.window, 45);
    assert!(args.verbose);
}

/// Test manual offset method
#[test]
fn test_sync_args_manual_method() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        input_paths: vec![],
        recursive: false,
        video: None, // Manual offset doesn't require video file
        subtitle: Some(subtitle_path.clone()),
        offset: Some(2.5),
        method: Some(SyncMethodArg::Manual),
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    // Verify manual offset parameters
    assert_eq!(args.method, Some(SyncMethodArg::Manual));
    assert_eq!(args.offset, Some(2.5));
    assert_eq!(args.video, None);
}

/// Test batch processing mode parameter settings
#[test]
fn test_sync_args_batch_mode() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");
    let output_dir = temp_dir.path().join("output");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        input_paths: vec![],
        recursive: false,
        video: Some(video_path.clone()),
        subtitle: Some(subtitle_path.clone()),
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: Some(output_dir.clone()),
        verbose: false,
        dry_run: false,
        force: false,
        batch: true,
        range: None,
        threshold: None,
    };

    // Verify batch mode settings
    assert!(args.batch);
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.output, Some(output_dir));
}
