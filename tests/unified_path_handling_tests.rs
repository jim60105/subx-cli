//! Integration tests for unified input path handling across all CLI commands
//!
//! This test module verifies that all SubX CLI commands correctly implement
//! the unified path merging logic, allowing both main parameters (path, input, etc.)
//! and -i parameters to be used together or separately.

use std::fs;
use tempfile::TempDir;

use subx_cli::cli::{ConvertArgs, DetectEncodingArgs, InputPathHandler, MatchArgs, SyncArgs};
use subx_cli::error::SubXError;

/// Test that MatchArgs correctly merges path and input_paths
#[test]
fn test_match_args_path_merging() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    let file1 = temp_dir.path().join("file1.srt");

    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();
    fs::write(&file1, "test content").unwrap();

    // Test merging path and input_paths
    let args = MatchArgs {
        path: Some(dir1.clone()),
        input_paths: vec![dir2.clone(), file1.clone()],
        dry_run: false,
        confidence: 80,
        recursive: false,
        backup: false,
        copy: false,
        move_files: false,
    };

    let handler = args.get_input_handler().unwrap();
    let directories = handler.get_directories();

    // Should contain both dir1 and dir2, plus parent of file1
    assert!(directories.len() >= 2);
    assert!(directories.contains(&dir1));
    assert!(directories.contains(&dir2));
}

/// Test that ConvertArgs correctly merges input and input_paths
#[test]
fn test_convert_args_path_merging() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.srt");
    let file2 = temp_dir.path().join("file2.ass");
    let dir1 = temp_dir.path().join("dir1");

    fs::write(&file1, "test content").unwrap();
    fs::write(&file2, "test content").unwrap();
    fs::create_dir(&dir1).unwrap();

    use subx_cli::cli::OutputSubtitleFormat;

    // Test merging input and input_paths
    let args = ConvertArgs {
        input: Some(file1.clone()),
        input_paths: vec![file2.clone(), dir1.clone()],
        recursive: false,
        format: Some(OutputSubtitleFormat::Vtt),
        output: None,
        keep_original: false,
        encoding: "utf-8".to_string(),
    };

    let handler = args.get_input_handler().unwrap();
    let files = handler.collect_files().unwrap();

    // Should collect both files
    assert!(files.len() >= 2);
    assert!(files.contains(&file1));
    assert!(files.contains(&file2));
}

/// Test that SyncArgs correctly merges video, subtitle and input_paths
#[test]
fn test_sync_args_path_merging() {
    let temp_dir = TempDir::new().unwrap();
    let video = temp_dir.path().join("video.mp4");
    let subtitle = temp_dir.path().join("subtitle.srt");
    let dir1 = temp_dir.path().join("dir1");

    fs::write(&video, "test content").unwrap();
    fs::write(&subtitle, "test content").unwrap();
    fs::create_dir(&dir1).unwrap();

    use subx_cli::cli::SyncMethodArg;

    // Test merging video, subtitle and input_paths
    let args = SyncArgs {
        positional_paths: Vec::new(),
        video: Some(video.clone()),
        subtitle: Some(subtitle.clone()),
        input_paths: vec![dir1.clone()],
        recursive: false,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 30,
        vad_sensitivity: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: None,
    };

    let handler = args.get_input_handler().unwrap();
    let files = handler.collect_files().unwrap();

    // Should collect video and subtitle files
    assert!(files.len() >= 2);
    assert!(files.contains(&video));
    assert!(files.contains(&subtitle));
}

/// Test that DetectEncodingArgs correctly merges file_paths and input_paths
#[test]
fn test_detect_encoding_args_path_merging() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.srt");
    let file2 = temp_dir.path().join("file2.txt");
    let dir1 = temp_dir.path().join("dir1");

    fs::write(&file1, "test content").unwrap();
    fs::write(&file2, "test content").unwrap();
    fs::create_dir(&dir1).unwrap();

    // Test merging file_paths and input_paths
    let args = DetectEncodingArgs {
        verbose: false,
        input_paths: vec![dir1.clone()],
        recursive: false,
        file_paths: vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
        ],
    };

    let handler = args.get_input_handler().unwrap();
    let files = handler.collect_files().unwrap();

    // Should collect both files
    assert!(files.len() >= 2);
    assert!(files.contains(&file1));
    assert!(files.contains(&file2));
}

/// Test that InputPathHandler::merge_paths_from_multiple_sources works correctly
#[test]
fn test_input_path_handler_merge() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.srt");
    let file2 = temp_dir.path().join("file2.ass");
    let dir1 = temp_dir.path().join("dir1");

    fs::write(&file1, "test content").unwrap();
    fs::write(&file2, "test content").unwrap();
    fs::create_dir(&dir1).unwrap();

    let optional_paths = vec![Some(file1.clone()), None];
    let multiple_paths = vec![dir1.clone()];
    let string_paths = vec![file2.to_string_lossy().to_string()];

    let merged = InputPathHandler::merge_paths_from_multiple_sources(
        &optional_paths,
        &multiple_paths,
        &string_paths,
    )
    .unwrap();

    assert_eq!(merged.len(), 3);
    assert!(merged.contains(&file1));
    assert!(merged.contains(&dir1));
    assert!(merged.contains(&file2));
}

/// Test that error is returned when no paths are specified
#[test]
fn test_no_input_specified_error() {
    let result = InputPathHandler::merge_paths_from_multiple_sources(&[None, None], &[], &[]);

    assert!(matches!(result, Err(SubXError::NoInputSpecified)));
}

/// Test that get_directories returns correct directories
#[test]
fn test_get_directories() {
    let temp_dir = TempDir::new().unwrap();
    let dir1 = temp_dir.path().join("dir1");
    let dir2 = temp_dir.path().join("dir2");
    let file1 = temp_dir.path().join("file1.srt");
    let file_in_dir1 = dir1.join("file2.srt");

    fs::create_dir(&dir1).unwrap();
    fs::create_dir(&dir2).unwrap();
    fs::write(&file1, "test content").unwrap();
    fs::write(&file_in_dir1, "test content").unwrap();

    let paths = vec![dir1.clone(), file1.clone(), file_in_dir1.clone()];
    let handler = InputPathHandler::from_args(&paths, false).unwrap();
    let directories = handler.get_directories();

    // Should contain dir1, temp_dir (parent of file1), and dir1 again (parent of file_in_dir1)
    // But dir1 should be deduplicated
    assert!(directories.contains(&dir1));
    assert!(directories.contains(&temp_dir.path().to_path_buf()));

    // dir1 should only appear once due to deduplication
    let dir1_count = directories.iter().filter(|&d| d == &dir1).count();
    assert_eq!(dir1_count, 1);
}
