use subx_cli::config::Config;
use subx_cli::services::ai::azure_openai::AzureOpenAIClient;
use subx_cli::services::ai::openai::OpenAIClient;
use subx_cli::services::ai::openrouter::OpenRouterClient;
/// Tests for AI service trait implementations
/// This test file covers the new PromptBuilder, ResponseParser, and HttpRetryClient trait implementations
use subx_cli::services::ai::prompts::{PromptBuilder, ResponseParser};
use subx_cli::services::ai::retry::HttpRetryClient;
use subx_cli::services::ai::{AnalysisRequest, ContentSample, VerificationRequest};

mod common;

/// Test PromptBuilder trait implementation for OpenAIClient
#[tokio::test]
async fn test_openai_client_prompt_builder_trait() {
    let client = OpenAIClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // Test build_analysis_prompt trait method
    let analysis_request = AnalysisRequest {
        video_files: vec!["ID:file_video1 | Name:movie.mkv | Path:/test/movie.mkv".to_string()],
        subtitle_files: vec!["ID:file_sub1 | Name:movie.srt | Path:/test/movie.srt".to_string()],
        content_samples: vec![ContentSample {
            filename: "movie.srt".to_string(),
            content_preview: "Test subtitle content".to_string(),
            file_size: 1024,
        }],
    };

    let prompt = client.build_analysis_prompt(&analysis_request);
    assert!(prompt.contains("ID:file_video1"));
    assert!(prompt.contains("ID:file_sub1"));
    assert!(prompt.contains("Test subtitle content"));
    assert!(prompt.contains("Video files:"));
    assert!(prompt.contains("Subtitle files:"));
    assert!(prompt.contains("Response format must be JSON"));

    // Test build_verification_prompt trait method
    let verification_request = VerificationRequest {
        video_file: "movie.mkv".to_string(),
        subtitle_file: "movie.srt".to_string(),
        match_factors: vec![
            "filename_similarity".to_string(),
            "content_correlation".to_string(),
        ],
    };

    let verification_prompt = client.build_verification_prompt(&verification_request);
    assert!(verification_prompt.contains("movie.mkv"));
    assert!(verification_prompt.contains("movie.srt"));
    assert!(verification_prompt.contains("filename_similarity"));
    assert!(verification_prompt.contains("content_correlation"));
    assert!(verification_prompt.contains("JSON format"));

    // Test system messages
    assert_eq!(
        OpenAIClient::get_analysis_system_message(),
        "You are a professional subtitle matching assistant that can analyze the correspondence between video and subtitle files."
    );
    assert_eq!(
        OpenAIClient::get_verification_system_message(),
        "Please evaluate the confidence level of subtitle matching and provide a score between 0-1."
    );
}

/// Test ResponseParser trait implementation for OpenAIClient
#[tokio::test]
async fn test_openai_client_response_parser_trait() {
    let client = OpenAIClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // Test parse_match_result trait method
    let match_response = r#"{
        "matches": [
            {
                "video_file_id": "file_video1",
                "subtitle_file_id": "file_sub1",
                "confidence": 0.95,
                "match_factors": ["filename_similarity", "content_correlation"]
            }
        ],
        "confidence": 0.9,
        "reasoning": "Strong match based on filename patterns"
    }"#;

    let match_result = client.parse_match_result(match_response).unwrap();
    assert_eq!(match_result.matches.len(), 1);
    assert_eq!(match_result.matches[0].video_file_id, "file_video1");
    assert_eq!(match_result.matches[0].subtitle_file_id, "file_sub1");
    assert_eq!(match_result.matches[0].confidence, 0.95);
    assert_eq!(match_result.confidence, 0.9);

    // Test parse_confidence_score trait method
    let confidence_response = r#"{
        "score": 0.88,
        "factors": ["filename_similarity", "content_correlation"]
    }"#;

    let confidence_score = client.parse_confidence_score(confidence_response).unwrap();
    assert_eq!(confidence_score.score, 0.88);
    assert_eq!(confidence_score.factors.len(), 2);
    assert!(
        confidence_score
            .factors
            .contains(&"filename_similarity".to_string())
    );
    assert!(
        confidence_score
            .factors
            .contains(&"content_correlation".to_string())
    );
}

/// Test HttpRetryClient trait implementation for OpenAIClient
#[tokio::test]
async fn test_openai_client_http_retry_trait() {
    let client = OpenAIClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
        0.7,
        1000,
        3,
        500,
    );

    // Test trait methods
    assert_eq!(client.retry_attempts(), 3);
    assert_eq!(client.retry_delay_ms(), 500);
}

/// Test PromptBuilder trait implementation for OpenRouterClient
#[tokio::test]
async fn test_openrouter_client_prompt_builder_trait() {
    let client = OpenRouterClient::new(
        "test-key".to_string(),
        "anthropic/claude-3-haiku".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // Test build_analysis_prompt trait method
    let analysis_request = AnalysisRequest {
        video_files: vec![
            "ID:file_video2 | Name:episode1.mkv | Path:/series/episode1.mkv".to_string(),
        ],
        subtitle_files: vec![
            "ID:file_sub2 | Name:episode1.srt | Path:/series/episode1.srt".to_string(),
        ],
        content_samples: vec![],
    };

    let prompt = client.build_analysis_prompt(&analysis_request);
    assert!(prompt.contains("ID:file_video2"));
    assert!(prompt.contains("ID:file_sub2"));
    assert!(prompt.contains("episode1.mkv"));
    assert!(prompt.contains("episode1.srt"));
    assert!(prompt.contains("Video files:"));
    assert!(prompt.contains("Subtitle files:"));

    // Test build_verification_prompt trait method
    let verification_request = VerificationRequest {
        video_file: "episode1.mkv".to_string(),
        subtitle_file: "episode1.srt".to_string(),
        match_factors: vec!["exact_match".to_string()],
    };

    let verification_prompt = client.build_verification_prompt(&verification_request);
    assert!(verification_prompt.contains("episode1.mkv"));
    assert!(verification_prompt.contains("episode1.srt"));
    assert!(verification_prompt.contains("exact_match"));

    // Test system messages
    assert_eq!(
        OpenRouterClient::get_analysis_system_message(),
        "You are a professional subtitle matching assistant that can analyze the correspondence between video and subtitle files."
    );
    assert_eq!(
        OpenRouterClient::get_verification_system_message(),
        "Please evaluate the confidence level of subtitle matching and provide a score between 0-1."
    );
}

/// Test ResponseParser trait implementation for OpenRouterClient
#[tokio::test]
async fn test_openrouter_client_response_parser_trait() {
    let client = OpenRouterClient::new(
        "test-key".to_string(),
        "anthropic/claude-3-haiku".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // Test parse_match_result trait method with complex response
    let match_response = r#"{
        "matches": [
            {
                "video_file_id": "file_video2",
                "subtitle_file_id": "file_sub2",
                "confidence": 0.99,
                "match_factors": ["exact_filename_match"]
            },
            {
                "video_file_id": "file_video3",
                "subtitle_file_id": "file_sub3",
                "confidence": 0.75,
                "match_factors": ["partial_match"]
            }
        ],
        "confidence": 0.87,
        "reasoning": "Multiple matches found with varying confidence levels"
    }"#;

    let match_result = client.parse_match_result(match_response).unwrap();
    assert_eq!(match_result.matches.len(), 2);
    assert_eq!(match_result.matches[0].video_file_id, "file_video2");
    assert_eq!(match_result.matches[1].video_file_id, "file_video3");
    assert_eq!(match_result.confidence, 0.87);

    // Test parse_confidence_score trait method
    let confidence_response = r#"{
        "score": 0.92,
        "factors": ["exact_filename_match", "timing_correlation", "content_similarity"]
    }"#;

    let confidence_score = client.parse_confidence_score(confidence_response).unwrap();
    assert_eq!(confidence_score.score, 0.92);
    assert_eq!(confidence_score.factors.len(), 3);
}

/// Test HttpRetryClient trait implementation for OpenRouterClient
#[tokio::test]
async fn test_openrouter_client_http_retry_trait() {
    let client = OpenRouterClient::new(
        "test-key".to_string(),
        "anthropic/claude-3-haiku".to_string(),
        0.7,
        1000,
        5,
        750,
    );

    // Test trait methods
    assert_eq!(client.retry_attempts(), 5);
    assert_eq!(client.retry_delay_ms(), 750);
}

/// Test PromptBuilder trait implementation for AzureOpenAIClient
#[tokio::test]
async fn test_azure_openai_client_prompt_builder_trait() {
    let mut config = Config::default();
    config.ai.provider = "azure-openai".to_string();
    config.ai.api_key = Some("test-key".to_string());
    config.ai.model = "deployment-name".to_string();
    config.ai.base_url = "https://test.openai.azure.com".to_string();
    config.ai.api_version = Some("2024-02-01".to_string());
    config.ai.retry_attempts = 3;
    config.ai.retry_delay_ms = 1000;

    let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

    // Test build_analysis_prompt trait method
    let analysis_request = AnalysisRequest {
        video_files: vec![
            "ID:file_azure1 | Name:azure_test.mkv | Path:/azure/azure_test.mkv".to_string(),
        ],
        subtitle_files: vec![
            "ID:file_azure_sub1 | Name:azure_test.srt | Path:/azure/azure_test.srt".to_string(),
        ],
        content_samples: vec![ContentSample {
            filename: "azure_test.srt".to_string(),
            content_preview: "Azure OpenAI test subtitle".to_string(),
            file_size: 1024,
        }],
    };

    let prompt = client.build_analysis_prompt(&analysis_request);
    assert!(prompt.contains("ID:file_azure1"));
    assert!(prompt.contains("ID:file_azure_sub1"));
    assert!(prompt.contains("Azure OpenAI test subtitle"));
    assert!(prompt.contains("Video files:"));

    // Test build_verification_prompt trait method
    let verification_request = VerificationRequest {
        video_file: "azure_test.mkv".to_string(),
        subtitle_file: "azure_test.srt".to_string(),
        match_factors: vec!["azure_similarity".to_string()],
    };

    let verification_prompt = client.build_verification_prompt(&verification_request);
    assert!(verification_prompt.contains("azure_test.mkv"));
    assert!(verification_prompt.contains("azure_test.srt"));
    assert!(verification_prompt.contains("azure_similarity"));

    // Test system messages
    assert_eq!(
        AzureOpenAIClient::get_analysis_system_message(),
        "You are a professional subtitle matching assistant that can analyze the correspondence between video and subtitle files."
    );
    assert_eq!(
        AzureOpenAIClient::get_verification_system_message(),
        "Please evaluate the confidence level of subtitle matching and provide a score between 0-1."
    );
}

/// Test ResponseParser trait implementation for AzureOpenAIClient
#[tokio::test]
async fn test_azure_openai_client_response_parser_trait() {
    let mut config = Config::default();
    config.ai.provider = "azure-openai".to_string();
    config.ai.api_key = Some("test-key".to_string());
    config.ai.model = "deployment-name".to_string();
    config.ai.base_url = "https://test.openai.azure.com".to_string();
    config.ai.api_version = Some("2024-02-01".to_string());

    let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

    // Test parse_match_result trait method
    let match_response = r#"{
        "matches": [
            {
                "video_file_id": "file_azure1",
                "subtitle_file_id": "file_azure_sub1",
                "confidence": 0.97,
                "match_factors": ["azure_similarity", "metadata_match"]
            }
        ],
        "confidence": 0.95,
        "reasoning": "Azure OpenAI analysis shows strong correlation"
    }"#;

    let match_result = client.parse_match_result(match_response).unwrap();
    assert_eq!(match_result.matches.len(), 1);
    assert_eq!(match_result.matches[0].video_file_id, "file_azure1");
    assert_eq!(match_result.matches[0].subtitle_file_id, "file_azure_sub1");
    assert_eq!(match_result.matches[0].confidence, 0.97);

    // Test parse_confidence_score trait method
    let confidence_response = r#"{
        "score": 0.94,
        "factors": ["azure_similarity", "metadata_match", "timing_analysis"]
    }"#;

    let confidence_score = client.parse_confidence_score(confidence_response).unwrap();
    assert_eq!(confidence_score.score, 0.94);
    assert_eq!(confidence_score.factors.len(), 3);
}

/// Test HttpRetryClient trait implementation for AzureOpenAIClient
#[tokio::test]
async fn test_azure_openai_client_http_retry_trait() {
    let mut config = Config::default();
    config.ai.provider = "azure-openai".to_string();
    config.ai.api_key = Some("test-key".to_string());
    config.ai.model = "deployment-name".to_string();
    config.ai.base_url = "https://test.openai.azure.com".to_string();
    config.ai.api_version = Some("2024-02-01".to_string());
    config.ai.retry_attempts = 4;
    config.ai.retry_delay_ms = 800;

    let client = AzureOpenAIClient::from_config(&config.ai).unwrap();

    // Test trait methods
    assert_eq!(client.retry_attempts(), 4);
    assert_eq!(client.retry_delay_ms(), 800);
}

/// Test trait error handling for invalid JSON
#[tokio::test]
async fn test_response_parser_error_handling() {
    let client = OpenAIClient::new(
        "test-key".to_string(),
        "gpt-4".to_string(),
        0.7,
        1000,
        3,
        1000,
    );

    // Test parse_match_result with invalid JSON
    let invalid_response = r#"{"invalid": "json structure"}"#;
    let result = client.parse_match_result(invalid_response);
    assert!(result.is_err());

    // Test parse_confidence_score with invalid JSON
    let invalid_confidence = r#"{"wrong": "format"}"#;
    let result = client.parse_confidence_score(invalid_confidence);
    assert!(result.is_err());
}

/// Test trait consistency across all AI providers
#[tokio::test]
async fn test_trait_consistency_across_providers() {
    let openai_client =
        OpenAIClient::new("key".to_string(), "gpt-4".to_string(), 0.7, 1000, 3, 1000);
    let openrouter_client =
        OpenRouterClient::new("key".to_string(), "claude".to_string(), 0.7, 1000, 3, 1000);

    let mut azure_config = Config::default();
    azure_config.ai.provider = "azure-openai".to_string();
    azure_config.ai.api_key = Some("key".to_string());
    azure_config.ai.model = "deployment".to_string();
    azure_config.ai.base_url = "https://test.openai.azure.com".to_string();
    azure_config.ai.api_version = Some("2024-02-01".to_string());
    azure_config.ai.retry_attempts = 3;
    azure_config.ai.retry_delay_ms = 1000;

    let azure_client = AzureOpenAIClient::from_config(&azure_config.ai).unwrap();

    let request = AnalysisRequest {
        video_files: vec!["ID:test_video | Name:test.mkv | Path:/test.mkv".to_string()],
        subtitle_files: vec!["ID:test_sub | Name:test.srt | Path:/test.srt".to_string()],
        content_samples: vec![],
    };

    // All clients should produce identical prompts via trait
    let openai_prompt = openai_client.build_analysis_prompt(&request);
    let openrouter_prompt = openrouter_client.build_analysis_prompt(&request);
    let azure_prompt = azure_client.build_analysis_prompt(&request);

    assert_eq!(openai_prompt, openrouter_prompt);
    assert_eq!(openrouter_prompt, azure_prompt);

    // All clients should have identical system messages
    assert_eq!(
        OpenAIClient::get_analysis_system_message(),
        OpenRouterClient::get_analysis_system_message()
    );
    assert_eq!(
        OpenRouterClient::get_analysis_system_message(),
        AzureOpenAIClient::get_analysis_system_message()
    );
}
