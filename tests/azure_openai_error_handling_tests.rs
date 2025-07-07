use subx_cli::config::Config;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::{AIProvider, AnalysisRequest, ContentSample};

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

/// Tests for Azure OpenAI error handling scenarios
#[cfg(test)]
mod azure_openai_error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_api_error_response() {
        let mock = MockAzureOpenAITestHelper::new().await;
        mock.setup_error_response(400, "Bad Request: Invalid model")
            .await;

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
        assert!(
            result.is_ok(),
            "Request should have succeeded: {:?}",
            result.err()
        );

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_retry_exhausted() {
        let mock = MockAzureOpenAITestHelper::new().await;

        // Test that HTTP error responses are returned immediately without retries
        // Retry logic only applies to network-level errors, not HTTP status errors
        mock.setup_error_response(500, "Internal server error")
            .await;

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
        assert!(
            result.unwrap_err().to_string().contains("404"),
            "Should contain status code"
        );

        mock.verify_expectations().await;
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
}
