//! Integration tests for dependency injection configuration system.
//!
//! This module tests the new configuration service system and dependency injection
//! implementation, ensuring that the system works correctly with isolated test
//! configurations and supports parallel test execution.

use std::sync::Arc;
use subx_cli::config::{ConfigService, ProductionConfigService, TestConfigService};
use subx_cli::{App, Result};
use tokio::fs;

mod common;
use common::cli_helpers::{CLITestHelper, OutputValidator};

#[tokio::test]
async fn test_app_with_production_config_service() -> Result<()> {
    // Test creating an app with production config service
    let app = App::new_with_production_config()?;
    let config = app.get_config()?;

    // Verify basic configuration structure
    assert!(!config.ai.provider.is_empty());
    assert!(!config.ai.model.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_app_with_test_config_service() -> Result<()> {
    // Create a test configuration service
    let test_config_service = Arc::new(TestConfigService::with_ai_settings(
        "test-provider",
        "test-model",
    ));

    // Create app with test config service
    let app = App::new(test_config_service);
    let config = app.get_config()?;

    // Verify test configuration is used
    assert_eq!(config.ai.provider, "test-provider");
    assert_eq!(config.ai.model, "test-model");

    Ok(())
}

#[tokio::test]
async fn test_config_service_isolation() -> Result<()> {
    // Create two different test configuration services
    let config_service_1 = Arc::new(TestConfigService::with_ai_settings("provider1", "model1"));
    let config_service_2 = Arc::new(TestConfigService::with_ai_settings("provider2", "model2"));

    // Create apps with different config services
    let app1 = App::new(config_service_1);
    let app2 = App::new(config_service_2);

    // Verify configurations are isolated
    let config1 = app1.get_config()?;
    let config2 = app2.get_config()?;

    assert_eq!(config1.ai.provider, "provider1");
    assert_eq!(config1.ai.model, "model1");

    assert_eq!(config2.ai.provider, "provider2");
    assert_eq!(config2.ai.model, "model2");

    Ok(())
}

#[tokio::test]
async fn test_cli_helper_with_dependency_injection() -> Result<()> {
    // Create CLI test helper with custom configuration
    let mut helper = CLITestHelper::with_ai_settings("test-ai", "test-model");

    // Create test workspace
    let workspace = helper.create_isolated_test_workspace().await?;

    // Verify workspace creation
    assert!(workspace.exists());
    assert!(workspace.join("media").exists());
    assert!(workspace.join("subtitles").exists());
    assert!(workspace.join("test_config.toml").exists());

    // Verify configuration service
    let config = helper.config_service().get_config()?;
    assert_eq!(config.ai.provider, "test-ai");
    assert_eq!(config.ai.model, "test-model");

    Ok(())
}

#[tokio::test]
async fn test_parallel_config_service_safety() -> Result<()> {
    // Test that multiple config services can run in parallel without interference
    let mut handles = Vec::new();

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let provider = format!("provider{}", i);
            let model = format!("model{}", i);

            let config_service = Arc::new(TestConfigService::with_ai_settings(&provider, &model));
            let app = App::new(config_service);
            let config = app.get_config().unwrap();

            assert_eq!(config.ai.provider, provider);
            assert_eq!(config.ai.model, model);

            i
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.unwrap();
        assert_eq!(result, i);
    }

    Ok(())
}

#[tokio::test]
async fn test_config_service_reload() -> Result<()> {
    // Test configuration reloading functionality
    let config_service = Arc::new(TestConfigService::with_defaults());

    // Initial config load
    let config1 = config_service.get_config()?;
    assert!(!config1.ai.provider.is_empty());

    // Reload configuration
    config_service.reload()?;

    // Verify config is still accessible after reload
    let config2 = config_service.get_config()?;
    assert_eq!(config1.ai.provider, config2.ai.provider);
    assert_eq!(config1.ai.model, config2.ai.model);

    Ok(())
}

#[tokio::test]
async fn test_production_config_service_creation() -> Result<()> {
    // Test production config service creation
    let config_service = ProductionConfigService::new()?;

    // Verify it can load configuration
    let config = config_service.get_config()?;
    assert!(!config.ai.provider.is_empty());

    // Test reload functionality
    config_service.reload()?;
    let config_after_reload = config_service.get_config()?;
    assert_eq!(config.ai.provider, config_after_reload.ai.provider);

    Ok(())
}

#[tokio::test]
async fn test_cli_helper_file_creation() -> Result<()> {
    let mut helper = CLITestHelper::new();

    // Test subtitle file creation
    let subtitle_content = r#"1
00:00:01,000 --> 00:00:03,000
Test subtitle content

2
00:00:04,000 --> 00:00:06,000
Second subtitle entry
"#;

    let subtitle_path = helper
        .create_subtitle_file("test.srt", subtitle_content)
        .await?;
    assert!(subtitle_path.exists());

    let content = fs::read_to_string(&subtitle_path).await?;
    assert!(content.contains("Test subtitle content"));

    // Test video file creation
    let video_path = helper.create_video_file("test.mp4").await?;
    assert!(video_path.exists());

    Ok(())
}

#[tokio::test]
async fn test_output_validator() {
    let validator = OutputValidator::new()
        .expect_pattern(r"✓.*completed")
        .expect_pattern(r"success")
        .reject_pattern(r"error")
        .reject_pattern(r"failed");

    // Test successful output
    let success_output = "✓ Operation completed successfully";
    let result = validator.validate(success_output);
    assert!(result.is_valid());
    assert_eq!(result.failures().len(), 0);

    // Test failed output
    let failure_output = "✗ Operation failed with error";
    let result = validator.validate(failure_output);
    assert!(!result.is_valid());
    assert!(!result.failures().is_empty());
}

#[tokio::test]
async fn test_config_service_thread_safety() -> Result<()> {
    let config_service = Arc::new(TestConfigService::with_ai_settings("thread-test", "model"));

    // Clone the service for multiple threads
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let service = Arc::clone(&config_service);
            tokio::spawn(async move {
                // Each thread should be able to get the same configuration
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.provider, "thread-test");
                i
            })
        })
        .collect();

    // Wait for all threads to complete
    for handle in handles {
        handle.await.unwrap();
    }

    Ok(())
}

/// Test that the new system eliminates the need for unsafe code.
#[tokio::test]
async fn test_no_unsafe_code_needed() -> Result<()> {
    // Create multiple isolated test environments simultaneously
    let mut test_configs = Vec::new();

    for i in 0..5 {
        let provider = format!("provider{}", i);
        let model = format!("model{}", i);
        let config_service = Arc::new(TestConfigService::with_ai_settings(&provider, &model));
        test_configs.push((i, config_service));
    }

    // Verify each has its own isolated configuration
    for (i, config_service) in test_configs {
        let config = config_service.get_config()?;
        assert_eq!(config.ai.provider, format!("provider{}", i));
        assert_eq!(config.ai.model, format!("model{}", i));
    }

    // No unsafe code or global resets needed!
    Ok(())
}
