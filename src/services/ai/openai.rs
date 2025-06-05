use crate::error::SubXError;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// OpenAI 客戶端實作
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAIClient {
    /// 建立新的 OpenAIClient
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("建立 HTTP 客戶端失敗");
        Self {
            client,
            api_key,
            model,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.3,
            "max_tokens": 1000,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(SubXError::AiService(format!(
                "OpenAI API 錯誤 {}: {}",
                status, error_text
            )));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| SubXError::AiService("無效的 API 回應格式".to_string()))?;

        Ok(content.to_string())
    }
}

#[async_trait]
impl AIProvider for OpenAIClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let messages = vec![
            json!({"role": "system", "content": "你是一個專業的字幕匹配助手，能夠分析影片和字幕檔案的對應關係。"}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_match_result(&response)
    }

    async fn verify_match(&self, verification: VerificationRequest) -> Result<ConfidenceScore> {
        let prompt = self.build_verification_prompt(&verification);
        let messages = vec![
            json!({"role": "system", "content": "請評估字幕匹配的信心度，提供 0-1 之間的分數。"}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_confidence_score(&response)
    }
}
