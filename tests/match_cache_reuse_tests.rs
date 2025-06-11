//! Integration tests for match command cache reuse with copy/move modes.

use std::fs;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::TestConfigBuilder;
use tempfile::TempDir;

#[tokio::test]
#[ignore]
async fn test_cache_reuse_preserves_copy_mode() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "sub").unwrap();

    let args_preview = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    let config_service = TestConfigBuilder::new()
        .with_ai_api_key("test-key")
        .build_service();
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    let args_execute = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: true,
        move_files: false,
    };
    match_command::execute(args_execute, &config_service)
        .await
        .unwrap();

    assert!(
        video_dir.join("movie.srt").exists(),
        "Expected subtitle copy to video directory"
    );
    assert!(
        subtitle_dir.join("movie.srt").exists(),
        "Original subtitle should remain"
    );
}

#[tokio::test]
#[ignore]
async fn test_cache_reuse_preserves_move_mode() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let video_dir = root.join("videos");
    let subtitle_dir = root.join("subtitles");
    fs::create_dir_all(&video_dir).unwrap();
    fs::create_dir_all(&subtitle_dir).unwrap();

    fs::write(video_dir.join("movie.mp4"), "video").unwrap();
    fs::write(subtitle_dir.join("movie.srt"), "sub").unwrap();

    let args_preview = MatchArgs {
        path: root.to_path_buf(),
        dry_run: true,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    let config_service = TestConfigBuilder::new()
        .with_ai_api_key("test-key")
        .build_service();
    match_command::execute(args_preview, &config_service)
        .await
        .unwrap();

    let args_execute = MatchArgs {
        path: root.to_path_buf(),
        dry_run: false,
        recursive: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: true,
    };
    match_command::execute(args_execute, &config_service)
        .await
        .unwrap();

    assert!(
        video_dir.join("movie.srt").exists(),
        "Expected subtitle move to video directory"
    );
    assert!(
        !subtitle_dir.join("movie.srt").exists(),
        "Original subtitle should have moved"
    );
}
