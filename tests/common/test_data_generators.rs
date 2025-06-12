use serde_json::json;

/// Generator for mock match responses used in integration tests.
#[allow(dead_code)]
pub struct MatchResponseGenerator;

#[allow(dead_code)]
impl MatchResponseGenerator {
    /// Generate a single successful match response.
    pub fn successful_single_match() -> String {
        json!({
            "matches": [
                {
                    "video_file_id": "file_video123",
                    "subtitle_file_id": "file_subtitle123",
                    "confidence": 0.95,
                    "match_factors": ["filename_similarity", "content_correlation"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "High confidence match based on filename similarity and content analysis."
        })
        .to_string()
    }

    /// Generate a successful match response with specific file IDs.
    pub fn successful_match_with_ids(video_id: &str, subtitle_id: &str) -> String {
        json!({
            "matches": [
                {
                    "video_file_id": video_id,
                    "subtitle_file_id": subtitle_id,
                    "confidence": 0.95,
                    "match_factors": ["filename_similarity", "content_correlation"]
                }
            ],
            "confidence": 0.95,
            "reasoning": "High confidence match based on filename similarity and content analysis."
        })
        .to_string()
    }

    /// Generate a response indicating no matches found.
    pub fn no_matches_found() -> String {
        json!({
            "matches": [],
            "confidence": 0.1,
            "reasoning": "No suitable matches found between video and subtitle files."
        })
        .to_string()
    }

    /// Generate a response with multiple high-confidence matches.
    pub fn multiple_matches() -> String {
        json!({
            "matches": [
                {
                    "video_file_id": "file_video1",
                    "subtitle_file_id": "file_subtitle1",
                    "confidence": 0.92,
                    "match_factors": ["filename_similarity"]
                },
                {
                    "video_file_id": "file_video2",
                    "subtitle_file_id": "file_subtitle2",
                    "confidence": 0.87,
                    "match_factors": ["content_correlation", "language_match"]
                }
            ],
            "confidence": 0.89,
            "reasoning": "Multiple high-confidence matches identified."
        })
        .to_string()
    }
}
