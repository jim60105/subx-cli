use subx_cli::config::Config;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::openai::OpenAIClient;
use subx_cli::services::ai::openrouter::OpenRouterClient;
/// Tests for HTTP retry trait implementation and retry functionality
/// This test file covers the HttpRetryClient trait and make_http_request_with_retry_impl function
use subx_cli::services::ai::retry::HttpRetryClient;

mod common;

/// Test HttpRetryClient trait for all AI provider types
#[tokio::test]
async fn test_http_retry_client_trait_all_providers() {
    // Test OpenAI client
    let openai_client =
        OpenAIClient::new("test".to_string(), "gpt-4".to_string(), 0.7, 1000, 5, 300);
    assert_eq!(openai_client.retry_attempts(), 5);
    assert_eq!(openai_client.retry_delay_ms(), 300);

    // Test OpenRouter client
    let openrouter_client =
        OpenRouterClient::new("test".to_string(), "claude".to_string(), 0.7, 1000, 7, 500);
    assert_eq!(openrouter_client.retry_attempts(), 7);
    assert_eq!(openrouter_client.retry_delay_ms(), 500);

    // Test Azure OpenAI client
    let mut config = Config::default();
    config.ai.provider = "azure-openai".to_string();
    config.ai.api_key = Some("test".to_string());
    config.ai.model = "deployment".to_string();
    config.ai.base_url = "https://test.azure.com".to_string();
    config.ai.api_version = Some("2024-02-01".to_string());
    config.ai.retry_attempts = 4;
    config.ai.retry_delay_ms = 250;

    let azure_client = AzureOpenAIClient::from_config(&config.ai).unwrap();
    assert_eq!(azure_client.retry_attempts(), 4);
    assert_eq!(azure_client.retry_delay_ms(), 250);
}

/// Test retry configuration variations
#[tokio::test]
async fn test_retry_configuration_variations() {
    // Test with different retry configurations
    let client_1 = OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 1, 100);
    assert_eq!(client_1.retry_attempts(), 1);
    assert_eq!(client_1.retry_delay_ms(), 100);

    let client_2 = OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 10, 2000);
    assert_eq!(client_2.retry_attempts(), 10);
    assert_eq!(client_2.retry_delay_ms(), 2000);
}

/// Test OpenRouter specific retry settings
#[tokio::test]
async fn test_openrouter_retry_settings() {
    let client = OpenRouterClient::new(
        "test-key".to_string(),
        "anthropic/claude-3-haiku".to_string(),
        0.7,
        2000,
        6,
        750,
    );

    assert_eq!(client.retry_attempts(), 6);
    assert_eq!(client.retry_delay_ms(), 750);
}

/// Test Azure OpenAI retry settings from config
#[tokio::test]
async fn test_azure_openai_retry_from_config() {
    let mut config = Config::default();
    config.ai.provider = "azure-openai".to_string();
    config.ai.api_key = Some("test-key".to_string());
    config.ai.model = "gpt-4".to_string();
    config.ai.base_url = "https://example.openai.azure.com".to_string();
    config.ai.api_version = Some("2024-02-01".to_string());
    config.ai.retry_attempts = 8;
    config.ai.retry_delay_ms = 1500;

    let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
    assert_eq!(client.retry_attempts(), 8);
    assert_eq!(client.retry_delay_ms(), 1500);
}

/// Test extreme retry configurations
#[tokio::test]
async fn test_extreme_retry_configurations() {
    // Test zero retries
    let client_zero = OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 0, 1000);
    assert_eq!(client_zero.retry_attempts(), 0);

    // Test very short delay
    let client_short = OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 3, 1);
    assert_eq!(client_short.retry_delay_ms(), 1);

    // Test large retry count
    let client_large =
        OpenRouterClient::new("key".to_string(), "model".to_string(), 0.7, 1000, 100, 50);
    assert_eq!(client_large.retry_attempts(), 100);
}

/// Test trait consistency across different models
#[tokio::test]
async fn test_trait_consistency_across_models() {
    let openai_gpt3 = OpenAIClient::new(
        "key".to_string(),
        "gpt-3.5-turbo".to_string(),
        0.7,
        1000,
        3,
        1000,
    );
    let openai_gpt4 = OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 3, 1000);

    let openrouter_claude = OpenRouterClient::new(
        "key".to_string(),
        "anthropic/claude-3".to_string(),
        0.7,
        1000,
        3,
        1000,
    );
    let openrouter_llama = OpenRouterClient::new(
        "key".to_string(),
        "meta-llama/llama-3".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // All should have the same retry behavior regardless of model
    assert_eq!(openai_gpt3.retry_attempts(), openai_gpt4.retry_attempts());
    assert_eq!(openai_gpt3.retry_delay_ms(), openai_gpt4.retry_delay_ms());

    assert_eq!(
        openrouter_claude.retry_attempts(),
        openrouter_llama.retry_attempts()
    );
    assert_eq!(
        openrouter_claude.retry_delay_ms(),
        openrouter_llama.retry_delay_ms()
    );
}
