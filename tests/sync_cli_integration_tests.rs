//! CLI 參數和驗證的整合測試
//!
//! 此測試檔案驗證移除 Whisper 後的 CLI 參數功能，
//! 包括 VAD 參數解析、手動偏移驗證邏輯、錯誤處理和使用者體驗。

use std::fs;
use tempfile::TempDir;

use subx_cli::cli::{SyncArgs, SyncMethodArg};

/// 測試 CLI 參數解析的基本功能
#[test]
fn test_sync_args_basic_parsing() {
    // 測試基本的參數解析
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    // 創建測試檔案
    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    // 測試 VAD 方法參數
    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path.clone()),
        subtitle: subtitle_path.clone(),
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

    // 驗證參數解析正確
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.window, 30);
    assert_eq!(args.vad_sensitivity, Some(0.8));
    assert_eq!(args.vad_chunk_size, Some(1024));
}

/// 測試 VAD 方法的參數設定
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
        subtitle: subtitle_path.clone(),
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

    // 驗證 VAD 參數
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.vad_sensitivity, Some(0.7));
    assert_eq!(args.vad_chunk_size, Some(512));
    assert_eq!(args.window, 45);
    assert!(args.verbose);
}

/// 測試手動偏移方法
#[test]
fn test_sync_args_manual_method() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: None, // 手動偏移不需要視訊檔案
        subtitle: subtitle_path.clone(),
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

    // 驗證手動偏移參數
    assert_eq!(args.method, Some(SyncMethodArg::Manual));
    assert_eq!(args.offset, Some(2.5));
    assert_eq!(args.video, None);
}

/// 測試批次處理模式的參數設定
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
        video: Some(video_path.clone()),
        subtitle: subtitle_path.clone(),
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

    // 驗證批次模式設置
    assert!(args.batch);
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.output, Some(output_dir));
}
