use crate::Result;
use crate::error::SubXError;
use crate::services::ai::{AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest};
use serde_json;

impl super::OpenAIClient {
    /// Build content analysis prompt
    pub fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str("Please analyze the matching relationship between the following video and subtitle files:\n\n");

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
            "Please provide matching suggestions based on filename patterns and other factors.\n\
            Response format should be JSON:\n\
            {\n\
              \"matches\": [\n\
                {\n\
                  \"video_file\": \"video_filename\",\n\
                  \"subtitle_file\": \"subtitle_filename\",\n\
                  \"confidence\": 0.95,\n\
                  \"match_factors\": [\"filename_similarity\"]\n\
                }\n\
              ],\n\
              \"confidence\": 0.9,\n\
              \"reasoning\": \"explanation_for_matching\"\n\
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
