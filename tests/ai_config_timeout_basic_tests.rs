use subx_cli::config::{AIConfig, Config};

#[test]
fn test_ai_config_new_field() {
    // Test if new request_timeout_seconds field exists and has correct default value
    let ai_config = AIConfig::default();
    assert_eq!(ai_config.request_timeout_seconds, 120);

    let config = Config::default();
    assert_eq!(config.ai.request_timeout_seconds, 120);
}

#[test]
fn test_ai_config_timeout_validation() {
    use subx_cli::config::validator::validate_config;

    // Test valid configuration
    let mut config = Config::default();
    config.ai.request_timeout_seconds = 60;
    assert!(validate_config(&config).is_ok());

    // Test invalid configuration - too small
    config.ai.request_timeout_seconds = 5;
    assert!(validate_config(&config).is_err());

    // Test invalid configuration - too large
    config.ai.request_timeout_seconds = 700;
    assert!(validate_config(&config).is_err());
}
