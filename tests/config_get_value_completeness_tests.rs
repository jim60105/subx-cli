use subx_cli::Result;
use subx_cli::config::{Config, ConfigService, TestConfigService};

#[test]
fn test_get_config_value_ai_configurations() -> Result<()> {
    let mut config = Config::default();
    config.ai.provider = "test_provider".to_string();
    config.ai.model = "test_model".to_string();
    config.ai.api_key = Some("test_key".to_string());
    config.ai.base_url = "https://test.api".to_string();
    config.ai.temperature = 0.8;
    config.ai.max_sample_length = 5000;
    config.ai.retry_attempts = 5;
    config.ai.retry_delay_ms = 2000;

    let service = TestConfigService::new(config);

    // Test all AI configuration items
    assert_eq!(service.get_config_value("ai.provider")?, "test_provider");
    assert_eq!(service.get_config_value("ai.model")?, "test_model");
    assert_eq!(service.get_config_value("ai.api_key")?, "test_key");
    assert_eq!(service.get_config_value("ai.base_url")?, "https://test.api");
    assert_eq!(service.get_config_value("ai.temperature")?, "0.8");
    assert_eq!(service.get_config_value("ai.max_sample_length")?, "5000");
    assert_eq!(service.get_config_value("ai.retry_attempts")?, "5");
    assert_eq!(service.get_config_value("ai.retry_delay_ms")?, "2000");

    Ok(())
}

#[test]
fn test_get_config_value_vad_configurations() -> Result<()> {
    let mut config = Config::default();
    config.sync.vad.enabled = true;
    config.sync.vad.sensitivity = 0.9;
    config.sync.vad.padding_chunks = 5;
    config.sync.vad.min_speech_duration_ms = 150;

    let service = TestConfigService::new(config);

    // Test all VAD configuration items
    assert_eq!(service.get_config_value("sync.vad.enabled")?, "true");
    assert_eq!(service.get_config_value("sync.vad.sensitivity")?, "0.9");
    assert_eq!(service.get_config_value("sync.vad.padding_chunks")?, "5");
    assert_eq!(
        service.get_config_value("sync.vad.min_speech_duration_ms")?,
        "150"
    );

    Ok(())
}

#[test]
fn test_get_config_value_parallel_configurations() -> Result<()> {
    let mut config = Config::default();
    config.parallel.max_workers = 12;
    config.parallel.task_queue_size = 2000;
    config.parallel.enable_task_priorities = true;
    config.parallel.auto_balance_workers = false;
    config.parallel.overflow_strategy = subx_cli::config::OverflowStrategy::Drop;

    let service = TestConfigService::new(config);

    // Test all parallel configuration items
    assert_eq!(service.get_config_value("parallel.max_workers")?, "12");
    assert_eq!(
        service.get_config_value("parallel.task_queue_size")?,
        "2000"
    );
    assert_eq!(
        service.get_config_value("parallel.enable_task_priorities")?,
        "true"
    );
    assert_eq!(
        service.get_config_value("parallel.auto_balance_workers")?,
        "false"
    );
    assert_eq!(
        service.get_config_value("parallel.overflow_strategy")?,
        "Drop"
    );

    Ok(())
}

#[test]
fn test_get_set_config_value_consistency() -> Result<()> {
    let service = TestConfigService::with_defaults();

    // Test all supported configuration keys
    let test_cases = vec![
        // AI configuration (9 items)
        ("ai.provider", "openai"),
        ("ai.model", "gpt-4"),
        (
            "ai.api_key",
            "sk-test-1234567890123456789012345678901234567890",
        ), // Use valid format API key
        ("ai.base_url", "https://api.test.com"),
        ("ai.temperature", "0.7"),
        ("ai.max_sample_length", "8000"),
        ("ai.max_tokens", "4000"),
        ("ai.retry_attempts", "3"),
        ("ai.retry_delay_ms", "1500"),
        // format configuration (4 items)
        ("formats.default_output", "vtt"),
        ("formats.preserve_styling", "true"),
        ("formats.default_encoding", "utf-8"),
        ("formats.encoding_detection_confidence", "0.9"),
        // sync configuration (2 items)
        ("sync.max_offset_seconds", "90"), // Use integer format to avoid floating point representation issues
        ("sync.default_method", "vad"),
        // VAD configuration (5 items)
        ("sync.vad.enabled", "true"),
        ("sync.vad.sensitivity", "0.8"),
        ("sync.vad.padding_chunks", "4"),
        ("sync.vad.min_speech_duration_ms", "120"),
        // general configuration (5 items)
        ("general.backup_enabled", "true"),
        ("general.max_concurrent_jobs", "8"),
        ("general.task_timeout_seconds", "600"),
        ("general.enable_progress_bar", "false"),
        ("general.worker_idle_timeout_seconds", "120"),
        // parallel configuration (5 items)
        ("parallel.max_workers", "6"),
        ("parallel.task_queue_size", "1500"),
        ("parallel.enable_task_priorities", "true"),
        ("parallel.auto_balance_workers", "false"),
        ("parallel.overflow_strategy", "Expand"),
    ];

    for (key, value) in test_cases {
        // Test set then get
        service.set_config_value(key, value)?;

        let retrieved = service.get_config_value(key)?;
        assert_eq!(retrieved, value, "Value mismatch for {}", key);
    }

    Ok(())
}

#[test]
fn test_unsupported_config_key_error() -> Result<()> {
    let service = TestConfigService::with_defaults();

    // Test unsupported configuration keys
    let result = service.get_config_value("unknown.key");
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Unknown configuration key"));

    Ok(())
}

#[test]
fn test_max_offset_seconds_get_set() -> Result<()> {
    let service = TestConfigService::with_defaults();

    // Test setting and getting max_offset_seconds
    service.set_config_value("sync.max_offset_seconds", "120")?;
    assert_eq!(service.get_config_value("sync.max_offset_seconds")?, "120");

    // Test setting smaller value
    service.set_config_value("sync.max_offset_seconds", "30")?;
    assert_eq!(service.get_config_value("sync.max_offset_seconds")?, "30");

    Ok(())
}
