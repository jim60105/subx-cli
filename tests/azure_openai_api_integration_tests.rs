use subx_cli::config::{Config, TestConfigService};
use subx_cli::core::ComponentFactory;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::{AIProvider, AnalysisRequest, ContentSample, VerificationRequest};

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
        let confidence_response =
            r#"{"score": 0.95, "factors": ["High confidence based on content analysis"]}"#;
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
        mock.mock_chat_completion_success_with_api_key(&response_content, "test-api-key")
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
        assert!(result.is_ok());

        mock.verify_expectations().await;
    }

    #[tokio::test]
    async fn test_azure_openai_bearer_token_authentication() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let response_content = MatchResponseGenerator::successful_single_match();
        mock.mock_chat_completion_success_with_bearer_token(&response_content, "Bearer test-token")
            .await;

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

/// Tests for Azure OpenAI factory integration
#[cfg(test)]
mod azure_openai_factory_tests {
    use super::*;

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
        assert!(
            result.is_ok(),
            "Factory failed to create Azure OpenAI provider: {:?}",
            result.err()
        );
    }
}
