//! Comprehensive tests for configuration command-line arguments.
//!
//! This module provides complete test coverage for the ConfigArgs and ConfigAction
//! types, testing all variants and their functionality according to the testing
//! guidelines in `docs/testing-guidelines.md`.

use subx_cli::cli::{ConfigAction, ConfigArgs};

mod config_args_tests {
    use super::*;

    #[test]
    fn test_config_args_creation() {
        // Test Set action
        let set_args = ConfigArgs {
            action: ConfigAction::Set {
                key: "ai.provider".to_string(),
                value: "openai".to_string(),
            },
        };

        match &set_args.action {
            ConfigAction::Set { key, value } => {
                assert_eq!(key, "ai.provider");
                assert_eq!(value, "openai");
            }
            _ => panic!("Expected Set action"),
        }

        // Test Get action
        let get_args = ConfigArgs {
            action: ConfigAction::Get {
                key: "ai.model".to_string(),
            },
        };

        match &get_args.action {
            ConfigAction::Get { key } => {
                assert_eq!(key, "ai.model");
            }
            _ => panic!("Expected Get action"),
        }

        // Test List action
        let list_args = ConfigArgs {
            action: ConfigAction::List,
        };

        match &list_args.action {
            ConfigAction::List => {
                // List action has no parameters
            }
            _ => panic!("Expected List action"),
        }

        // Test Reset action
        let reset_args = ConfigArgs {
            action: ConfigAction::Reset,
        };

        match &reset_args.action {
            ConfigAction::Reset => {
                // Reset action has no parameters
            }
            _ => panic!("Expected Reset action"),
        }
    }

    #[test]
    fn test_config_action_debug_implementation() {
        let set_action = ConfigAction::Set {
            key: "test.key".to_string(),
            value: "test_value".to_string(),
        };
        let debug_output = format!("{:?}", set_action);
        assert!(debug_output.contains("Set"));
        assert!(debug_output.contains("test.key"));
        assert!(debug_output.contains("test_value"));

        let get_action = ConfigAction::Get {
            key: "another.key".to_string(),
        };
        let debug_output = format!("{:?}", get_action);
        assert!(debug_output.contains("Get"));
        assert!(debug_output.contains("another.key"));

        let list_action = ConfigAction::List;
        let debug_output = format!("{:?}", list_action);
        assert!(debug_output.contains("List"));

        let reset_action = ConfigAction::Reset;
        let debug_output = format!("{:?}", reset_action);
        assert!(debug_output.contains("Reset"));
    }

    #[test]
    fn test_config_args_debug_implementation() {
        let args = ConfigArgs {
            action: ConfigAction::Set {
                key: "debug.test".to_string(),
                value: "debug_value".to_string(),
            },
        };

        let debug_output = format!("{:?}", args);
        assert!(debug_output.contains("ConfigArgs"));
        assert!(debug_output.contains("Set"));
        assert!(debug_output.contains("debug.test"));
        assert!(debug_output.contains("debug_value"));
    }

    #[test]
    fn test_config_action_variants() {
        // Test all ConfigAction variants exist and can be created
        let variants = vec![
            ConfigAction::Set {
                key: "test.set".to_string(),
                value: "set_value".to_string(),
            },
            ConfigAction::Get {
                key: "test.get".to_string(),
            },
            ConfigAction::List,
            ConfigAction::Reset,
        ];

        assert_eq!(variants.len(), 4);

        // Verify each variant
        for (i, action) in variants.iter().enumerate() {
            match (i, action) {
                (0, ConfigAction::Set { key, value }) => {
                    assert_eq!(key, "test.set");
                    assert_eq!(value, "set_value");
                }
                (1, ConfigAction::Get { key }) => {
                    assert_eq!(key, "test.get");
                }
                (2, ConfigAction::List) => {
                    // List has no fields to check
                }
                (3, ConfigAction::Reset) => {
                    // Reset has no fields to check
                }
                _ => panic!("Unexpected variant at index {}", i),
            }
        }
    }

    #[test]
    fn test_config_action_set_with_various_keys() {
        let test_cases = vec![
            ("ai.provider", "openai"),
            ("ai.model", "gpt-4.1"),
            ("ai.api_key", "sk-test123"),
            ("general.timeout", "30"),
            ("sync.max_offset_seconds", "15.0"),
            ("parallel.max_workers", "4"),
            ("general.backup_enabled", "true"),
        ];

        for (key, value) in test_cases {
            let action = ConfigAction::Set {
                key: key.to_string(),
                value: value.to_string(),
            };

            match action {
                ConfigAction::Set { key: k, value: v } => {
                    assert_eq!(k, key);
                    assert_eq!(v, value);
                }
                _ => panic!("Expected Set action"),
            }
        }
    }

    #[test]
    fn test_config_action_get_with_various_keys() {
        let test_keys = vec![
            "ai.provider",
            "ai.model",
            "ai.api_key",
            "general.timeout",
            "sync.max_offset_seconds",
            "parallel.max_workers",
            "general.backup_enabled",
        ];

        for key in test_keys {
            let action = ConfigAction::Get {
                key: key.to_string(),
            };

            match action {
                ConfigAction::Get { key: k } => {
                    assert_eq!(k, key);
                }
                _ => panic!("Expected Get action"),
            }
        }
    }

    #[test]
    fn test_config_action_with_empty_strings() {
        // Test Set action with empty key and value
        let set_empty_key = ConfigAction::Set {
            key: "".to_string(),
            value: "value".to_string(),
        };

        match set_empty_key {
            ConfigAction::Set { key, value } => {
                assert!(key.is_empty());
                assert_eq!(value, "value");
            }
            _ => panic!("Expected Set action"),
        }

        let set_empty_value = ConfigAction::Set {
            key: "key".to_string(),
            value: "".to_string(),
        };

        match set_empty_value {
            ConfigAction::Set { key, value } => {
                assert_eq!(key, "key");
                assert!(value.is_empty());
            }
            _ => panic!("Expected Set action"),
        }

        // Test Get action with empty key
        let get_empty_key = ConfigAction::Get {
            key: "".to_string(),
        };

        match get_empty_key {
            ConfigAction::Get { key } => {
                assert!(key.is_empty());
            }
            _ => panic!("Expected Get action"),
        }
    }

    #[test]
    fn test_config_action_with_special_characters() {
        let special_cases = vec![
            ("key.with.dots", "value with spaces"),
            ("key-with-dashes", "value_with_underscores"),
            ("key_with_underscores", "value-with-dashes"),
            ("key123", "value456"),
            ("UPPERCASE.KEY", "UPPERCASE_VALUE"),
            ("mixed.Case.Key", "Mixed_Case_Value"),
        ];

        for (key, value) in special_cases {
            let set_action = ConfigAction::Set {
                key: key.to_string(),
                value: value.to_string(),
            };

            match set_action {
                ConfigAction::Set { key: k, value: v } => {
                    assert_eq!(k, key);
                    assert_eq!(v, value);
                }
                _ => panic!("Expected Set action"),
            }

            let get_action = ConfigAction::Get {
                key: key.to_string(),
            };

            match get_action {
                ConfigAction::Get { key: k } => {
                    assert_eq!(k, key);
                }
                _ => panic!("Expected Get action"),
            }
        }
    }

    #[test]
    fn test_config_args_with_all_actions() {
        let test_cases = vec![
            ConfigArgs {
                action: ConfigAction::Set {
                    key: "test.set".to_string(),
                    value: "test_value".to_string(),
                },
            },
            ConfigArgs {
                action: ConfigAction::Get {
                    key: "test.get".to_string(),
                },
            },
            ConfigArgs {
                action: ConfigAction::List,
            },
            ConfigArgs {
                action: ConfigAction::Reset,
            },
        ];

        for (i, args) in test_cases.iter().enumerate() {
            match (i, &args.action) {
                (0, ConfigAction::Set { key, value }) => {
                    assert_eq!(key, "test.set");
                    assert_eq!(value, "test_value");
                }
                (1, ConfigAction::Get { key }) => {
                    assert_eq!(key, "test.get");
                }
                (2, ConfigAction::List) => {
                    // List has no parameters
                }
                (3, ConfigAction::Reset) => {
                    // Reset has no parameters
                }
                _ => panic!("Unexpected case at index {}", i),
            }
        }
    }
}
