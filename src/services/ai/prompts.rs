use crate::Result;
use crate::error::SubXError;
use crate::services::ai::{AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest};
use serde_json;

impl super::OpenAIClient {
    /// 建立內容分析的 Prompt
    pub fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str("請分析以下影片和字幕檔案的匹配關係：\n\n");

        prompt.push_str("影片檔案：\n");
        for video in &request.video_files {
            prompt.push_str(&format!("- {}\n", video));
        }

        prompt.push_str("\n字幕檔案：\n");
        for subtitle in &request.subtitle_files {
            prompt.push_str(&format!("- {}\n", subtitle));
        }

        if !request.content_samples.is_empty() {
            prompt.push_str("\n字幕內容預覽：\n");
            for sample in &request.content_samples {
                prompt.push_str(&format!("檔案: {}\n", sample.filename));
                prompt.push_str(&format!("內容: {}\n\n", sample.content_preview));
            }
        }

        prompt.push_str(
            "請根據檔名模式等因素，提供匹配建議。\n\
            回應格式為 JSON：\n\
            {\n\
              \"matches\": [\n\
                {\n\
                  \"video_file\": \"影片檔名\",\n\
                  \"subtitle_file\": \"字幕檔名\",\n\
                  \"confidence\": 0.95,\n\
                  \"match_factors\": [\"檔名相似\"]\n\
                }\n\
              ],\n\
              \"confidence\": 0.9,\n\
              \"reasoning\": \"匹配原因說明\"\n\
            }",
        );

        prompt
    }

    /// 從 AI 回應中解析匹配結果
    pub fn parse_match_result(&self, response: &str) -> Result<MatchResult> {
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        serde_json::from_str(json_str)
            .map_err(|e| SubXError::AiService(format!("AI 回應解析失敗: {}", e)))
    }

    /// 建立匹配驗證的 Prompt
    pub fn build_verification_prompt(&self, request: &VerificationRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str("請根據以下匹配資訊評估信心度：\n");
        prompt.push_str(&format!("影片檔案: {}\n", request.video_file));
        prompt.push_str(&format!("字幕檔案: {}\n", request.subtitle_file));
        prompt.push_str("匹配因素：\n");
        for factor in &request.match_factors {
            prompt.push_str(&format!("- {}\n", factor));
        }
        prompt.push_str(
            "\n請以 JSON 格式回應，格式如下：\n{\n  \"score\": 0.9,\n  \"factors\": [\"...\"]\n}",
        );
        prompt
    }

    /// 從 AI 回應中解析信心度分數
    pub fn parse_confidence_score(&self, response: &str) -> Result<ConfidenceScore> {
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        serde_json::from_str(json_str)
            .map_err(|e| SubXError::AiService(format!("AI 信心度解析失敗: {}", e)))
    }
}
