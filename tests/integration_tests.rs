use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create test media and subtitle files
fn create_test_media_files(dir: &Path) {
    // Create dummy video file
    let video_path = dir.join("video.mp4");
    fs::write(&video_path, b"").unwrap();
    // Create simple SRT subtitle file
    let srt_path = dir.join("video.srt");
    let srt_content = "\
1
00:00:00,000 --> 00:00:01,000
Hello
";
    fs::write(&srt_path, srt_content).unwrap();
}

/// Test match command's dry-run functionality
fn test_match_command(dir: &Path) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.env("OPENAI_API_KEY", "test")
        .current_dir(dir)
        .arg("match")
        .arg(dir)
        .arg("--dry-run")
        .assert()
        .success()
        // Characterised (B4 revival, task 6.3): the dry-run banner is the
        // capitalised "Preview mode" line printed by display_match_results.
        .stdout(predicate::str::contains("Preview mode"));
}

/// Test convert command to convert SRT to VTT
fn test_convert_command(dir: &Path) {
    let subtitle = dir.join("video.srt");
    let output = dir.join("video.vtt");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("convert")
        .arg(&subtitle)
        // Characterised (B4 revival, task 6.3): convert deletes the source
        // unless --keep-original is given, which would starve the sync leg
        // below of its subtitle input.
        .arg("--keep-original")
        .arg("--format")
        .arg("vtt")
        .assert()
        .success();
    // Check output file and content
    assert!(output.exists(), "Converted VTT file does not exist");
    assert!(
        subtitle.exists(),
        "--keep-original must preserve the source"
    );
    let content = fs::read_to_string(output).unwrap();
    assert!(
        content.contains("WEBVTT"),
        "Output does not contain WEBVTT header"
    );
}

/// Test sync command's manual offset functionality
fn test_sync_command(dir: &Path) {
    let video = dir.join("video.mp4");
    let subtitle = dir.join("video.srt");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_subx-cli"));
    cmd.arg("sync")
        .arg(&video)
        .arg(&subtitle)
        .arg("--offset")
        .arg("1")
        .assert()
        .success()
        // Characterised (B4 revival, task 6.3): the manual-offset run reports
        // through the success summary; "manual offset applied" was never
        // printed — the verbose line is "Using manual offset:".
        .stdout(predicate::str::contains("Sync completed: offset 1.000s"));
}

#[test]
fn test_full_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path();
    create_test_media_files(dir);
    test_match_command(dir);
    test_convert_command(dir);
    test_sync_command(dir);
}
