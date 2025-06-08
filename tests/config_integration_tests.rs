//! 配置系統整合測試

use std::env;
use subx_cli::config::{init_config_manager, load_config};
use tempfile::TempDir;

#[test]
fn test_full_config_integration() {
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
    env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
    env::set_var("OPENAI_API_KEY", "env-api-key");

    // 測試完整流程
    assert!(init_config_manager().is_ok());
    let config = load_config().unwrap();

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
    env::set_var("SUBX_CONFIG_PATH", config_path.to_str().unwrap());
    env::set_var("OPENAI_BASE_URL", "https://env-override.com/v1");

    // 測試統一配置系統
    assert!(init_config_manager().is_ok());
    let config = load_config().unwrap();

    // 驗證環境變數覆蓋檔案設定
    assert_eq!(config.ai.base_url, "https://env-override.com/v1");

    env::remove_var("SUBX_CONFIG_PATH");
    env::remove_var("OPENAI_BASE_URL");
}
