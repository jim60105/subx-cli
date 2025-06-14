//! CLI 參數和驗證的整合測試
//!
//! 此測試檔案驗證 Backlog 32.5 中定義的所有新 CLI 參數功能，
//! 包括參數解析、驗證邏輯、錯誤處理和使用者體驗。

use std::fs;
use tempfile::TempDir;

use subx_cli::cli::{SyncArgs, SyncMethodArg};
use subx_cli::config::{TestConfigBuilder, TestConfigService};

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

    // 測試 Whisper 方法參數
    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path.clone()),
        subtitle: subtitle_path.clone(),
        offset: None,
        method: Some(SyncMethodArg::Whisper),
        window: 30,
        whisper_model: Some("whisper-1".to_string()),
        whisper_language: Some("auto".to_string()),
        whisper_temperature: Some(0.1),
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

    // 驗證參數解析正確
    assert_eq!(args.method, Some(SyncMethodArg::Whisper));
    assert_eq!(args.window, 30);
    assert_eq!(args.whisper_model, Some("whisper-1".to_string()));
    assert_eq!(args.whisper_language, Some("auto".to_string()));
    assert_eq!(args.whisper_temperature, Some(0.1));
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
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 45,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: Some(0.8),
        vad_chunk_size: Some(1024),
        output: None,
        verbose: true,
        dry_run: false,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert_eq!(args.window, 45);
    assert_eq!(args.vad_sensitivity, Some(0.8));
    assert_eq!(args.vad_chunk_size, Some(1024));
    assert!(args.verbose);
}

/// 測試手動偏移模式
#[test]
fn test_sync_args_manual_offset() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: None, // 手動模式不需要視訊檔案
        subtitle: subtitle_path,
        offset: Some(2.5),
        method: Some(SyncMethodArg::Manual),
        window: 30,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    assert_eq!(args.offset, Some(2.5));
    assert_eq!(args.method, Some(SyncMethodArg::Manual));
    assert!(args.dry_run);
    assert!(args.is_manual_mode());
}

/// 測試內建參數驗證功能
#[test]
fn test_sync_args_validation() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    // 測試有效的參數組合
    #[allow(deprecated)]
    let valid_args = SyncArgs {
        video: Some(video_path.clone()),
        subtitle: subtitle_path.clone(),
        offset: None,
        method: Some(SyncMethodArg::Whisper),
        window: 30,
        whisper_model: Some("whisper-1".to_string()),
        whisper_language: None,
        whisper_temperature: None,
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

    // 使用內建驗證方法
    let validation_result = valid_args.validate();
    assert!(
        validation_result.is_ok(),
        "Valid args should pass validation"
    );

    // 測試無效的參數組合：手動方法但沒有偏移量
    #[allow(deprecated)]
    let invalid_args = SyncArgs {
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: None, // 手動方法需要偏移量
        method: Some(SyncMethodArg::Manual),
        window: 30,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
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

    let validation_result = invalid_args.validate();
    assert!(
        validation_result.is_err(),
        "Manual method without offset should fail validation"
    );
}

/// 測試輸出路徑生成
#[test]
fn test_output_path_generation() {
    let temp_dir = TempDir::new().unwrap();
    let subtitle_path = temp_dir.path().join("movie.srt");

    #[allow(deprecated)]
    let args = SyncArgs {
        video: None,
        subtitle: subtitle_path.clone(),
        offset: Some(2.5),
        method: Some(SyncMethodArg::Manual),
        window: 30,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None, // 使用預設輸出路徑
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    let output_path = args.get_output_path();
    assert!(output_path.to_string_lossy().contains("movie_synced.srt"));
}

/// 測試方法轉換
#[test]
fn test_sync_method_conversion() {
    use subx_cli::core::sync::SyncMethod;

    // 測試 SyncMethodArg 到 SyncMethod 的轉換
    let whisper_method: SyncMethod = SyncMethodArg::Whisper.into();
    assert_eq!(whisper_method, SyncMethod::WhisperApi);

    let vad_method: SyncMethod = SyncMethodArg::Vad.into();
    assert_eq!(vad_method, SyncMethod::LocalVad);

    let manual_method: SyncMethod = SyncMethodArg::Manual.into();
    assert_eq!(manual_method, SyncMethod::Manual);
}

/// 測試批次模式設定
#[test]
fn test_batch_mode_settings() {
    let temp_dir = TempDir::new().unwrap();
    let video_dir = temp_dir.path().join("videos");
    let subtitle_dir = temp_dir.path().join("subtitles");

    std::fs::create_dir(&video_dir).unwrap();
    std::fs::create_dir(&subtitle_dir).unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_dir),
        subtitle: subtitle_dir,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 30,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: Some(0.75),
        vad_chunk_size: None,
        output: None,
        verbose: true,
        dry_run: false,
        force: false,
        batch: true, // 啟用批次模式
        range: None,
        threshold: None,
    };

    assert!(args.batch);
    assert_eq!(args.method, Some(SyncMethodArg::Vad));
    assert!(args.verbose);
}

/// 測試高級 Whisper 設定
#[test]
fn test_advanced_whisper_settings() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: None,
        method: Some(SyncMethodArg::Whisper),
        window: 60, // 較長的分析窗口
        whisper_model: Some("whisper-1".to_string()),
        whisper_language: Some("zh".to_string()), // 指定中文
        whisper_temperature: Some(0.2),           // 稍微提高隨機性
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: Some(temp_dir.path().join("output.srt")),
        verbose: true,
        dry_run: false,
        force: true, // 強制覆蓋
        batch: false,
        range: None,
        threshold: None,
    };

    assert_eq!(args.window, 60);
    assert_eq!(args.whisper_language, Some("zh".to_string()));
    assert_eq!(args.whisper_temperature, Some(0.2));
    assert!(args.force);
    assert!(args.output.is_some());
}

/// 測試 VAD 精細調整設定
#[test]
fn test_vad_fine_tuning_settings() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: None,
        method: Some(SyncMethodArg::Vad),
        window: 45,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: Some(0.9), // 高敏感度
        vad_chunk_size: Some(2048), // 大塊大小
        output: None,
        verbose: true,
        dry_run: true, // 乾跑模式
        force: false,
        batch: false,
        range: None,
        threshold: None,
    };

    assert_eq!(args.vad_sensitivity, Some(0.9));
    assert_eq!(args.vad_chunk_size, Some(2048));
    assert!(args.dry_run);

    // 使用內建驗證方法
    let validation_result = args.validate();
    assert!(validation_result.is_ok());
}

/// 測試環境變數覆蓋（模擬）
#[test]
fn test_environment_variable_override_simulation() {
    // 這個測試模擬環境變數覆蓋配置的情況
    let config = TestConfigBuilder::new()
        .with_whisper_enabled(true)
        .with_vad_enabled(true)
        .build_config();

    let config_service = TestConfigService::new(config);

    // 驗證配置結構包含新的同步設定
    assert!(config_service.config().sync.whisper.enabled);
    assert!(config_service.config().sync.vad.enabled);
    assert_eq!(config_service.config().sync.analysis_window_seconds, 30);
}

/// 測試向後兼容性
#[test]
fn test_backward_compatibility() {
    let temp_dir = TempDir::new().unwrap();
    let video_path = temp_dir.path().join("test.mp4");
    let subtitle_path = temp_dir.path().join("test.srt");

    fs::write(&video_path, b"fake video content").unwrap();
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest\n").unwrap();

    #[allow(deprecated)]
    let args = SyncArgs {
        video: Some(video_path),
        subtitle: subtitle_path,
        offset: None,
        method: None, // 未指定方法，應該使用預設行為
        window: 30,
        whisper_model: None,
        whisper_language: None,
        whisper_temperature: None,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        verbose: false,
        dry_run: false,
        force: false,
        batch: false,
        range: Some(10.0),    // 已棄用的參數
        threshold: Some(0.8), // 已棄用的參數
    };

    // 測試向後兼容的方法
    assert_eq!(args.sync_method(), subx_cli::cli::SyncMethod::Auto);

    // 驗證仍然可以通過基本驗證
    let validation_result = args.validate();
    assert!(validation_result.is_ok());
}
