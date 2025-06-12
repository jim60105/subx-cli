//! Config system integration tests: ProductionConfigService file I/O and reset_to_defaults

#[path = "common/file_managers.rs"]
mod file_managers;
use file_managers::TestFileManager;
use std::sync::Arc;
use subx_cli::config::{
    ConfigService, ProductionConfigService, TestConfigBuilder, TestEnvironmentProvider,
};
use toml;

/// Test loading configuration from a custom file
#[tokio::test]
async fn test_load_config_from_custom_file() {
    let mut fm = TestFileManager::new();
    let dir = fm.create_isolated_test_directory("cfg_load").await.unwrap();
    let path = dir.join("custom.toml");

    // Write custom configuration file
    let custom = TestConfigBuilder::new()
        .with_ai_provider("anthropic")
        .build_config();
    let toml = toml::to_string_pretty(&custom).unwrap();
    tokio::fs::write(&path, toml).await.unwrap();

    // Load through ProductionConfigService with_custom_file
    let svc = ProductionConfigService::new()
        .unwrap()
        .with_custom_file(path.clone())
        .unwrap();
    let loaded = svc.get_config().unwrap();
    assert_eq!(loaded.ai.provider, "anthropic");
}

/// Test that reset_to_defaults overwrites the config file with default content
#[tokio::test]
async fn test_reset_to_defaults_writes_default() {
    let mut fm = TestFileManager::new();
    let dir = fm
        .create_isolated_test_directory("cfg_reset")
        .await
        .unwrap();
    let path = dir.join("cfg_reset.toml");

    // Write invalid or custom content
    tokio::fs::write(&path, "invalid content").await.unwrap();

    // Point SUBX_CONFIG_PATH to the test file
    let mut env = TestEnvironmentProvider::new();
    env.set_var("SUBX_CONFIG_PATH", path.to_str().unwrap());
    let svc = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();
    svc.reset_to_defaults().unwrap();

    let content = tokio::fs::read_to_string(&path).await.unwrap();
    // Default config should include ai.provider item
    assert!(
        content.contains("provider = \"openai\""),
        "reset file content incorrect: {}",
        content
    );
    // Verify loaded as default value
    let parsed: subx_cli::config::Config = toml::from_str(&content).unwrap();
    assert_eq!(
        parsed.ai.provider,
        subx_cli::config::Config::default().ai.provider
    );
}
