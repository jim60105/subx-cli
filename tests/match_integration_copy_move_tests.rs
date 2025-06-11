use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Integration tests for `subx match` with --copy and --move flags.
#[test]
fn test_match_copy_and_move_operations() {
    let base = TempDir::new().unwrap();
    let base_path = base.path();

    let video_dir = base_path.join("video");
    let subtitle_dir = base_path.join("subtitle");
    fs::create_dir(&video_dir).unwrap();
    fs::create_dir(&subtitle_dir).unwrap();

    // Create video and subtitle files
    let video = video_dir.join("movie.mp4");
    fs::write(&video, b"").unwrap();
    let subtitle = subtitle_dir.join("movie.srt");
    let content = "1\n00:00:00,000 --> 00:00:01,000\nHello\n";
    fs::write(&subtitle, content).unwrap();

    // Test copy mode: subtitle should be renamed and copied, original retained
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("match")
        .arg(base_path)
        .arg("--recursive")
        .arg("--copy")
        .assert()
        .success();
    assert!(video_dir.join("movie.srt").exists());
    assert!(subtitle_dir.join("movie.srt").exists());

    // Reset for move test: remove copied and renamed subtitle, restore original name
    fs::remove_file(video_dir.join("movie.srt")).ok();
    fs::remove_file(subtitle_dir.join("movie.srt")).ok();
    fs::write(&subtitle, content).unwrap();

    // Test move mode: subtitle should be moved, no longer in original folder
    let mut cmd = Command::cargo_bin("subx").unwrap();
    cmd.arg("match")
        .arg(base_path)
        .arg("--recursive")
        .arg("--move")
        .assert()
        .success();
    assert!(video_dir.join("movie.srt").exists());
    assert!(!subtitle_dir.join("movie.srt").exists());
}
