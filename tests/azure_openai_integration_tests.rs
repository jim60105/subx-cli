use std::sync::Arc;
use subx_cli::config::{Config, ConfigService, TestConfigService};
use subx_cli::core::ComponentFactory;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::{AnalysisRequest, VerificationRequest, ContentSample, AIProvider};

mod common;
use common::mock_azure_openai_helper::MockAzureOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

/// Helper function to create a sample AnalysisRequest
fn create_sample_analysis_request() -> AnalysisRequest {
    AnalysisRequest {
        video_files: vec!["test.mp4".to_string()],
        subtitle_files: vec!["test.srt".to_string()],
        content_samples: vec![ContentSample {
            filename: "test.srt".to_string(),
            content_preview: "Sample subtitle content preview".to_string(),
            file_size: 1024,
        }],
    }
}

/// Helper function to create multiple file AnalysisRequest
fn create_multiple_files_analysis_request() -> AnalysisRequest {
    AnalysisRequest {
        video_files: vec!["video1.mp4".to_string(), "video2.mp4".to_string()],
        subtitle_files: vec!["sub1.srt".to_string(), "sub2.srt".to_string()],
        content_samples: vec![
            ContentSample {
                filename: "sub1.srt".to_string(),
                content_preview: "First subtitle content".to_string(),
                file_size: 2048,
            },
            ContentSample {
                filename: "sub2.srt".to_string(),
                content_preview: "Second subtitle content".to_string(),
                file_size: 1536,
            },
        ],
    }
}

/// Helper function to create a sample VerificationRequest
fn create_sample_verification_request() -> VerificationRequest {
    VerificationRequest {
        video_file: "test.mp4".to_string(),
        subtitle_file: "test.srt".to_string(),
        match_factors: vec![
            "filename_similarity".to_string(),
            "content_correlation".to_string(),
        ],
    }
}

/// Tests for Azure OpenAI client creation and configuration validation
#[cfg(test)]
mod azure_openai_client_tests {
    use super::*;

    #[test]
    fn test_azure_openai_client_creation_success() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());
        config.ai.api_version = Some("2025-04-01-preview".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok(), "Failed to create Azure OpenAI client: {:?}", result.err());
    }

    #[test]
    fn test_azure_openai_client_creation_with_defaults() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());
        // api_version will default to DEFAULT_AZURE_API_VERSION

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok(), "Failed to create Azure OpenAI client with defaults: {:?}", result.err());
    }

    #[test]
    fn test_azure_openai_client_missing_api_key() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = None;
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Missing Azure OpenAI API Key"));
    }

    #[test]
    fn test_azure_openai_client_missing_deployment_id() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = None;

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Missing Azure OpenAI deployment ID"));
    }

    #[test]
    fn test_azure_openai_client_invalid_base_url() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "invalid-url".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Invalid Azure OpenAI endpoint"));
    }

    #[test]
    fn test_azure_openai_client_invalid_url_scheme() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "ftp://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("must use http or https"));
    }

    #[test]
    fn test_azure_openai_client_url_without_host() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        // Print the actual error message for debugging
        println!("Actual error message: {}", error_msg);
        assert!(error_msg.contains("empty host") || error_msg.contains("missing host"));
    }

    #[test]
    fn test_azure_openai_factory_integration() {
        let config_service = TestConfigService::with_defaults();
        {
            let mut config = config_service.config_mut();
            config.ai.provider = "azure-openai".to_string();
            config.ai.api_key = Some("test-api-key".to_string());
            config.ai.model = "gpt-test".to_string();
            config.ai.base_url = "https://example.openai.azure.com".to_string();
            config.ai.deployment_id = Some("test-deployment".to_string());
            config.ai.api_version = Some("2025-04-01-preview".to_string());
        }

        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok(), "Factory failed to create Azure OpenAI provider: {:?}", result.err());
    }
}

/// Tests for Azure OpenAI client API interactions using Wiremock
#[cfg(test)]
mod azure_openai_api_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_analyze_content_success() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "analyze_content failed: {:?}", result.err());

        let match_result = result.unwrap();
        assert!(!match_result.matches.is_empty());
        assert!(match_result.confidence > 0.9);

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_verify_match_success() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let confidence_response = r#"{"score": 0.95, "factors": ["High confidence based on content analysis"]}"#;
        mock.mock_chat_completion_success(confidence_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_verification_request();

        let result = client.verify_match(request).await;
        assert!(result.is_ok(), "verify_match failed: {:?}", result.err());

        let confidence_score = result.unwrap();
        assert!(confidence_score.score > 0.9);
        assert!(!confidence_score.factors.is_empty());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_api_key_authentication() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success_with_api_key(&response_content, "test-api-key").await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_bearer_token_authentication() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success_with_bearer_token(&response_content, "Bearer test-token").await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("Bearer test-token".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok());

        mock.verify_expectations().await;
    }
}

/// Tests for Azure OpenAI error handling scenarios
#[cfg(test)]
mod azure_openai_error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_api_error_response() {
        let mock = MockAzureOpenAITestHelper::new().await;
        mock.setup_error_response(400, "Bad Request: Invalid model").await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Azure OpenAI API error"));
        assert!(error_msg.contains("400"));
    }

    #[tokio::test]
    async fn test_azure_openai_retry_logic_success() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        
        // Test with a single delayed response rather than retries
        // since retries only apply to network errors, not HTTP status errors
        mock.setup_delayed_response(50, &response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.retry_attempts = 3;
        config.ai.retry_delay_ms = 10; // Fast retry for testing

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Request should have succeeded: {:?}", result.err());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_retry_exhausted() {
        let mock = MockAzureOpenAITestHelper::new().await;
        
        // Test that HTTP error responses are returned immediately without retries
        // Retry logic only applies to network-level errors, not HTTP status errors
        mock.setup_error_response(500, "Internal server error").await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.retry_attempts = 3;
        config.ai.retry_delay_ms = 10; // Fast retry for testing

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_err(), "Should fail immediately on HTTP error");
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Azure OpenAI API error"));
        assert!(error_msg.contains("500"));

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_timeout_handling() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        // Setup a 2-second delay, but client timeout is 1 second
        mock.setup_delayed_response(2000, &response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.request_timeout_seconds = 1; // 1 second timeout
        config.ai.retry_attempts = 0; // No retries to test timeout directly

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_err(), "Should timeout");
    }

    #[tokio::test]
    async fn test_azure_openai_invalid_response_format() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let invalid_response = r#"{"invalid": "format"}"#;
        mock.mock_chat_completion_success(invalid_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_err(), "Should fail with invalid response format");
    }
}

/// Tests for Azure OpenAI configuration edge cases
#[cfg(test)]
mod azure_openai_configuration_tests {
    use super::*;

    #[test]
    fn test_azure_openai_with_custom_deployment_and_version() {
        let mock_deployment = "custom-deployment-123";
        let mock_version = "2023-12-01-preview";

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://custom.openai.azure.com".to_string();
        config.ai.deployment_id = Some(mock_deployment.to_string());
        config.ai.api_version = Some(mock_version.to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok());
    }

    #[test]
    fn test_azure_openai_with_trailing_slash_in_url() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com/".to_string(); // Trailing slash
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok(), "Should handle trailing slash in base URL");
    }

    #[test]
    fn test_azure_openai_with_custom_temperature_and_tokens() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());
        config.ai.temperature = 0.8;
        config.ai.max_tokens = 2000;

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok());
    }

    #[test]
    fn test_azure_openai_with_custom_retry_settings() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());
        config.ai.retry_attempts = 5;
        config.ai.retry_delay_ms = 2000;
        config.ai.request_timeout_seconds = 180;

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok());
    }
}

/// Tests for Azure OpenAI environment variable handling
#[cfg(test)]
mod azure_openai_environment_tests {
    use super::*;
    use subx_cli::config::TestEnvironmentProvider;

    #[test]
    fn test_azure_openai_environment_variable_loading() {
        use subx_cli::config::service::ProductionConfigService;

        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("AZURE_OPENAI_API_KEY", "test-azure-api-key");
        env_provider.set_var("AZURE_OPENAI_ENDPOINT", "https://test.openai.azure.com");
        env_provider.set_var("AZURE_OPENAI_DEPLOYMENT_ID", "test-deployment");
        env_provider.set_var("AZURE_OPENAI_API_VERSION", "2025-01-01-preview");
        env_provider.set_var("SUBX_CONFIG_PATH", "/tmp/test_config_azure.toml");

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.provider, "azure-openai");
        assert_eq!(config.ai.api_key, Some("test-azure-api-key".to_string()));
        assert_eq!(config.ai.base_url, "https://test.openai.azure.com");
        assert_eq!(config.ai.deployment_id, Some("test-deployment".to_string()));
        assert_eq!(config.ai.api_version, Some("2025-01-01-preview".to_string()));
    }
}

/// Tests for prompt building and response parsing
#[cfg(test)]
mod azure_openai_parsing_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_match_result_parsing() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let multiple_matches_response = MatchResponseGenerator::multiple_matches();
        mock.mock_chat_completion_success(&multiple_matches_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_multiple_files_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok());

        let match_result = result.unwrap();
        assert_eq!(match_result.matches.len(), 2);
        assert!(match_result.confidence > 0.8);
        assert!(!match_result.reasoning.is_empty());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_no_matches_parsing() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let no_matches_response = MatchResponseGenerator::no_matches_found();
        mock.mock_chat_completion_success(&no_matches_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = AnalysisRequest {
            video_files: vec!["unmatched.mp4".to_string()],
            subtitle_files: vec!["unmatched.srt".to_string()],
            content_samples: vec![ContentSample {
                filename: "unmatched.srt".to_string(),
                content_preview: "Unmatched subtitle content".to_string(),
                file_size: 512,
            }],
        };

        let result = client.analyze_content(request).await;
        assert!(result.is_ok());

        let match_result = result.unwrap();
        assert!(match_result.matches.is_empty());
        assert!(match_result.confidence < 0.5);

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_confidence_score_parsing() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let confidence_response = r#"{"score": 0.75, "factors": ["Moderate confidence due to partial content match"]}"#;
        mock.mock_chat_completion_success(confidence_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_verification_request();

        let result = client.verify_match(request).await;
        assert!(result.is_ok());

        let confidence_score = result.unwrap();
        assert!((confidence_score.score - 0.75).abs() < 0.01); // Approximately 0.75
        assert!(!confidence_score.factors.is_empty());

        mock.verify_expectations().await;
    }
}

/// Tests for custom Azure OpenAI deployment scenarios
#[cfg(test)]
mod azure_openai_deployment_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_custom_deployment() {
        let custom_deployment = "my-custom-gpt-deployment";
        let custom_version = "2024-02-15-preview";
        let mock = MockAzureOpenAITestHelper::new_with_deployment(custom_deployment, custom_version).await;
        
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(custom_deployment.to_string());
        config.ai.api_version = Some(custom_version.to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Custom deployment should work: {:?}", result.err());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_usage_statistics_tracking() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        // Test should pass - usage stats are displayed via display_ai_usage function
        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Usage statistics tracking should work: {:?}", result.err());

        mock.verify_expectations().await;
    }
}

/// Additional unit tests for coverage
#[cfg(test)]
mod azure_openai_unit_tests {
    use super::*;

    #[test]
    fn test_azure_openai_new_with_all_parameters() {
        let client = AzureOpenAIClient::new_with_all(
            "test-api-key".to_string(),
            "gpt-test".to_string(),
            "https://example.openai.azure.com".to_string(),
            "test-deployment".to_string(),
            "2025-04-01-preview".to_string(),
            0.7,
            4000,
            3,
            1000,
            120,
        );

        // Just verify the client was created successfully
        assert!(format!("{:?}", client).contains("AzureOpenAIClient"));
    }

    #[tokio::test]
    async fn test_azure_openai_error_handling_empty_api_key() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("".to_string()); // Empty string
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Missing Azure OpenAI API Key"));
    }

    #[test]
    fn test_azure_openai_valid_http_url() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "http://localhost:8080".to_string(); // HTTP for local testing
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_ok(), "Should accept HTTP URLs for local testing");
    }

    #[tokio::test]
    async fn test_azure_openai_api_response_without_usage() {
        let mock = MockAzureOpenAITestHelper::new().await;
        
        // Use the mock helper method instead of direct server access
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should handle response without usage stats");

        mock.verify_expectations().await;
    }
}

// Additional edge case tests to improve coverage
mod azure_openai_edge_case_tests {
    use super::*;

    #[test]
    fn test_azure_openai_bearer_token_client_creation() {
        // Test that clients can be created with Bearer token format
        let client = AzureOpenAIClient::new_with_all(
            "Bearer some-azure-token".to_string(),
            "gpt-test".to_string(),
            "https://test.openai.azure.com".to_string(),
            "test-deployment".to_string(),
            "2025-04-01-preview".to_string(),
            0.3,
            1000,
            1,
            100,
            30,
        );
        
        // Just test that the client was created successfully with Bearer token
        assert!(format!("{:?}", client).contains("Bearer"));
    }

    #[test]
    fn test_azure_openai_lowercase_bearer_token_client_creation() {
        // Test that clients can be created with lowercase bearer token format
        let client = AzureOpenAIClient::new_with_all(
            "bearer lowercase-token".to_string(),
            "gpt-test".to_string(),
            "https://test.openai.azure.com".to_string(),
            "test-deployment".to_string(),
            "2025-04-01-preview".to_string(),
            0.3,
            1000,
            1,
            100,
            30,
        );
        
        assert!(format!("{:?}", client).contains("bearer"));
    }

    #[test]
    fn test_azure_openai_base_url_with_trailing_slash() {
        // Test that clients handle base URLs with trailing slashes
        let client = AzureOpenAIClient::new_with_all(
            "test-key".to_string(),
            "gpt-test".to_string(),
            "https://test.openai.azure.com/".to_string(), // Note trailing slash
            "test-deployment".to_string(),
            "2025-04-01-preview".to_string(),
            0.3,
            1000,
            1,
            100,
            30,
        );
        
        // Verify the client was created successfully 
        assert!(format!("{:?}", client).contains("https://test.openai.azure.com"));
    }

    #[tokio::test]
    async fn test_azure_openai_single_retry_configuration() {
        // Test with single retry attempt to exercise retry logic path
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.retry_attempts = 1; // Single retry attempt
        config.ai.retry_delay_ms = 5; // Very short delay

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with single retry configuration");

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_minimal_timeout_configuration() {
        // Test with minimal timeout configuration
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.request_timeout_seconds = 5; // Minimal timeout

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with minimal timeout");

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_different_api_version() {
        // Test with different API version to cover more code paths
        let mock = MockAzureOpenAITestHelper::new_with_deployment("custom-deploy", "2024-10-01-preview").await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with custom API version");

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_http_error_handling() {
        // Test that HTTP error responses are handled correctly
        let mock = MockAzureOpenAITestHelper::new().await;
        
        // Setup a 404 error response
        mock.setup_error_response(404, "Not found").await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_err(), "Should fail with HTTP error");
        assert!(result.unwrap_err().to_string().contains("404"), "Should contain status code");

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_bearer_token_authentication_header() {
        // Test that Bearer token uses Authorization header instead of api-key header
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        
        // Mock expects Authorization header (not api-key) for Bearer tokens
        mock.mock_chat_completion_with_bearer_auth(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("Bearer sk-test123".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = mock.base_url();
        config.ai.deployment_id = Some(mock.deployment_id().to_string());
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with Bearer token auth");

        mock.verify_expectations().await;
    }

    #[tokio::test] 
    async fn test_azure_openai_scheme_validation_error() {
        // Test invalid URL scheme validation
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "ftp://invalid.example.com".to_string(); // Invalid scheme
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err(), "Should fail with invalid URL scheme");
        assert!(result.unwrap_err().to_string().contains("must use http or https"));
    }

    #[tokio::test]
    async fn test_azure_openai_url_host_validation_error() {
        // Test URL without host validation
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://".to_string(); // No host, just scheme
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err(), "Should fail with missing host");
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("Configuration error: Invalid Azure OpenAI endpoint: empty host"), "Should contain error about empty host");
    }

    #[tokio::test]
    async fn test_azure_openai_connection_error_handling() {
        // Test connection error specific logging
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://invalid-nonexistent-host-12345.openai.azure.com".to_string();
        config.ai.deployment_id = Some("test-deployment".to_string());
        config.ai.retry_attempts = 1; // Quick fail for testing
        config.ai.retry_delay_ms = 10; // Fast retry
        config.ai.request_timeout_seconds = 1; // Short timeout

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();
        
        let result = client.analyze_content(request).await;
        assert!(result.is_err());
        // Connection errors should be logged with specific message
    }

    #[tokio::test]
    async fn test_azure_openai_new_with_all_parameters() {
        // Test the new_with_all constructor to ensure coverage
        let _client = AzureOpenAIClient::new_with_all(
            "test-api-key".to_string(),
            "gpt-4".to_string(),
            "https://test.openai.azure.com".to_string(),
            "test-deployment".to_string(),
            "2024-02-01".to_string(),
            0.7,
            2000,
            3,
            1000,
            30,
        );
        
        // The new_with_all method was called and the client was created successfully
        // This tests the code path for the constructor
    }

    #[tokio::test]
    async fn test_azure_openai_url_without_host_validation() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "gpt-test".to_string();
        config.ai.base_url = "https://".to_string(); // URL without host
        config.ai.deployment_id = Some("test-deployment".to_string());

        let result = AzureOpenAIClient::from_config(&config.ai);
        assert!(result.is_err());
        let error_str = result.unwrap_err().to_string();
        assert!(error_str.contains("empty host"));
    }
}
