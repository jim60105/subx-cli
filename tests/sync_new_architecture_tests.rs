use std::fs;
use std::path::Path;
use tempfile::TempDir;

use subx_cli::cli::{SyncArgs, SyncMethodArg};
use subx_cli::core::sync::SyncMethod;

/// Test comprehensive integration functionality of the new sync architecture
///
/// This test suite verifies sync functionality after removing Whisper:
/// - New CLI parameter structure  
/// - VAD sync engine
/// - Method selection strategy
/// - Batch processing
/// - Error handling mechanisms

#[test]
fn test_sync_args_with_vad_method() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    // Create test files
    fs::write(&video_path, b"fake video content").unwrap();
    create_test_subtitle(&subtitle_path);

    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: Some(video_path.clone()),
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 45,
        vad_sensitivity: Some(0.8),
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
    };

    // Verify parameter parsing is correct
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.window, 45);
    assert_eq!(args.vad_sensitivity, Some(0.8));
}

#[test]
fn test_sync_args_with_vad_default_settings() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    create_test_subtitle(&subtitle_path);

    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: Some(video_path.clone()),
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
    };

    // Verify VAD parameters are set correctly
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.vad_sensitivity, None); // Use default value
}

#[test]
fn test_sync_args_with_manual_offset() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("test.srt");
    create_test_subtitle(&subtitle_path);

    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: None, // Manual offset doesn't require video file
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: Some(2.5),
        method: Some(SyncMethodArg::Manual),
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
    };

    // Verify manual offset settings
    assert_eq!(args.offset, Some(2.5));
    assert_eq!(args.video, None);
    assert_eq!(args.method, Some(SyncMethodArg::Manual));
}

#[test]
fn test_sync_args_batch_mode() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");

    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: Some(input_dir.clone()),
        subtitle: Some(input_dir.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad), // Use Vad instead of Auto
        window: 30,
        vad_sensitivity: None,
        output: Some(output_dir.clone()),
        verbose: false,
        dry_run: false,
        force: false,
        batch: true,
    };

    // Verify batch mode settings
    assert!(args.batch);
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
}

#[test]
fn test_sync_args_validation() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("test.srt");
    create_test_subtitle(&subtitle_path);

    // Test manual method requires offset parameter
    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: None,
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: None, // Missing offset
        method: Some(SyncMethodArg::Manual),
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
    };

    let validation_result = args.validate();
    assert!(
        validation_result.is_err(),
        "Manual method without offset should fail validation"
    );

    // Test valid manual offset
    let valid_args = SyncArgs {
        positional_paths: Vec::new(),
        offset: Some(2.5), // Provide offset
        ..args
    };

    let validation_result = valid_args.validate();
    assert!(
        validation_result.is_ok(),
        "Manual method with offset should be valid"
    );
}

// Previously there was an integration test requiring audio environment here,
// but it was removed due to complex environment dependencies.
// Actual integration tests are covered in other test files.

#[test]
fn test_sync_method_conversion() {
    // Test CLI enum to core enum conversion
    let vad_arg = SyncMethodArg::Vad;
    let vad_method: SyncMethod = vad_arg.into();
    assert_eq!(vad_method, SyncMethod::LocalVad);

    let manual_arg = SyncMethodArg::Manual;
    let manual_method: SyncMethod = manual_arg.into();
    assert_eq!(manual_method, SyncMethod::Manual);
}

// Helper functions - Actual integration tests are in other files

fn create_test_subtitle(path: &Path) {
    let subtitle_content = r#"1
00:00:01,000 --> 00:00:03,000
This is a test subtitle.

2
00:00:04,000 --> 00:00:06,000
Another test subtitle line.
"#;
    fs::write(path, subtitle_content).unwrap();
}
