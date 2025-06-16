use subx_cli::config::{AIConfig, Config};

#[test]
fn test_ai_config_new_field() {
    // 測試新的 request_timeout_seconds 欄位是否存在且有正確的預設值
    let ai_config = AIConfig::default();
    assert_eq!(ai_config.request_timeout_seconds, 120);

    let config = Config::default();
    assert_eq!(config.ai.request_timeout_seconds, 120);
}

#[test]
fn test_ai_config_timeout_validation() {
    use subx_cli::config::validator::validate_config;

    // 測試有效配置
    let mut config = Config::default();
    config.ai.request_timeout_seconds = 60;
    assert!(validate_config(&config).is_ok());

    // 測試無效配置 - 太小
    config.ai.request_timeout_seconds = 5;
    assert!(validate_config(&config).is_err());

    // 測試無效配置 - 太大
    config.ai.request_timeout_seconds = 700;
    assert!(validate_config(&config).is_err());
}
