use subx_cli::config::Config;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::{AIProvider, AnalysisRequest, ContentSample, VerificationRequest};

mod common;
use common::mock_azure_openai_helper::MockAzureOpenAITestHelper;
use common::test_data_generators::MatchResponseGenerator;

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

/// Tests for prompt building and response parsing
#[cfg(test)]
mod azure_openai_parsing_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_openai_match_result_parsing() {
        let mock = MockAzureOpenAITestHelper::new().await;
        let multiple_matches_response = MatchResponseGenerator::multiple_matches();
        mock.mock_chat_completion_success(&multiple_matches_response)
            .await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
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
        mock.mock_chat_completion_success(&no_matches_response)
            .await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
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
        let confidence_response =
            r#"{"score": 0.75, "factors": ["Moderate confidence due to partial content match"]}"#;
        mock.mock_chat_completion_success(confidence_response).await;

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = mock.deployment_id().to_string();
        config.ai.base_url = mock.base_url();
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
