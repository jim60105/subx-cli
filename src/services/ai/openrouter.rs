use crate::Result;
use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::AiUsageStats;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time;

use crate::services::ai::prompts::{PromptBuilder, ResponseParser};
use crate::services::ai::retry::HttpRetryClient;

/// OpenRouter client implementation
#[derive(Debug)]
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    base_url: String,
}

impl PromptBuilder for OpenRouterClient {}
impl ResponseParser for OpenRouterClient {}
impl HttpRetryClient for OpenRouterClient {
    fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }
    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

impl OpenRouterClient {
    /// Create new OpenRouterClient with default configuration
    pub fn new(
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        retry_attempts: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self::new_with_base_url_and_timeout(
            api_key,
            model,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            "https://openrouter.ai/api/v1".to_string(),
            120,
        )
    }

    /// Create new OpenRouterClient with custom base URL and timeout
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_base_url_and_timeout(
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        retry_attempts: u32,
        retry_delay_ms: u64,
        base_url: String,
        request_timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_key,
            model,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Create client from unified configuration
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .ok_or_else(|| SubXError::config("Missing OpenRouter API Key"))?;

        // Validate base URL format
        Self::validate_base_url(&config.base_url)?;

        Ok(Self::new_with_base_url_and_timeout(
            api_key.clone(),
            config.model.clone(),
            config.temperature,
            config.max_tokens,
            config.retry_attempts,
            config.retry_delay_ms,
            config.base_url.clone(),
            config.request_timeout_seconds,
        ))
    }

    /// Validate base URL format
    fn validate_base_url(url: &str) -> crate::Result<()> {
        use url::Url;
        let parsed =
            Url::parse(url).map_err(|e| SubXError::config(format!("Invalid base URL: {}", e)))?;

        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(SubXError::config(
                "Base URL must use http or https protocol".to_string(),
            ));
        }

        if parsed.host().is_none() {
            return Err(SubXError::config(
                "Base URL must contain a valid hostname".to_string(),
            ));
        }

        Ok(())
    }

    async fn chat_completion(&self, messages: Vec<Value>) -> Result<String> {
        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        let request = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/jim60105/subx-cli")
            .header("X-Title", "Subx")
            .json(&request_body);

        let response = self.make_request_with_retry(request).await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Err(SubXError::AiService(format!(
                "OpenRouter API error {}: {}",
                status, error_text
            )));
        }

        let response_json: Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| SubXError::AiService("Invalid API response format".to_string()))?;

        // Parse usage statistics and display
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

    async fn make_request_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> reqwest::Result<reqwest::Response> {
        let mut attempts = 0;
        loop {
            match request.try_clone().unwrap().send().await {
                Ok(resp) => {
                    // Retry on server error statuses (5xx) if attempts remain
                    if resp.status().is_server_error() && (attempts as u32) < self.retry_attempts {
                        attempts += 1;
                        log::warn!(
                            "Request attempt {} failed with status {}. Retrying in {}ms...",
                            attempts,
                            resp.status(),
                            self.retry_delay_ms
                        );
                        time::sleep(Duration::from_millis(self.retry_delay_ms)).await;
                        continue;
                    }
                    if attempts > 0 {
                        log::info!("Request succeeded after {} retry attempts", attempts);
                    }
                    return Ok(resp);
                }
                Err(e) if (attempts as u32) < self.retry_attempts => {
                    attempts += 1;
                    log::warn!(
                        "Request attempt {} failed: {}. Retrying in {}ms...",
                        attempts,
                        e,
                        self.retry_delay_ms
                    );

                    if e.is_timeout() {
                        log::warn!(
                            "This appears to be a timeout error. If this persists, consider increasing 'ai.request_timeout_seconds' in your configuration."
                        );
                    }

                    time::sleep(Duration::from_millis(self.retry_delay_ms)).await;
                    continue;
                }
                Err(e) => {
                    log::error!(
                        "Request failed after {} attempts. Final error: {}",
                        attempts + 1,
                        e
                    );

                    if e.is_timeout() {
                        log::error!(
                            "AI service error: Request timed out after multiple attempts. \
                        This usually indicates network connectivity issues or server overload. \
                        Try increasing 'ai.request_timeout_seconds' configuration. \
                        Hint: check network connection and API service status"
                        );
                    } else if e.is_connect() {
                        log::error!(
                            "AI service error: Connection failed. \
                        Hint: check network connection and API base URL settings"
                        );
                    }

                    return Err(e);
                }
            }
        }
    }
}

#[async_trait]
impl AIProvider for OpenRouterClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let messages = vec![
            json!({"role": "system", "content": "You are a professional subtitle matching assistant that can analyze the correspondence between video and subtitle files."}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_match_result(&response)
    }

    async fn verify_match(&self, verification: VerificationRequest) -> Result<ConfidenceScore> {
        let prompt = self.build_verification_prompt(&verification);
        let messages = vec![
            json!({"role": "system", "content": "Please evaluate the confidence level of subtitle matching and provide a score between 0-1."}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_confidence_score(&response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
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
    async fn test_openrouter_client_creation() {
        let client = OpenRouterClient::new(
            "test-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.5,
            1000,
            2,
            100,
        );
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 1000);
        assert_eq!(client.retry_attempts, 2);
        assert_eq!(client.retry_delay_ms, 100);
        assert_eq!(client.base_url, "https://openrouter.ai/api/v1");
    }

    #[tokio::test]
    async fn test_openrouter_client_creation_with_custom_base_url() {
        let client = OpenRouterClient::new_with_base_url_and_timeout(
            "test-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.3,
            2000,
            3,
            200,
            "https://custom-openrouter.ai/api/v1".into(),
            60,
        );
        assert_eq!(client.base_url, "https://custom-openrouter.ai/api/v1");
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .and(header(
                "HTTP-Referer",
                "https://github.com/jim60105/subx-cli",
            ))
            .and(header("X-Title", "Subx"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "test response content"}}],
                "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
            })))
            .mount(&server)
            .await;

        let mut client = OpenRouterClient::new(
            "test-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.3,
            1000,
            1,
            0,
        );
        client.base_url = server.uri();

        let messages = vec![json!({"role":"user","content":"test"})];
        let resp = client.chat_completion(messages).await.unwrap();
        assert_eq!(resp, "test response content");
    }

    #[tokio::test]
    async fn test_chat_completion_error_handling() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": {"message":"Invalid API key"}
            })))
            .mount(&server)
            .await;

        let mut client = OpenRouterClient::new(
            "bad-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.3,
            1000,
            1,
            0,
        );
        client.base_url = server.uri();

        let messages = vec![json!({"role":"user","content":"test"})];
        let result = client.chat_completion(messages).await;
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("OpenRouter API error 401")
        );
    }

    #[tokio::test]
    async fn test_retry_mechanism() {
        let server = MockServer::start().await;

        // First request fails, second succeeds
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "success after retry"}}]
            })))
            .mount(&server)
            .await;

        let mut client = OpenRouterClient::new(
            "test-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.3,
            1000,
            2,  // Allow 2 retries
            50, // Short delay for testing
        );
        client.base_url = server.uri();

        let messages = vec![json!({"role":"user","content":"test"})];
        let result = client.chat_completion(messages).await.unwrap();
        assert_eq!(result, "success after retry");
    }

    #[test]
    fn test_openrouter_client_from_config() {
        let config = crate::config::AIConfig {
            provider: "openrouter".to_string(),
            api_key: Some("test-key".to_string()),
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            max_sample_length: 500,
            temperature: 0.7,
            max_tokens: 2000,
            retry_attempts: 3,
            retry_delay_ms: 150,
            request_timeout_seconds: 120,
            api_version: None,
        };

        let client = OpenRouterClient::from_config(&config).unwrap();
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "deepseek/deepseek-r1-0528:free");
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 2000);
        assert_eq!(client.retry_attempts, 3);
        assert_eq!(client.retry_delay_ms, 150);
    }

    #[test]
    fn test_openrouter_client_from_config_missing_api_key() {
        let config = crate::config::AIConfig {
            provider: "openrouter".to_string(),
            api_key: None,
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            max_sample_length: 500,
            temperature: 0.3,
            max_tokens: 1000,
            retry_attempts: 2,
            retry_delay_ms: 100,
            request_timeout_seconds: 30,
            api_version: None,
        };

        let result = OpenRouterClient::from_config(&config);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("Missing OpenRouter API Key")
        );
    }

    #[test]
    fn test_openrouter_client_from_config_invalid_base_url() {
        let config = crate::config::AIConfig {
            provider: "openrouter".to_string(),
            api_key: Some("test-key".to_string()),
            model: "deepseek/deepseek-r1-0528:free".to_string(),
            base_url: "ftp://invalid.url".to_string(),
            max_sample_length: 500,
            temperature: 0.3,
            max_tokens: 1000,
            retry_attempts: 2,
            retry_delay_ms: 100,
            request_timeout_seconds: 30,
            api_version: None,
        };

        let result = OpenRouterClient::from_config(&config);
        assert!(result.is_err());
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("must use http or https protocol")
        );
    }

    #[test]
    fn test_prompt_building_and_parsing() {
        let client = OpenRouterClient::new(
            "test-key".into(),
            "deepseek/deepseek-r1-0528:free".into(),
            0.1,
            1000,
            0,
            0,
        );
        let request = AnalysisRequest {
            video_files: vec!["video1.mp4".into()],
            subtitle_files: vec!["subtitle1.srt".into()],
            content_samples: vec![],
        };

        let prompt = client.build_analysis_prompt(&request);
        assert!(prompt.contains("video1.mp4"));
        assert!(prompt.contains("subtitle1.srt"));
        assert!(prompt.contains("JSON"));

        let json_response = r#"{ "matches": [], "confidence":0.9, "reasoning":"test reason" }"#;
        let match_result = client.parse_match_result(json_response).unwrap();
        assert_eq!(match_result.confidence, 0.9);
        assert_eq!(match_result.reasoning, "test reason");
    }
}
