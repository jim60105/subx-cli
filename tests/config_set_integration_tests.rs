//! Integration tests for config set operations.

use subx_cli::config::{ConfigService, TestConfigBuilder, validator};
use subx_cli::test_with_config;

#[test]
fn test_set_ai_provider_success() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            // Set AI provider
            let result = config_service.set_config_value("ai.provider", "anthropic");
            assert!(result.is_ok());

            // Verify the value was set
            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.provider, "anthropic");

            // Verify it can be retrieved
            let value = config_service.get_config_value("ai.provider").unwrap();
            assert_eq!(value, "anthropic");
        }
    );
}

#[test]
fn test_set_ai_provider_invalid_value() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("ai.provider", "invalid_provider");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Invalid value"));
        }
    );
}

#[test]
fn test_set_ai_temperature_success() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("ai.temperature", "0.7");
            assert!(result.is_ok());

            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.temperature, 0.7);
        }
    );
}

#[test]
fn test_set_ai_temperature_out_of_range() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("ai.temperature", "1.5");
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("out of range"));
        }
    );
}

#[test]
fn test_set_ai_api_key_success() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("ai.api_key", "sk-1234567890abcdef");
            assert!(result.is_ok());

            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.api_key, Some("sk-1234567890abcdef".to_string()));
        }
    );
}

#[test]
fn test_set_ai_api_key_empty_clears_value() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            // First set a value
            config_service
                .set_config_value("ai.api_key", "sk-1234567890abcdef")
                .unwrap();

            // Then clear it
            let result = config_service.set_config_value("ai.api_key", "");
            assert!(result.is_ok());

            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.api_key, None);
        }
    );
}

#[test]
fn test_set_boolean_values() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            // Test various boolean representations
            let boolean_tests = vec![
                ("true", true),
                ("false", false),
                ("1", true),
                ("0", false),
                ("yes", true),
                ("no", false),
                ("on", true),
                ("off", false),
                ("enabled", true),
                ("disabled", false),
            ];

            for (input, expected) in boolean_tests {
                let result = config_service.set_config_value("general.backup_enabled", input);
                assert!(result.is_ok(), "Failed to set boolean value: {}", input);

                let config = config_service.get_config().unwrap();
                assert_eq!(
                    config.general.backup_enabled, expected,
                    "Wrong boolean value for input: {}",
                    input
                );
            }
        }
    );
}

#[test]
fn test_set_integer_values() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("general.max_concurrent_jobs", "8");
            assert!(result.is_ok());

            let config = config_service.get_config().unwrap();
            assert_eq!(config.general.max_concurrent_jobs, 8);
        }
    );
}

#[test]
fn test_set_unknown_key() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            let result = config_service.set_config_value("unknown.key", "value");
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("Unknown configuration key")
            );
        }
    );
}

#[test]
fn test_set_preserves_other_values() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            // Set initial values
            config_service
                .set_config_value("ai.provider", "openai")
                .unwrap();
            config_service
                .set_config_value("ai.temperature", "0.3")
                .unwrap();

            // Change one value
            config_service
                .set_config_value("ai.provider", "anthropic")
                .unwrap();

            // Verify the other value is preserved
            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.provider, "anthropic");
            assert_eq!(config.ai.temperature, 0.3);
        }
    );
}

#[test]
fn test_set_validates_entire_config() {
    test_with_config!(
        TestConfigBuilder::new(),
        |config_service: &dyn ConfigService| {
            // This test ensures that setting a value doesn't break configuration validation
            let result = config_service.set_config_value("ai.provider", "openai");
            assert!(result.is_ok());

            // The configuration should still be valid after the change
            let config = config_service.get_config().unwrap();
            assert!(validator::validate_config(&config).is_ok());
        }
    );
}
