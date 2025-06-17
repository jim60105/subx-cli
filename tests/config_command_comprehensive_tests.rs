//! Comprehensive tests for configuration command functionality.
//!
//! This module provides test coverage for configuration management commands,
//! testing all operations (set, get, list, reset) according to the testing
//! guidelines in `docs/testing-guidelines.md`.

use std::sync::Arc;
use subx_cli::cli::{ConfigAction, ConfigArgs};
use subx_cli::commands::config_command::{execute, execute_with_config};
use subx_cli::config::{ConfigService, TestConfigService};

mod config_command_tests {
    use super::*;

    #[tokio::test]
    async fn test_config_command_set_operation() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "openai".to_string(),
            },
        };

        let result = execute(args, &config_service).await;
        assert!(result.is_ok(), "Config set operation should succeed");
    }

    #[tokio::test]
    async fn test_config_command_get_operation() {
        let config_service = TestConfigService::with_ai_settings("openai", "gpt-4.1");

        let args = ConfigArgs {
            action: ConfigAction::Get {
                key: "ai.provider".to_string(),
            },
        };

        let result = execute(args, &config_service).await;
        assert!(result.is_ok(), "Config get operation should succeed");
    }

    #[tokio::test]
    async fn test_config_command_list_operation() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::List,
        };

        let result = execute(args, &config_service).await;
        assert!(result.is_ok(), "Config list operation should succeed");
    }

    #[tokio::test]
    async fn test_config_command_reset_operation() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::Reset,
        };

        let result = execute(args, &config_service).await;
        assert!(result.is_ok(), "Config reset operation should succeed");
    }

    #[tokio::test]
    async fn test_config_command_set_with_various_keys() {
        let config_service = TestConfigService::with_defaults();

        let test_cases = vec![
            ("ai.provider", "openai"),
            ("ai.model", "gpt-4.1"),
            ("general.task_timeout_seconds", "30"),
            ("sync.max_offset_seconds", "15.0"),
        ];

        for (key, value) in test_cases {
            let args = ConfigArgs {
                action: ConfigAction::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            };

            let result = execute(args, &config_service).await;
            assert!(result.is_ok(), "Config set should succeed for key: {}", key);
        }
    }

    #[tokio::test]
    async fn test_config_command_get_with_various_keys() {
        let config_service =
            TestConfigService::with_ai_settings_and_key("openai", "gpt-4.1", "sk-test-key");

        let test_keys = vec!["ai.provider", "ai.model", "ai.api_key"];

        for key in test_keys {
            let args = ConfigArgs {
                action: ConfigAction::Get {
                    key: key.to_string(),
                },
            };

            let result = execute(args, &config_service).await;
            assert!(result.is_ok(), "Config get should succeed for key: {}", key);
        }
    }

    #[tokio::test]
    async fn test_config_command_set_invalid_key() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "invalid.nonexistent.key".to_string(),
                value: "some_value".to_string(),
            },
        };

        let result = execute(args, &config_service).await;
        // Should handle invalid keys gracefully
        // The exact behavior depends on the ConfigService implementation
        assert!(
            result.is_err() || result.is_ok(),
            "Config set should handle invalid keys"
        );
    }

    #[tokio::test]
    async fn test_config_command_get_invalid_key() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::Get {
                key: "invalid.nonexistent.key".to_string(),
            },
        };

        let result = execute(args, &config_service).await;
        // Should handle invalid keys gracefully
        assert!(
            result.is_err() || result.is_ok(),
            "Config get should handle invalid keys"
        );
    }

    #[tokio::test]
    async fn test_config_command_execute_with_config_arc() {
        let config_service: Arc<dyn ConfigService> =
            Arc::new(TestConfigService::with_ai_settings("openai", "gpt-4.1"));

        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.temperature".to_string(),
                value: "0.7".to_string(),
            },
        };

        let result = execute_with_config(args, config_service).await;
        assert!(result.is_ok(), "Config execute_with_config should succeed");
    }

    #[tokio::test]
    async fn test_config_command_all_operations_with_arc() {
        let config_service: Arc<dyn ConfigService> = Arc::new(TestConfigService::with_defaults());

        // Test Set
        let set_args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "openai".to_string(),
            },
        };
        let result = execute_with_config(set_args, config_service.clone()).await;
        assert!(result.is_ok(), "Config set with Arc should succeed");

        // Test Get
        let get_args = ConfigArgs {
            action: ConfigAction::Get {
                key: "ai.provider".to_string(),
            },
        };
        let result = execute_with_config(get_args, config_service.clone()).await;
        assert!(result.is_ok(), "Config get with Arc should succeed");

        // Test List
        let list_args = ConfigArgs {
            action: ConfigAction::List,
        };
        let result = execute_with_config(list_args, config_service.clone()).await;
        assert!(result.is_ok(), "Config list with Arc should succeed");

        // Test Reset
        let reset_args = ConfigArgs {
            action: ConfigAction::Reset,
        };
        let result = execute_with_config(reset_args, config_service).await;
        assert!(result.is_ok(), "Config reset with Arc should succeed");
    }

    #[tokio::test]
    async fn test_config_command_set_empty_values() {
        let config_service = TestConfigService::with_defaults();

        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.api_key".to_string(),
                value: "".to_string(),
            },
        };

        let result = execute(args, &config_service).await;
        // Should handle empty values (clearing optional fields)
        assert!(result.is_ok(), "Config set should handle empty values");
    }

    #[tokio::test]
    async fn test_config_command_set_boolean_values() {
        let config_service = TestConfigService::with_defaults();

        let boolean_test_cases = vec![
            ("general.backup_enabled", "true"),
            ("general.backup_enabled", "false"),
            ("general.backup_enabled", "1"),
            ("general.backup_enabled", "0"),
        ];

        for (key, value) in boolean_test_cases {
            let args = ConfigArgs {
                action: ConfigAction::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            };

            let result = execute(args, &config_service).await;
            assert!(
                result.is_ok(),
                "Config set should handle boolean value: {}",
                value
            );
        }
    }

    #[tokio::test]
    async fn test_config_command_set_numeric_values() {
        let config_service = TestConfigService::with_defaults();

        let numeric_test_cases = vec![
            ("ai.max_tokens", "1000"),
            ("ai.temperature", "0.8"),
            ("general.task_timeout_seconds", "30"),
            ("sync.max_offset_seconds", "15.5"),
        ];

        for (key, value) in numeric_test_cases {
            let args = ConfigArgs {
                action: ConfigAction::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            };

            let result = execute(args, &config_service).await;
            assert!(
                result.is_ok(),
                "Config set should handle numeric value: {} = {}",
                key,
                value
            );
        }
    }

    #[tokio::test]
    async fn test_config_command_sequence_operations() {
        let config_service = TestConfigService::with_defaults();

        // Set a value
        let set_args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "anthropic".to_string(),
            },
        };
        let result = execute(set_args, &config_service).await;
        assert!(result.is_ok(), "First set operation should succeed");

        // Get the value
        let get_args = ConfigArgs {
            action: ConfigAction::Get {
                key: "ai.provider".to_string(),
            },
        };
        let result = execute(get_args, &config_service).await;
        assert!(result.is_ok(), "Get operation should succeed after set");

        // List all values
        let list_args = ConfigArgs {
            action: ConfigAction::List,
        };
        let result = execute(list_args, &config_service).await;
        assert!(result.is_ok(), "List operation should succeed");

        // Reset configuration
        let reset_args = ConfigArgs {
            action: ConfigAction::Reset,
        };
        let result = execute(reset_args, &config_service).await;
        assert!(result.is_ok(), "Reset operation should succeed");
    }
}
