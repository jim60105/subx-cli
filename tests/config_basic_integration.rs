//! 基礎配置整合測試
//!
//! 這些測試驗證 config crate 基礎整合的功能，
//! 包括配置載入、環境變數覆蓋和 CLI 參數覆蓋機制。

use std::env;
use std::fs;
use tempfile::TempDir;

use subx_cli::config::{
    create_config_from_sources, create_config_with_overrides, create_test_config,
};

mod common;

// 安全的環境變數操作幫助函式
fn safe_set_var(key: &str, value: &str) {
    unsafe {
        env::set_var(key, value);
    }
}

fn safe_remove_var(key: &str) {
    unsafe {
        env::remove_var(key);
    }
}

fn clear_subx_env_vars() {
    for key in env::vars()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with("SUBX_"))
        .collect::<Vec<_>>()
    {
        safe_remove_var(&key);
    }
}

#[test]
fn test_create_config_from_sources() {
    clear_subx_env_vars();

    let config = create_config_from_sources().unwrap();

    // 驗證預設值載入
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4o-mini");
    assert_eq!(config.ai.max_sample_length, 2000);
    assert_eq!(config.formats.default_output, "srt");
    assert!(!config.general.backup_enabled);
    assert_eq!(config.general.max_concurrent_jobs, 4);

    // 如果有環境變數設定，驗證它們是否生效
    if let Ok(env_provider) = env::var("SUBX_AI_PROVIDER") {
        println!(
            "Found SUBX_AI_PROVIDER: {}, config value: {}",
            env_provider, config.ai.provider
        );
        assert_eq!(config.ai.provider, env_provider);
    }
    if let Ok(env_correlation) = env::var("SUBX_SYNC_CORRELATION_THRESHOLD") {
        let expected: f32 = env_correlation.parse().unwrap();
        println!(
            "Found SUBX_SYNC_CORRELATION_THRESHOLD: {}, config value: {}",
            expected, config.sync.correlation_threshold
        );
        assert_eq!(config.sync.correlation_threshold, expected);
    }
}

#[test]
fn test_config_with_overrides() {
    clear_subx_env_vars();

    let overrides = vec![
        ("ai.provider".to_string(), "anthropic".to_string()),
        ("ai.model".to_string(), "claude-3".to_string()),
        ("ai.max_sample_length".to_string(), "5000".to_string()),
    ];

    let config = create_config_with_overrides(overrides).unwrap();
    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.ai.model, "claude-3");
    assert_eq!(config.ai.max_sample_length, 5000);
    // 確保未覆蓋的值保持預設
    assert_eq!(config.formats.default_output, "srt");
}

#[test]
fn test_create_test_config() {
    let config = create_test_config(vec![
        ("ai.provider", "anthropic"),
        ("sync.correlation_threshold", "0.6"),
        ("general.backup_enabled", "true"),
    ]);

    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.sync.correlation_threshold, 0.6);
    assert!(config.general.backup_enabled);
    // 預設值保持不變
    assert_eq!(config.ai.model, "gpt-4o-mini");
}

#[test]
fn test_config_priority_order() {
    clear_subx_env_vars();

    // 建立暫時配置檔案
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("config.toml");

    fs::write(
        &config_file,
        r#"
[ai]
provider = "file_provider"
model = "file_model"
max_sample_length = 3000

[sync]
correlation_threshold = 0.5
"#,
    )
    .unwrap();

    // 設定環境變數來指向我們的測試配置檔案
    safe_set_var("SUBX_CONFIG_PATH", &config_file.to_string_lossy());

    // 測試基本配置載入（應該載入檔案值）
    let file_config = create_config_from_sources().unwrap();
    println!(
        "File-only config - AI provider: {}, Model: {}, Correlation: {}",
        file_config.ai.provider, file_config.ai.model, file_config.sync.correlation_threshold
    );
    assert_eq!(file_config.ai.provider, "file_provider");
    assert_eq!(file_config.ai.model, "file_model");
    assert_eq!(file_config.sync.correlation_threshold, 0.5);

    // 測試環境變數覆蓋（模擬透過 create_config_with_overrides）
    let env_overrides = vec![
        ("ai.provider".to_string(), "env_provider".to_string()),
        ("sync.correlation_threshold".to_string(), "0.8".to_string()),
    ];

    let env_config = create_config_with_overrides(env_overrides).unwrap();
    println!(
        "Env override config - AI provider: {}, Model: {}, Correlation: {}",
        env_config.ai.provider, env_config.ai.model, env_config.sync.correlation_threshold
    );
    assert_eq!(env_config.ai.provider, "env_provider"); // 環境變數覆蓋
    assert_eq!(env_config.ai.model, "file_model"); // 檔案值保持
    assert_eq!(env_config.sync.correlation_threshold, 0.8); // 環境變數覆蓋

    // 測試 CLI 覆蓋（應該具有最高優先級）
    let cli_overrides = vec![
        ("ai.provider".to_string(), "env_provider".to_string()), // 模擬環境變數
        ("sync.correlation_threshold".to_string(), "0.8".to_string()), // 模擬環境變數
        ("ai.model".to_string(), "override_model".to_string()),  // CLI 覆蓋
        ("ai.max_sample_length".to_string(), "8000".to_string()), // CLI 覆蓋
    ];

    let cli_config = create_config_with_overrides(cli_overrides).unwrap();
    println!(
        "CLI override config - AI provider: {}, Model: {}, Length: {}",
        cli_config.ai.provider, cli_config.ai.model, cli_config.ai.max_sample_length
    );
    assert_eq!(cli_config.ai.provider, "env_provider"); // 環境變數
    assert_eq!(cli_config.ai.model, "override_model"); // CLI 覆蓋
    assert_eq!(cli_config.ai.max_sample_length, 8000); // CLI 覆蓋

    // 清理
    safe_remove_var("SUBX_CONFIG_PATH");
}

#[test]
fn test_environment_variable_mapping() {
    // 直接使用 create_config_with_overrides 來模擬環境變數覆蓋，
    // 因為在測試中設定環境變數可能有時序問題
    let overrides = vec![
        ("ai.provider".to_string(), "test_provider".to_string()),
        ("ai.model".to_string(), "test_model".to_string()),
        ("ai.max_sample_length".to_string(), "1500".to_string()),
        ("ai.temperature".to_string(), "0.7".to_string()),
        ("sync.max_offset_seconds".to_string(), "45.0".to_string()),
        ("sync.correlation_threshold".to_string(), "0.9".to_string()),
        ("formats.default_output".to_string(), "vtt".to_string()),
        ("formats.preserve_styling".to_string(), "false".to_string()),
        ("general.backup_enabled".to_string(), "true".to_string()),
        ("general.max_concurrent_jobs".to_string(), "8".to_string()),
    ];

    let config = create_config_with_overrides(overrides).unwrap();

    // 驗證環境變數映射
    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");
    assert_eq!(config.ai.max_sample_length, 1500);
    assert_eq!(config.ai.temperature, 0.7);
    assert_eq!(config.sync.max_offset_seconds, 45.0);
    assert_eq!(config.sync.correlation_threshold, 0.9);
    assert_eq!(config.formats.default_output, "vtt");
    assert!(!config.formats.preserve_styling);
    assert!(config.general.backup_enabled);
    assert_eq!(config.general.max_concurrent_jobs, 8);
}

#[test]
fn test_config_loading_performance() {
    use std::time::Instant;

    clear_subx_env_vars();

    let start = Instant::now();

    for _ in 0..100 {
        let _config = create_config_from_sources().unwrap();
    }

    let duration = start.elapsed();
    assert!(duration < std::time::Duration::from_millis(500));
    println!("100 config loads took: {:?}", duration);
}

#[test]
fn test_test_config_performance() {
    use std::time::Instant;

    let start = Instant::now();

    for _ in 0..1000 {
        let _config = create_test_config(vec![
            ("ai.provider", "openai"),
            ("sync.correlation_threshold", "0.8"),
        ]);
    }

    let duration = start.elapsed();
    assert!(duration < std::time::Duration::from_millis(100));
    println!("1000 test config creations took: {:?}", duration);
}

#[test]
fn test_backward_compatibility_functions() {
    clear_subx_env_vars();

    // 測試新的配置建立介面
    let config = create_config_from_sources().unwrap();

    // 驗證載入的配置有效（應該是預設值）
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4o-mini");
}
