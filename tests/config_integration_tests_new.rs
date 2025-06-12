//! Configuration system integration tests

use subx_cli::config::{ConfigService, ProductionConfigService, TestConfigBuilder};

mod common;
use common::file_managers::TestFileManager;

#[test]
fn test_config_builder_basic_functionality() {
    // Test basic functionality of the config builder
    let config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4")
        .build_config();

    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
}

#[test]
fn test_config_builder_ai_settings() {
    // Test AI-related settings
    let config = TestConfigBuilder::new()
        .with_ai_provider("anthropic")
        .with_ai_model("claude-3")
        .build_config();

    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.ai.model, "claude-3");
}

#[test]
fn test_config_service_interface() {
    // Test config service interface
    let service = TestConfigBuilder::new()
        .with_ai_provider("test_provider")
        .with_ai_model("test_model")
        .build_service();

    let config = service.get_config().unwrap();
    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");

    // Test reload functionality
    assert!(service.reload().is_ok());
}

#[test]
fn test_production_config_service() {
    // Test production config service
    match ProductionConfigService::new() {
        Ok(service) => {
            let config = service.get_config().unwrap();
            // Verify default config is loaded
            assert!(!config.ai.provider.is_empty());
        }
        Err(_) => {
            // It's normal not to have a config file in the test environment
        }
    }
}

#[test]
fn test_config_with_custom_settings() {
    // Test custom settings
    let config = TestConfigBuilder::new()
        .with_ai_provider("custom_provider")
        .build_config();

    assert_eq!(config.ai.provider, "custom_provider");
}

#[tokio::test]
async fn test_config_with_file_manager() {
    // Test file management with the new test tool
    let mut file_manager = TestFileManager::new();
    let test_dir = file_manager
        .create_isolated_test_directory("config_test")
        .await
        .unwrap();
    let test_dir_path = test_dir.to_path_buf();

    // Create test config file
    let config_path = file_manager
        .create_test_config(&test_dir_path, "test_config.toml", &test_dir_path)
        .await
        .unwrap();

    assert!(config_path.exists());

    // Verify config file content
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[general]"));
    assert!(content.contains("[ai]"));
    assert!(content.contains("[test]"));
}

#[test]
fn test_multiple_config_instances() {
    // Test isolation of multiple config instances
    let config1 = TestConfigBuilder::new()
        .with_ai_provider("provider1")
        .build_config();

    let config2 = TestConfigBuilder::new()
        .with_ai_provider("provider2")
        .build_config();

    // Ensure config instances are isolated
    assert_eq!(config1.ai.provider, "provider1");
    assert_eq!(config2.ai.provider, "provider2");
}

#[test]
fn test_config_builder_chaining() {
    // Test chaining of config builder methods
    let config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4")
        .build_config();

    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
}

#[test]
fn test_parallel_config_creation() {
    // Test parallel creation of config instances
    use std::thread;

    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let config = TestConfigBuilder::new()
                    .with_ai_provider(&format!("provider_{}", i))
                    .build_config();
                config.ai.provider
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        let provider = handle.join().unwrap();
        assert_eq!(provider, format!("provider_{}", i));
    }
}
