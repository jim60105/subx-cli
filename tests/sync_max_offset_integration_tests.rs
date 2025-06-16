use std::fs;
use std::sync::Arc;
use subx_cli::Result;
use subx_cli::cli::SyncArgs;
use subx_cli::commands::sync_command;
use subx_cli::config::{Config, TestConfigService};
use subx_cli::core::sync::SyncEngine;
use tempfile::TempDir;

#[tokio::test]
async fn test_manual_offset_exceeds_max_limit() -> Result<()> {
    // 建立配置：max_offset_seconds = 30.0
    let mut config = Config::default();
    config.sync.max_offset_seconds = 30.0;
    let config_service = Arc::new(TestConfigService::new(config));

    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    // 嘗試使用超過限制的偏移量 (45.0 > 30.0)
    let args = SyncArgs {
        video: None,
        subtitle: Some(subtitle_path),
        input_paths: vec![],
        recursive: false,
        offset: Some(45.0),
        method: Some(subx_cli::cli::SyncMethodArg::Manual),
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: None,
        verbose: false,
        dry_run: true,
        force: false,
        batch: false,
        #[allow(deprecated)]
        range: None,
        #[allow(deprecated)]
        threshold: None,
    };

    let result = sync_command::execute_with_config(args, config_service).await;

    // 應該返回錯誤
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("exceeds the configured maximum allowed value"));
    assert!(error_msg.contains("30.00"));
    assert!(error_msg.contains("45.00"));

    Ok(())
}

#[tokio::test]
async fn test_manual_offset_within_limit() -> Result<()> {
    // 建立配置：max_offset_seconds = 60.0
    let mut config = Config::default();
    config.sync.max_offset_seconds = 60.0;
    let config_service = Arc::new(TestConfigService::new(config));

    let temp = TempDir::new().unwrap();
    let subtitle_path = temp.path().join("test.srt");
    fs::write(&subtitle_path, "1\n00:00:01,000 --> 00:00:03,000\nTest").unwrap();

    // 使用在限制內的偏移量 (25.0 < 60.0)
    let args = SyncArgs {
        video: None,
        subtitle: Some(subtitle_path.clone()),
        input_paths: vec![],
        recursive: false,
        offset: Some(25.0),
        method: Some(subx_cli::cli::SyncMethodArg::Manual), // 明確指定手動模式
        window: 30,
        vad_sensitivity: None,
        vad_chunk_size: None,
        output: Some(temp.path().join("output.srt")),
        verbose: false,
        dry_run: false,
        force: true,
        batch: false,
        #[allow(deprecated)]
        range: None,
        #[allow(deprecated)]
        threshold: None,
    };

    let result = sync_command::execute_with_config(args, config_service).await;

    // 應該成功執行
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_sync_engine_manual_offset_validation() -> Result<()> {
    // 建立配置：max_offset_seconds = 15.0
    let mut config = Config::default();
    config.sync.max_offset_seconds = 15.0;
    config.sync.vad.enabled = true;

    let sync_engine = SyncEngine::new(config.sync)?;
    let mut subtitle = create_test_subtitle();

    // 測試超過限制的偏移量
    let result = sync_engine.apply_manual_offset(&mut subtitle, 20.0);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Offset"));
    assert!(error_msg.contains("exceeds maximum allowed value"));

    // 測試在限制內的偏移量
    let result = sync_engine.apply_manual_offset(&mut subtitle, 10.0);
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_negative_offset_validation() -> Result<()> {
    // 測試負偏移量也受到限制
    let mut config = Config::default();
    config.sync.max_offset_seconds = 20.0;
    config.sync.vad.enabled = true;

    let sync_engine = SyncEngine::new(config.sync)?;
    let mut subtitle = create_test_subtitle();

    // 測試超過限制的負偏移量
    let result = sync_engine.apply_manual_offset(&mut subtitle, -25.0);
    assert!(result.is_err());

    // 測試在限制內的負偏移量
    let result = sync_engine.apply_manual_offset(&mut subtitle, -15.0);
    assert!(result.is_ok());

    Ok(())
}

fn create_test_subtitle() -> subx_cli::core::formats::Subtitle {
    use std::time::Duration;
    use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};

    Subtitle {
        entries: vec![SubtitleEntry::new(
            1,
            Duration::from_secs(10),
            Duration::from_secs(12),
            "Test subtitle".to_string(),
        )],
        metadata: SubtitleMetadata::default(),
        format: SubtitleFormatType::Srt,
    }
}
