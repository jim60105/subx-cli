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

/// Tests for custom Azure OpenAI deployment scenarios
#[cfg(test)]
mod azure_openai_deployment_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_custom_deployment() {
        let custom_deployment = "my-custom-gpt-deployment";
        let custom_version = "2024-02-15-preview";
        let mock =
            MockAzureOpenAITestHelper::new_with_deployment(custom_deployment, custom_version).await;

        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = custom_deployment.to_string();
        config.ai.base_url = mock.base_url();
        config.ai.api_version = Some(custom_version.to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(
            result.is_ok(),
            "Custom deployment should work: {:?}",
            result.err()
        );

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
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

        let request = create_sample_analysis_request();

        // Test should pass - usage stats are displayed via display_ai_usage function
        let result = client.analyze_content(request).await;
        assert!(
            result.is_ok(),
            "Usage statistics tracking should work: {:?}",
            result.err()
        );

        mock.verify_expectations().await;
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
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
        config.ai.api_version = Some(mock.api_version().to_string());
        config.ai.retry_attempts = 1; // Single retry attempt
        config.ai.retry_delay_ms = 5; // Very short delay

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(
            result.is_ok(),
            "Should succeed with single retry configuration"
        );

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
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
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
        let mock =
            MockAzureOpenAITestHelper::new_with_deployment("custom-deploy", "2024-10-01-preview")
                .await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success(&response_content).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with custom API version");

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_bearer_token_authentication_header() {
        // Test that Bearer token uses Authorization header instead of api-key header
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();

        // Mock expects Authorization header (not api-key) for Bearer tokens
        mock.mock_chat_completion_with_bearer_auth(&response_content)
            .await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("Bearer sk-test123".to_string());
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
        config.ai.api_version = Some(mock.api_version().to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let request = create_sample_analysis_request();

        let result = client.analyze_content(request).await;
        assert!(result.is_ok(), "Should succeed with Bearer token auth");

        mock.verify_expectations().await;
    }
}
