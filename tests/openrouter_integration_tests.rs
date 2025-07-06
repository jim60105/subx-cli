use subx_cli::config::{DEFAULT_FREE_MODEL, TestConfigService};
use subx_cli::core::ComponentFactory;

#[tokio::test]
async fn test_openrouter_client_creation() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key("openrouter", DEFAULT_FREE_MODEL, "test-key");

    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_openrouter_config_validation() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key("openrouter", DEFAULT_FREE_MODEL, "");

    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();

    assert!(result.is_err());
    let error_msg = result.err().unwrap().to_string();
    assert!(
        error_msg.contains("API key cannot be empty")
            || error_msg.contains("Missing OpenRouter API Key")
            || error_msg.contains("AI API key is required"),
        "Unexpected error message: {}",
        error_msg
    );
}
