use subx_cli::config::service::ConfigService;
use subx_cli::config::{TestConfigService, validate_config, validate_field};

#[test]
fn test_complete_validation_flow() {
    let config_service = TestConfigService::default();

    // Test that invalid values are rejected
    let result = config_service.set_config_value("ai.temperature", "3.0");
    assert!(result.is_err());

    // Test that valid values are accepted
    let result = config_service.set_config_value("ai.temperature", "0.8");
    assert!(result.is_ok());

    // Test complete configuration validation
    let config = config_service.get_config().unwrap();
    let result = validate_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_field_validation_integration() {
    // Test AI fields
    assert!(validate_field("ai.provider", "openai").is_ok());
    assert!(validate_field("ai.provider", "invalid").is_err());
    assert!(validate_field("ai.temperature", "0.8").is_ok());
    assert!(validate_field("ai.temperature", "3.0").is_err());

    // Test sync fields
    assert!(validate_field("sync.vad.sensitivity", "0.7").is_ok());
    assert!(validate_field("sync.vad.sensitivity", "1.5").is_err());
    assert!(validate_field("sync.vad.padding_chunks", "3").is_ok());
    assert!(validate_field("sync.vad.padding_chunks", "11").is_err()); // Exceeds max

    // Test unknown field
    assert!(validate_field("unknown.field", "value").is_err());
}

#[test]
fn test_service_validation_integration() {
    let config_service = TestConfigService::default();

    // Test setting multiple valid values
    assert!(
        config_service
            .set_config_value("ai.provider", "openai")
            .is_ok()
    );
    assert!(config_service.set_config_value("ai.model", "gpt-4").is_ok());
    assert!(
        config_service
            .set_config_value("ai.temperature", "0.7")
            .is_ok()
    );
    assert!(
        config_service
            .set_config_value("sync.vad.sensitivity", "0.8")
            .is_ok()
    );

    // Verify configuration is valid
    let config = config_service.get_config().unwrap();
    assert!(validate_config(&config).is_ok());

    // Verify values were set correctly
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
    assert_eq!(config.ai.temperature, 0.7);
    assert_eq!(config.sync.vad.sensitivity, 0.8);
}

#[test]
fn test_validation_error_messages() {
    // Test that validation errors provide useful messages
    let result = validate_field("ai.temperature", "3.0");
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("temperature") || error_msg.contains("range"));

    let result = validate_field("sync.vad.padding_chunks", "11");
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("padding_chunks") || error_msg.contains("range"));
}
