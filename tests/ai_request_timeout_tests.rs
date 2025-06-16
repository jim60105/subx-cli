use subx_cli::config::{Config, ConfigService, TestConfigService};

#[test]
fn test_ai_request_timeout_configuration() {
    // Test default timeout value
    let config = Config::default();
    assert_eq!(config.ai.request_timeout_seconds, 120);

    // Test configuration service support
    let service = TestConfigService::with_defaults();

    // Test getting configuration value
    let timeout_value = service
        .get_config_value("ai.request_timeout_seconds")
        .unwrap();
    assert_eq!(timeout_value, "120");

    // Test setting configuration value
    service
        .set_config_value("ai.request_timeout_seconds", "180")
        .unwrap();
    let new_timeout_value = service
        .get_config_value("ai.request_timeout_seconds")
        .unwrap();
    assert_eq!(new_timeout_value, "180");

    // Test configuration validation
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.request_timeout_seconds, 180);
}

#[test]
fn test_ai_request_timeout_validation() {
    let service = TestConfigService::with_defaults();

    // Test valid values
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "60")
            .is_ok()
    );
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "300")
            .is_ok()
    );
    assert!(
        service
            .set_config_value("ai.request_timeout_seconds", "600")
            .is_ok()
    );

    // Test invalid values - too small
    let result = service.set_config_value("ai.request_timeout_seconds", "5");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Value 5 is out of range [10, 600]")
    );

    // Test invalid values - too large
    let result = service.set_config_value("ai.request_timeout_seconds", "700");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Value 700 is out of range [10, 600]")
    );
}
