//! Integration tests for the config command set functionality.

use subx_cli::cli::{ConfigAction, ConfigArgs};
use subx_cli::commands::config_command;
use subx_cli::config::{ConfigService, TestConfigBuilder};

#[tokio::test]
async fn test_config_command_set_success() {
    let config_service = TestConfigBuilder::new().build_service();
    let args = ConfigArgs {
        action: ConfigAction::Set {
            key: "ai.provider".to_string(),
            value: "anthropic".to_string(),
        },
    };

    let result = config_command::execute(args, &config_service).await;
    assert!(result.is_ok());

    // Verify the value was set
    let config = config_service.get_config().unwrap();
    assert_eq!(config.ai.provider, "anthropic");
}

#[tokio::test]
async fn test_config_command_set_invalid_key() {
    let config_service = TestConfigBuilder::new().build_service();
    let args = ConfigArgs {
        action: ConfigAction::Set {
            key: "invalid.key".to_string(),
            value: "value".to_string(),
        },
    };

    let result = config_command::execute(args, &config_service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_command_set_invalid_value() {
    let config_service = TestConfigBuilder::new().build_service();
    let args = ConfigArgs {
        action: ConfigAction::Set {
            key: "ai.temperature".to_string(),
            value: "invalid_number".to_string(),
        },
    };

    let result = config_command::execute(args, &config_service).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_command_set_with_arc() {
    // Use Arc to wrap the TestConfigService for execute_with_config
    let service = std::sync::Arc::new(TestConfigBuilder::new().build_service());
    let args = ConfigArgs {
        action: ConfigAction::Set {
            key: "ai.model".to_string(),
            value: "gpt-4".to_string(),
        },
    };

    let result = config_command::execute_with_config(args, service.clone()).await;
    assert!(result.is_ok());

    // Verify the value was set
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.model, "gpt-4");
}

#[tokio::test]
async fn test_config_command_set_multiple_values() {
    let config_service = TestConfigBuilder::new().build_service();
    let settings = vec![
        ("ai.provider", "openai"),
        ("ai.temperature", "0.7"),
        ("general.backup_enabled", "true"),
        ("parallel.max_workers", "8"),
    ];

    for (key, value) in settings {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: key.to_string(),
                value: value.to_string(),
            },
        };
        let result = config_command::execute(args, &config_service).await;
        assert!(result.is_ok(), "Failed to set {}: {}", key, value);
    }

    // Verify all values were set correctly
    let config = config_service.get_config().unwrap();
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.temperature, 0.7);
    assert_eq!(config.general.backup_enabled, true);
    assert_eq!(config.parallel.max_workers, 8);
}
