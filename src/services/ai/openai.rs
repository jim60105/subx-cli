use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::AiUsageStats;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use crate::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use serde_json::Value;
use std::time::Duration;

/// OpenAI 客戶端實作
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

// 模擬測試: OpenAIClient 與 AIProvider 介面
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::eq};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    mock! {
        AIClient {}

        #[async_trait]
        impl AIProvider for AIClient {
            async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult>;
            async fn verify_match(&self, verification: VerificationRequest) -> crate::Result<ConfidenceScore>;
        }
    }

    #[tokio::test]
    async fn test_openai_client_creation() {
        let client = OpenAIClient::new("test-key".into(), "gpt-4o-mini".into());
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "測試回應內容"}}]
            })))
            .mount(&server)
            .await;
        let mut client = OpenAIClient::new("test-key".into(), "gpt-4o-mini".into());
        client.base_url = server.uri();
        let messages = vec![json!({"role":"user","content":"測試"})];
        let resp = client.chat_completion(messages).await.unwrap();
        assert_eq!(resp, "測試回應內容");
    }

    #[tokio::test]
    async fn test_chat_completion_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": {"message":"Invalid API key"}
            })))
            .mount(&server)
            .await;
        let mut client = OpenAIClient::new("bad-key".into(), "gpt-4o-mini".into());
        client.base_url = server.uri();
        let messages = vec![json!({"role":"user","content":"測試"})];
        let result = client.chat_completion(messages).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_analyze_content_with_mock() {
        let mut mock = MockAIClient::new();
        let req = AnalysisRequest {
            video_files: vec!["v.mp4".into()],
            subtitle_files: vec!["s.srt".into()],
            content_samples: vec![],
        };
        let expected = MatchResult {
            matches: vec![],
            confidence: 0.5,
            reasoning: "OK".into(),
        };
        mock.expect_analyze_content()
            .with(eq(req.clone()))
            .times(1)
            .returning(move |_| Ok(expected.clone()));
        let res = mock.analyze_content(req.clone()).await.unwrap();
        assert_eq!(res.confidence, 0.5);
    }

    #[test]
    fn test_prompt_building_and_parsing() {
        let client = OpenAIClient::new("k".into(), "m".into());
        let request = AnalysisRequest {
            video_files: vec!["F1.mp4".into()],
            subtitle_files: vec!["S1.srt".into()],
            content_samples: vec![],
        };
        let prompt = client.build_analysis_prompt(&request);
        assert!(prompt.contains("F1.mp4"));
        assert!(prompt.contains("S1.srt"));
        assert!(prompt.contains("JSON"));
        let json_resp = r#"{ "matches": [], "confidence":0.9, "reasoning":"r" }"#;
        let mr = client.parse_match_result(json_resp).unwrap();
        assert_eq!(mr.confidence, 0.9);
    }
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

        let response_json: Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| SubXError::AiService("無效的 API 回應格式".to_string()))?;

        // 解析使用統計並顯示
        if let Some(usage_obj) = response_json.get("usage") {
            if let (Some(p), Some(c), Some(t)) = (
                usage_obj.get("prompt_tokens").and_then(Value::as_u64),
                usage_obj.get("completion_tokens").and_then(Value::as_u64),
                usage_obj.get("total_tokens").and_then(Value::as_u64),
            ) {
                let stats = AiUsageStats {
                    model: self.model.clone(),
                    prompt_tokens: p as u32,
                    completion_tokens: c as u32,
                    total_tokens: t as u32,
                };
                display_ai_usage(&stats);
            }
        }

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
