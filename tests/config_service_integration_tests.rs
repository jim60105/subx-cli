//! Integration tests for the configuration service system.
//!
//! These tests verify that the new dependency injection-based configuration
//! system works correctly in various scenarios.

use std::sync::Arc;
use subx_cli::App;
use subx_cli::config::{
    ConfigService, ProductionConfigService, TestConfigBuilder, TestConfigService,
};

#[test]
fn test_production_config_service_creation() {
    let service =
        ProductionConfigService::new().expect("Failed to create production config service");
    let config = service.get_config().expect("Failed to get config");

    // Verify default values are loaded
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4.1-mini");
    assert_eq!(config.formats.default_output, "srt");
}

#[test]
fn test_test_config_service_creation() {
    let service = TestConfigService::with_defaults();
    let config = service.get_config().expect("Failed to get config");

    // Verify default values
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4.1-mini");
}

#[test]
fn test_test_config_service_with_custom_ai() {
    let service = TestConfigService::with_ai_settings("anthropic", "claude-3");
    let config = service.get_config().expect("Failed to get config");

    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.ai.model, "claude-3");
}

#[test]
fn test_config_builder_basic() {
    let config = TestConfigBuilder::new()
        .with_ai_provider("test_provider")
        .with_ai_model("test_model")
        .build_config();

    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");
}

#[test]
fn test_config_builder_comprehensive() {
    let config = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_ai_model("gpt-4.1")
        .with_ai_api_key("test-key")
        .with_max_sample_length(5000)
        .with_analysis_window(30)
        // .with_max_offset removed
        .with_max_concurrent_jobs(8)
        .with_task_queue_size(200)
        .build_config();

    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4.1");
    assert_eq!(config.ai.api_key, Some("test-key".to_string()));
    assert_eq!(config.ai.max_sample_length, 5000);
    assert_eq!(config.sync.analysis_window_seconds, 30);
    assert_eq!(config.sync.default_method, "whisper");
    assert_eq!(config.general.max_concurrent_jobs, 8);
    assert_eq!(config.parallel.task_queue_size, 200);
}

#[test]
fn test_config_service_reload() {
    let service = ProductionConfigService::new().expect("Failed to create service");

    // First load
    let config1 = service.get_config().expect("Failed to get initial config");

    // Reload should work
    service.reload().expect("Failed to reload config");

    // Second load after reload
    let config2 = service
        .get_config()
        .expect("Failed to get config after reload");

    // Configs should be consistent (assuming no external changes)
    assert_eq!(config1.ai.provider, config2.ai.provider);
    assert_eq!(config1.ai.model, config2.ai.model);
}

#[test]
fn test_app_creation_with_config_service() {
    let config_service = Arc::new(TestConfigService::with_ai_settings(
        "test_provider",
        "test_model",
    ));
    let app = App::new(config_service);

    let config = app.get_config().expect("Failed to get config from app");
    assert_eq!(config.ai.provider, "test_provider");
    assert_eq!(config.ai.model, "test_model");
}

#[test]
fn test_app_creation_with_production_config() {
    let app =
        App::new_with_production_config().expect("Failed to create app with production config");
    let config = app.get_config().expect("Failed to get config");

    // Should have default production values
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4.1-mini");
}

#[test]
fn test_config_service_isolation() {
    // Create two different test services
    let service1 = TestConfigService::with_ai_settings("provider1", "model1");
    let service2 = TestConfigService::with_ai_settings("provider2", "model2");

    let config1 = service1.get_config().expect("Failed to get config1");
    let config2 = service2.get_config().expect("Failed to get config2");

    // They should have different values
    assert_eq!(config1.ai.provider, "provider1");
    assert_eq!(config1.ai.model, "model1");
    assert_eq!(config2.ai.provider, "provider2");
    assert_eq!(config2.ai.model, "model2");
}

#[test]
fn test_config_builder_service_creation() {
    let service = TestConfigBuilder::new()
        .with_ai_provider("builder_test")
        .with_analysis_window(30)
        .build_service();

    let config = service.get_config().expect("Failed to get config");
    assert_eq!(config.ai.provider, "builder_test");
    assert_eq!(config.sync.analysis_window_seconds, 30);
}

// Test using the new macros (temporarily disabled due to scope issues)
#[test]
fn test_manual_config_service_usage() {
    let service = TestConfigBuilder::new()
        .with_ai_provider("macro_test")
        .build_service();

    let config = service.get_config().expect("Failed to get config");
    assert_eq!(config.ai.provider, "macro_test");
}

#[test]
fn test_manual_default_config() {
    let service = TestConfigService::with_defaults();
    let config = service.get_config().expect("Failed to get config");
    assert_eq!(config.ai.provider, "openai");
}

#[test]
fn test_manual_ai_config() {
    let service = TestConfigService::with_ai_settings("anthropic", "claude-3");
    let config = service.get_config().expect("Failed to get config");
    assert_eq!(config.ai.provider, "anthropic");
    assert_eq!(config.ai.model, "claude-3");
}
