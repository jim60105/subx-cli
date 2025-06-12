use crate::Result;
use crate::error::SubXError;
use crate::services::ai::{AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest};
use serde_json;

impl super::OpenAIClient {
    /// Build content analysis prompt
    pub fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str("Please analyze the matching relationship between the following video and subtitle files. Each file has a unique ID that you must use in your response.\n\n");

        prompt.push_str("Video files:\n");
        for video in &request.video_files {
            prompt.push_str(&format!("- {}\n", video));
        }

        prompt.push_str("\nSubtitle files:\n");
        for subtitle in &request.subtitle_files {
            prompt.push_str(&format!("- {}\n", subtitle));
        }

        if !request.content_samples.is_empty() {
            prompt.push_str("\nSubtitle content preview:\n");
            for sample in &request.content_samples {
                prompt.push_str(&format!("File: {}\n", sample.filename));
                prompt.push_str(&format!("Content: {}\n\n", sample.content_preview));
            }
        }

        prompt.push_str(
            "Please provide matching suggestions based on filename patterns, content similarity, and other factors.\n\
            Response format must be JSON using the file IDs:\n\
            {\n\
              \"matches\": [\n\
                {\n\
                  \"video_file_id\": \"file_abc123456789abcd\",\n\
                  \"subtitle_file_id\": \"file_def456789abcdef0\",\n\
                  \"confidence\": 0.95,\n\
                  \"match_factors\": [\"filename_similarity\", \"content_correlation\"]\n\
                }\n\
              ],\n\
              \"confidence\": 0.9,\n\
              \"reasoning\": \"Explanation for the matching decisions\"\n\
            }",
        );

        prompt
    }

    /// Parse matching results from AI response
    pub fn parse_match_result(&self, response: &str) -> Result<MatchResult> {
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        serde_json::from_str(json_str)
            .map_err(|e| SubXError::AiService(format!("AI response parsing failed: {}", e)))
    }

    /// Build verification prompt
    pub fn build_verification_prompt(&self, request: &VerificationRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str(
            "Please evaluate the confidence level based on the following matching information:\n",
        );
        prompt.push_str(&format!("Video file: {}\n", request.video_file));
        prompt.push_str(&format!("Subtitle file: {}\n", request.subtitle_file));
        prompt.push_str("Matching factors:\n");
        for factor in &request.match_factors {
            prompt.push_str(&format!("- {}\n", factor));
        }
        prompt.push_str(
            "\nPlease respond in JSON format as follows:\n{\n  \"score\": 0.9,\n  \"factors\": [\"...\"]\n}",
        );
        prompt
    }

    /// Parse confidence score from AI response
    pub fn parse_confidence_score(&self, response: &str) -> Result<ConfidenceScore> {
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        serde_json::from_str(json_str)
            .map_err(|e| SubXError::AiService(format!("AI confidence parsing failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {

    use crate::services::ai::{AnalysisRequest, OpenAIClient};

    #[test]
    fn test_ai_prompt_with_file_ids_english() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 0, 0);
        let request = AnalysisRequest {
            video_files: vec!["ID:file_abc123456789abcd | Name:movie.mkv | Path:movie.mkv".into()],
            subtitle_files: vec![
                "ID:file_def456789abcdef0 | Name:movie.srt | Path:movie.srt".into(),
            ],
            content_samples: vec![],
        };

        let prompt = client.build_analysis_prompt(&request);

        assert!(prompt.contains("ID:file_abc123456789abcd"));
        assert!(prompt.contains("video_file_id"));
        assert!(prompt.contains("subtitle_file_id"));
        assert!(prompt.contains("Please analyze the matching"));
        assert!(prompt.contains("unique ID"));
        assert!(prompt.contains("Response format must be JSON"));
        assert!(!prompt.contains("請分析"));
        assert!(!prompt.contains("影片檔案"));
        assert!(!prompt.contains("字幕檔案"));
    }

    #[test]
    fn test_parse_match_result_with_ids() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 0, 0);
        let json_resp = r#"{
            "matches": [{
                "video_file_id": "file_abc123456789abcd",
                "subtitle_file_id": "file_def456789abcdef0",
                "confidence": 0.95,
                "match_factors": ["filename_similarity"]
            }],
            "confidence": 0.9,
            "reasoning": "Strong match based on filename patterns"
        }"#;

        let result = client.parse_match_result(json_resp).unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].video_file_id, "file_abc123456789abcd");
        assert_eq!(result.matches[0].subtitle_file_id, "file_def456789abcdef0");
        assert_eq!(result.matches[0].confidence, 0.95);
        assert_eq!(result.matches[0].match_factors[0], "filename_similarity");
    }

    #[test]
    fn test_ai_prompt_structure_consistency() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 0, 0);
        let request = AnalysisRequest {
            video_files: vec![
                "ID:file_video1 | Name:video1.mkv | Path:season1/video1.mkv".into(),
                "ID:file_video2 | Name:video2.mkv | Path:season1/video2.mkv".into(),
            ],
            subtitle_files: vec![
                "ID:file_sub1 | Name:sub1.srt | Path:season1/sub1.srt".into(),
                "ID:file_sub2 | Name:sub2.srt | Path:season1/sub2.srt".into(),
            ],
            content_samples: vec![],
        };

        let prompt = client.build_analysis_prompt(&request);

        assert!(prompt.contains("ID:file_video1"));
        assert!(prompt.contains("ID:file_video2"));
        assert!(prompt.contains("ID:file_sub1"));
        assert!(prompt.contains("ID:file_sub2"));
        assert!(prompt.contains("Video files:"));
        assert!(prompt.contains("Subtitle files:"));
        assert!(prompt.contains("Response format must be JSON"));
    }

    #[test]
    fn test_parse_confidence_score() {
        let client = OpenAIClient::new("test_key".into(), "gpt-4.1".into(), 0.1, 0, 0);
        let json_resp = r#"{
            "score": 0.88,
            "factors": ["filename_similarity", "content_correlation"]
        }"#;

        let result = client.parse_confidence_score(json_resp).unwrap();
        assert_eq!(result.score, 0.88);
        assert_eq!(
            result.factors,
            vec![
                "filename_similarity".to_string(),
                "content_correlation".to_string()
            ]
        );
    }
}
