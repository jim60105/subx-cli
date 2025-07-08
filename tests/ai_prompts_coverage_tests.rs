use subx_cli::services::ai::openai::OpenAIClient;
/// Additional tests for AI prompts functionality to increase coverage
/// This test file focuses on edge cases and error handling for prompts module
use subx_cli::services::ai::prompts::{
    PromptBuilder, build_analysis_prompt_base, build_verification_prompt_base,
    parse_confidence_score_base, parse_match_result_base,
};
use subx_cli::services::ai::{AnalysisRequest, ContentSample, VerificationRequest};

mod common;

/// Test build_analysis_prompt_base function directly
#[tokio::test]
async fn test_build_analysis_prompt_base_function() {
    let request = AnalysisRequest {
        video_files: vec!["ID:file_abc123 | Name:test.mkv | Path:/path/test.mkv".to_string()],
        subtitle_files: vec!["ID:file_def456 | Name:test.srt | Path:/path/test.srt".to_string()],
        content_samples: vec![ContentSample {
            filename: "test.srt".to_string(),
            content_preview: "Sample subtitle content".to_string(),
            file_size: 512,
        }],
    };

    let prompt = build_analysis_prompt_base(&request);

    assert!(prompt.contains("ID:file_abc123"));
    assert!(prompt.contains("ID:file_def456"));
    assert!(prompt.contains("Sample subtitle content"));
    assert!(prompt.contains("Please analyze the matching relationship"));
    assert!(prompt.contains("Response format must be JSON"));
    assert!(prompt.contains("video_file_id"));
    assert!(prompt.contains("subtitle_file_id"));
}

/// Test build_verification_prompt_base function directly
#[tokio::test]
async fn test_build_verification_prompt_base_function() {
    let request = VerificationRequest {
        video_file: "episode_01.mkv".to_string(),
        subtitle_file: "episode_01.srt".to_string(),
        match_factors: vec![
            "filename_similarity".to_string(),
            "content_correlation".to_string(),
            "timing_analysis".to_string(),
        ],
    };

    let prompt = build_verification_prompt_base(&request);

    assert!(prompt.contains("episode_01.mkv"));
    assert!(prompt.contains("episode_01.srt"));
    assert!(prompt.contains("filename_similarity"));
    assert!(prompt.contains("content_correlation"));
    assert!(prompt.contains("timing_analysis"));
    assert!(prompt.contains("Please evaluate the confidence level"));
    assert!(prompt.contains("JSON format"));
}

/// Test parse_match_result_base function directly with valid JSON
#[tokio::test]
async fn test_parse_match_result_base_function_valid() {
    let response = r#"{
        "matches": [
            {
                "video_file_id": "file_123",
                "subtitle_file_id": "file_456",
                "confidence": 0.98,
                "match_factors": ["exact_match", "timing_correlation"]
            }
        ],
        "confidence": 0.95,
        "reasoning": "High confidence match based on multiple factors"
    }"#;

    let result = parse_match_result_base(response).unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].video_file_id, "file_123");
    assert_eq!(result.matches[0].subtitle_file_id, "file_456");
    assert_eq!(result.matches[0].confidence, 0.98);
    assert_eq!(result.confidence, 0.95);
    assert_eq!(
        result.reasoning,
        "High confidence match based on multiple factors"
    );
}

/// Test parse_match_result_base function with JSON extraction
#[tokio::test]
async fn test_parse_match_result_base_json_extraction() {
    // Test with extra text around JSON
    let response = r#"Here is the analysis result: {
        "matches": [
            {
                "video_file_id": "file_789",
                "subtitle_file_id": "file_abc",
                "confidence": 0.85,
                "match_factors": ["pattern_match"]
            }
        ],
        "confidence": 0.82,
        "reasoning": "Moderate confidence"
    } This concludes the analysis."#;

    let result = parse_match_result_base(response).unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].video_file_id, "file_789");
    assert_eq!(result.confidence, 0.82);
}

/// Test parse_match_result_base function with invalid JSON
#[tokio::test]
async fn test_parse_match_result_base_invalid_json() {
    let invalid_response = r#"{"invalid": "structure", "missing": "required_fields"}"#;

    let result = parse_match_result_base(invalid_response);
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("AI response parsing failed"));
}

/// Test parse_confidence_score_base function directly with valid JSON
#[tokio::test]
async fn test_parse_confidence_score_base_function_valid() {
    let response = r#"{
        "score": 0.91,
        "factors": ["filename_match", "content_similarity", "duration_alignment"]
    }"#;

    let result = parse_confidence_score_base(response).unwrap();
    assert_eq!(result.score, 0.91);
    assert_eq!(result.factors.len(), 3);
    assert!(result.factors.contains(&"filename_match".to_string()));
    assert!(result.factors.contains(&"content_similarity".to_string()));
    assert!(result.factors.contains(&"duration_alignment".to_string()));
}

/// Test parse_confidence_score_base with JSON extraction
#[tokio::test]
async fn test_parse_confidence_score_base_json_extraction() {
    let response = r#"Analysis complete. Result: {
        "score": 0.76,
        "factors": ["partial_match", "metadata_correlation"]
    } End of analysis."#;

    let result = parse_confidence_score_base(response).unwrap();
    assert_eq!(result.score, 0.76);
    assert_eq!(result.factors.len(), 2);
    assert!(result.factors.contains(&"partial_match".to_string()));
    assert!(result.factors.contains(&"metadata_correlation".to_string()));
}

/// Test parse_confidence_score_base with invalid JSON
#[tokio::test]
async fn test_parse_confidence_score_base_invalid_json() {
    let invalid_response = r#"{"wrong": "format", "no_score": true}"#;

    let result = parse_confidence_score_base(invalid_response);
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("AI confidence parsing failed"));
}

/// Test empty and edge case inputs
#[tokio::test]
async fn test_prompt_building_edge_cases() {
    // Test with empty content samples
    let request_empty = AnalysisRequest {
        video_files: vec!["ID:file1 | Name:video.mkv | Path:/video.mkv".to_string()],
        subtitle_files: vec!["ID:file2 | Name:subtitle.srt | Path:/subtitle.srt".to_string()],
        content_samples: vec![],
    };

    let prompt = build_analysis_prompt_base(&request_empty);
    assert!(prompt.contains("ID:file1"));
    assert!(prompt.contains("ID:file2"));
    assert!(!prompt.contains("Subtitle content preview"));

    // Test with multiple files
    let request_multiple = AnalysisRequest {
        video_files: vec![
            "ID:vid1 | Name:ep1.mkv | Path:/ep1.mkv".to_string(),
            "ID:vid2 | Name:ep2.mkv | Path:/ep2.mkv".to_string(),
        ],
        subtitle_files: vec![
            "ID:sub1 | Name:ep1.srt | Path:/ep1.srt".to_string(),
            "ID:sub2 | Name:ep2.srt | Path:/ep2.srt".to_string(),
        ],
        content_samples: vec![],
    };

    let prompt_multiple = build_analysis_prompt_base(&request_multiple);
    assert!(prompt_multiple.contains("ID:vid1"));
    assert!(prompt_multiple.contains("ID:vid2"));
    assert!(prompt_multiple.contains("ID:sub1"));
    assert!(prompt_multiple.contains("ID:sub2"));
}

/// Test verification prompt with various match factors
#[tokio::test]
async fn test_verification_prompt_various_factors() {
    // Test with single factor
    let request_single = VerificationRequest {
        video_file: "movie.mkv".to_string(),
        subtitle_file: "movie.srt".to_string(),
        match_factors: vec!["exact_filename".to_string()],
    };

    let prompt_single = build_verification_prompt_base(&request_single);
    assert!(prompt_single.contains("exact_filename"));
    assert!(prompt_single.contains("movie.mkv"));
    assert!(prompt_single.contains("movie.srt"));

    // Test with no factors (edge case)
    let request_empty = VerificationRequest {
        video_file: "test.mkv".to_string(),
        subtitle_file: "test.srt".to_string(),
        match_factors: vec![],
    };

    let prompt_empty = build_verification_prompt_base(&request_empty);
    assert!(prompt_empty.contains("test.mkv"));
    assert!(prompt_empty.contains("test.srt"));
    assert!(prompt_empty.contains("Matching factors:"));
}

/// Test trait default implementations consistency
#[tokio::test]
async fn test_trait_default_implementations() {
    let client = OpenAIClient::new("test".to_string(), "gpt-4".to_string(), 0.7, 1000, 3, 1000);

    // Test that trait methods delegate to base functions
    let analysis_request = AnalysisRequest {
        video_files: vec!["ID:test_vid | Name:test.mkv | Path:/test.mkv".to_string()],
        subtitle_files: vec!["ID:test_sub | Name:test.srt | Path:/test.srt".to_string()],
        content_samples: vec![],
    };

    let trait_prompt = client.build_analysis_prompt(&analysis_request);
    let base_prompt = build_analysis_prompt_base(&analysis_request);
    assert_eq!(trait_prompt, base_prompt);

    let verification_request = VerificationRequest {
        video_file: "test.mkv".to_string(),
        subtitle_file: "test.srt".to_string(),
        match_factors: vec!["test_factor".to_string()],
    };

    let trait_verification = client.build_verification_prompt(&verification_request);
    let base_verification = build_verification_prompt_base(&verification_request);
    assert_eq!(trait_verification, base_verification);
}

/// Test JSON parsing with malformed JSON strings
#[tokio::test]
async fn test_json_parsing_malformed() {
    // Test completely invalid JSON
    let completely_invalid = "This is not JSON at all";
    let result = parse_match_result_base(completely_invalid);
    assert!(result.is_err());

    // Test JSON with syntax errors
    let syntax_error = r#"{"matches": [{"video_file_id": "test"}"#; // Missing closing brackets
    let result = parse_match_result_base(syntax_error);
    assert!(result.is_err());

    // Test empty string
    let empty = "";
    let result = parse_match_result_base(empty);
    assert!(result.is_err());
}

/// Test system message constants
#[tokio::test]
async fn test_system_message_constants() {
    // Test analysis system message
    let analysis_msg = OpenAIClient::get_analysis_system_message();
    assert!(analysis_msg.contains("professional subtitle matching assistant"));
    assert!(analysis_msg.contains("analyze the correspondence"));
    assert!(analysis_msg.contains("video and subtitle files"));

    // Test verification system message
    let verification_msg = OpenAIClient::get_verification_system_message();
    assert!(verification_msg.contains("evaluate the confidence level"));
    assert!(verification_msg.contains("subtitle matching"));
    assert!(verification_msg.contains("score between 0-1"));
}
