//! Config system integration tests: get_config_value method validation

use subx_cli::config::{ConfigService, TestConfigBuilder};

/// Validates that get_config_value correctly returns expected values
#[test]
fn test_get_config_value_success() {
    let service = TestConfigBuilder::new()
        .with_ai_provider("provX")
        .with_ai_model("modelY")
        .with_ai_api_key("keyZ")
        .with_ai_base_url("https://api.test")
        .build_service();

    assert_eq!(service.get_config_value("ai.provider").unwrap(), "provX");
    assert_eq!(service.get_config_value("ai.model").unwrap(), "modelY");
    assert_eq!(service.get_config_value("ai.api_key").unwrap(), "keyZ");
    assert_eq!(
        service.get_config_value("ai.base_url").unwrap(),
        "https://api.test"
    );
}

/// Validates that get_config_value returns error for unknown keys
#[test]
fn test_get_config_value_unknown_key() {
    let service = TestConfigBuilder::new().build_service();
    let err = service.get_config_value("unknown.key");
    assert!(err.is_err(), "Expected error for unknown key");
}
