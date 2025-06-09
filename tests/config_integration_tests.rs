//! 配置系統整合測試

use log::debug;
use std::env;
use subx_cli::config::{init_config_manager, load_config};
use tempfile::TempDir;

/// 重置全域配置管理器（測試專用）
fn reset_config_manager() {
    // 清除環境變數
    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_API_KEY");
    env::remove_var("OPENAI_BASE_URL");
    env::remove_var("SUBX_AI_MODEL");
}

#[test]
fn test_full_config_integration() {
    // Initialize logger for test
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    reset_config_manager();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 建立測試配置檔案
    let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
max_sample_length = 3000

[general]
backup_enabled = true
max_concurrent_jobs = 8
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Windows 特定修復：確保檔案完全寫入
    #[cfg(windows)]
    std::thread::sleep(std::time::Duration::from_millis(100));

    // 添加詳細除錯資訊
    debug!("Original config file path: {:?}", config_path);
    debug!(
        "Config file exists before canonicalize: {}",
        config_path.exists()
    );

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap();
        debug!("Config file content:\n{}", content);
        debug!("Config file size: {} bytes", content.len());
    }

    let config_path_str = config_path
        .canonicalize()
        .unwrap_or_else(|e| {
            debug!("Canonicalize failed: {}, using original path", e);
            config_path.clone()
        })
        .to_str()
        .unwrap()
        .to_string();

    debug!("Final config path string: {}", config_path_str);
    debug!("Setting SUBX_CONFIG_PATH environment variable");
    env::set_var("SUBX_CONFIG_PATH", &config_path_str);
    env::set_var("OPENAI_API_KEY", "env-api-key");

    // 驗證環境變數設定
    match env::var("SUBX_CONFIG_PATH") {
        Ok(path) => debug!("SUBX_CONFIG_PATH = {}", path),
        Err(e) => debug!("Failed to read SUBX_CONFIG_PATH: {}", e),
    }

    debug!("Calling init_config_manager()");
    // 測試完整流程
    assert!(init_config_manager().is_ok());

    debug!("Calling load_config()");
    let config = load_config().unwrap();

    // 添加詳細的配置值除錯資訊
    debug!("Loaded config values:");
    debug!("  config.ai.model = {}", config.ai.model);
    debug!(
        "  config.ai.max_sample_length = {}",
        config.ai.max_sample_length
    );
    debug!("  config.ai.provider = {}", config.ai.provider);
    debug!(
        "  config.general.max_concurrent_jobs = {}",
        config.general.max_concurrent_jobs
    );
    debug!("  config.ai.api_key = {:?}", config.ai.api_key);

    // 驗證檔案配置載入
    assert_eq!(config.ai.model, "gpt-4");
    assert_eq!(config.ai.max_sample_length, 3000);
    assert_eq!(config.general.max_concurrent_jobs, 8);

    // 驗證環境變數覆蓋
    assert_eq!(config.ai.api_key, Some("env-api-key".to_string()));

    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_API_KEY");
}

#[test]
fn test_base_url_unified_config_integration() {
    // Initialize logger for test
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    reset_config_manager();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // 建立測試配置檔案，包含 base_url 設定
    let config_content = r#"
[ai]
provider = "openai"
model = "gpt-4"
base_url = "https://api.custom.com/v1"
"#;

    std::fs::write(&config_path, config_content).unwrap();

    // Windows 特定修復：確保檔案完全寫入
    #[cfg(windows)]
    std::thread::sleep(std::time::Duration::from_millis(100));

    let config_path_str = config_path
        .canonicalize()
        .unwrap_or(config_path.clone())
        .to_str()
        .unwrap()
        .to_string();
    env::set_var("SUBX_CONFIG_PATH", &config_path_str);
    env::set_var("OPENAI_BASE_URL", "https://env-override.com/v1");

    // 測試統一配置系統
    assert!(init_config_manager().is_ok());
    let config = load_config().unwrap();

    // 驗證環境變數覆蓋檔案設定
    assert_eq!(config.ai.base_url, "https://env-override.com/v1");

    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_BASE_URL");
}
