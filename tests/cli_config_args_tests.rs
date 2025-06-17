//! Tests for CLI configuration arguments parsing and validation.
//!
//! This module provides comprehensive tests for the configuration command-line
//! interface, focusing on argument parsing, validation, and enum variants.
//! All tests follow the testing guidelines and use dependency injection patterns.

use clap::Parser;
use subx_cli::cli::{Cli, Commands, ConfigAction, ConfigArgs};

/// Test configuration args parsing from command line arguments
#[test]
fn test_config_args_set_parsing() {
    let cli = Cli::try_parse_from(["subx", "config", "set", "ai.provider", "openai"]).unwrap();

    if let Commands::Config(config_args) = cli.command {
        if let ConfigAction::Set { key, value } = config_args.action {
            assert_eq!(key, "ai.provider");
            assert_eq!(value, "openai");
        } else {
            panic!("Expected ConfigAction::Set");
        }
    } else {
        panic!("Expected Commands::Config");
    }
}

/// Test configuration args get parsing
#[test]
fn test_config_args_get_parsing() {
    let cli = Cli::try_parse_from(["subx", "config", "get", "ai.provider"]).unwrap();

    if let Commands::Config(config_args) = cli.command {
        if let ConfigAction::Get { key } = config_args.action {
            assert_eq!(key, "ai.provider");
        } else {
            panic!("Expected ConfigAction::Get");
        }
    } else {
        panic!("Expected Commands::Config");
    }
}

/// Test configuration args list parsing
#[test]
fn test_config_args_list_parsing() {
    let cli = Cli::try_parse_from(["subx", "config", "list"]).unwrap();

    if let Commands::Config(config_args) = cli.command {
        if let ConfigAction::List = config_args.action {
            // Success - correct parsing
        } else {
            panic!("Expected ConfigAction::List");
        }
    } else {
        panic!("Expected Commands::Config");
    }
}

/// Test configuration args reset parsing
#[test]
fn test_config_args_reset_parsing() {
    let cli = Cli::try_parse_from(["subx", "config", "reset"]).unwrap();

    if let Commands::Config(config_args) = cli.command {
        if let ConfigAction::Reset = config_args.action {
            // Success - correct parsing
        } else {
            panic!("Expected ConfigAction::Reset");
        }
    } else {
        panic!("Expected Commands::Config");
    }
}

/// Test config args debug formatting
#[test]
fn test_config_args_debug_formatting() {
    let config_args = ConfigArgs {
        action: ConfigAction::Set {
            key: "ai.provider".to_string(),
            value: "openai".to_string(),
        },
    };

    let debug_str = format!("{:?}", config_args);
    assert!(debug_str.contains("ConfigArgs"));
    assert!(debug_str.contains("Set"));
    assert!(debug_str.contains("ai.provider"));
    assert!(debug_str.contains("openai"));
}

/// Test config action variants debug formatting
#[test]
fn test_config_action_debug_formatting() {
    let actions = vec![
        ConfigAction::Set {
            key: "test.key".to_string(),
            value: "test.value".to_string(),
        },
        ConfigAction::Get {
            key: "test.key".to_string(),
        },
        ConfigAction::List,
        ConfigAction::Reset,
    ];

    for action in actions {
        let debug_str = format!("{:?}", action);
        assert!(!debug_str.is_empty());
    }
}

/// Test config args with complex key paths
#[test]
fn test_config_args_complex_key_paths() {
    let test_cases = vec![
        ("ai.provider", "openai"),
        ("ai.api_key", "sk-1234567890"),
        ("sync.max_offset_seconds", "15.0"),
        ("sync.vad.enabled", "true"),
        ("parallel.max_workers", "4"),
        ("general.backup_enabled", "false"),
    ];

    for (key, value) in test_cases {
        let cli = Cli::try_parse_from(["subx", "config", "set", key, value]).unwrap();

        if let Commands::Config(config_args) = cli.command {
            if let ConfigAction::Set {
                key: parsed_key,
                value: parsed_value,
            } = config_args.action
            {
                assert_eq!(parsed_key, key);
                assert_eq!(parsed_value, value);
            } else {
                panic!("Expected ConfigAction::Set for key: {}", key);
            }
        } else {
            panic!("Expected Commands::Config for key: {}", key);
        }
    }
}

/// Test config args with empty values
#[test]
fn test_config_args_empty_values() {
    let cli = Cli::try_parse_from(["subx", "config", "set", "ai.api_key", ""]).unwrap();

    if let Commands::Config(config_args) = cli.command {
        if let ConfigAction::Set { key, value } = config_args.action {
            assert_eq!(key, "ai.api_key");
            assert_eq!(value, "");
        } else {
            panic!("Expected ConfigAction::Set");
        }
    } else {
        panic!("Expected Commands::Config");
    }
}

/// Test config args with special characters in values
#[test]
fn test_config_args_special_characters() {
    let test_cases = vec![
        ("ai.base_url", "https://api.openai.com/v1"),
        ("ai.api_key", "sk-1234567890abcdef!@#$%"),
        ("general.output_pattern", "{name}.{ext}"),
    ];

    for (key, value) in test_cases {
        let cli = Cli::try_parse_from(["subx", "config", "set", key, value]).unwrap();

        if let Commands::Config(config_args) = cli.command {
            if let ConfigAction::Set {
                key: parsed_key,
                value: parsed_value,
            } = config_args.action
            {
                assert_eq!(parsed_key, key);
                assert_eq!(parsed_value, value);
            } else {
                panic!("Expected ConfigAction::Set for key: {}", key);
            }
        } else {
            panic!("Expected Commands::Config for key: {}", key);
        }
    }
}

/// Test config get with various key formats
#[test]
fn test_config_get_various_keys() {
    let test_keys = vec![
        "ai.provider",
        "ai.api_key",
        "sync.max_offset_seconds",
        "general.backup_enabled",
        "parallel.max_workers",
    ];

    for key in test_keys {
        let cli = Cli::try_parse_from(["subx", "config", "get", key]).unwrap();

        if let Commands::Config(config_args) = cli.command {
            if let ConfigAction::Get { key: parsed_key } = config_args.action {
                assert_eq!(parsed_key, key);
            } else {
                panic!("Expected ConfigAction::Get for key: {}", key);
            }
        } else {
            panic!("Expected Commands::Config for key: {}", key);
        }
    }
}

/// Test that config args fail with missing required arguments
#[test]
fn test_config_args_missing_required() {
    // Test set without key
    let result = Cli::try_parse_from(["subx", "config", "set"]);
    assert!(result.is_err());

    // Test set without value
    let result = Cli::try_parse_from(["subx", "config", "set", "ai.provider"]);
    assert!(result.is_err());

    // Test get without key
    let result = Cli::try_parse_from(["subx", "config", "get"]);
    assert!(result.is_err());
}

/// Test config action variants
#[test]
fn test_config_action_variants() {
    let actions = vec![
        ConfigAction::Set {
            key: "test.key".to_string(),
            value: "test.value".to_string(),
        },
        ConfigAction::Get {
            key: "test.key".to_string(),
        },
        ConfigAction::List,
        ConfigAction::Reset,
    ];

    // Test that all variants can be created and debugged
    for action in &actions {
        let debug_output = format!("{:?}", action);
        assert!(!debug_output.is_empty());
    }

    // Test pattern matching works for all variants
    for action in actions {
        match action {
            ConfigAction::Set { key, value } => {
                assert_eq!(key, "test.key");
                assert_eq!(value, "test.value");
            }
            ConfigAction::Get { key } => {
                assert_eq!(key, "test.key");
            }
            ConfigAction::List => {
                // List variant has no fields to check
            }
            ConfigAction::Reset => {
                // Reset variant has no fields to check
            }
        }
    }
}

/// Test struct construction and field access
#[test]
fn test_config_args_construction() {
    let config_args = ConfigArgs {
        action: ConfigAction::Set {
            key: "ai.provider".to_string(),
            value: "openai".to_string(),
        },
    };

    match config_args.action {
        ConfigAction::Set { key, value } => {
            assert_eq!(key, "ai.provider");
            assert_eq!(value, "openai");
        }
        _ => panic!("Expected ConfigAction::Set"),
    }
}

/// Test config action enum pattern matching
#[test]
fn test_config_action_pattern_matching() {
    let actions = vec![
        ConfigAction::Set {
            key: "ai.provider".to_string(),
            value: "openai".to_string(),
        },
        ConfigAction::Get {
            key: "ai.provider".to_string(),
        },
        ConfigAction::List,
        ConfigAction::Reset,
    ];

    for action in actions {
        match action {
            ConfigAction::Set { key, value } => {
                assert!(!key.is_empty());
                assert!(!value.is_empty());
            }
            ConfigAction::Get { key } => {
                assert!(!key.is_empty());
            }
            ConfigAction::List | ConfigAction::Reset => {
                // These variants have no data to validate
            }
        }
    }
}
