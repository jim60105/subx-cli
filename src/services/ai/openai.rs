use crate::Result;
use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::AiUsageStats;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;
use std::time::Duration;

use crate::services::ai::hosted_hint::{append_local_hint, maybe_attach_local_hint};
use crate::services::ai::prompts::{PromptBuilder, ResponseParser};
use crate::services::ai::retry::HttpRetryClient;

/// OpenAI client implementation
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    base_url: String,
}

impl std::fmt::Debug for OpenAIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIClient")
            .field("client", &self.client)
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("retry_attempts", &self.retry_attempts)
            .field("retry_delay_ms", &self.retry_delay_ms)
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl PromptBuilder for OpenAIClient {}
impl ResponseParser for OpenAIClient {}
impl HttpRetryClient for OpenAIClient {
    fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }
    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

// Mock testing: OpenAIClient with AIProvider interface
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
        let client = OpenAIClient::new("test-key".into(), "gpt-4.1-mini".into(), 0.5, 1000, 2, 100);
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "gpt-4.1-mini");
        assert_eq!(client.temperature, 0.5);
        assert_eq!(client.max_tokens, 1000);
        assert_eq!(client.retry_attempts, 2);
        assert_eq!(client.retry_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_chat_completion_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{"message": {"content": "test response content"}}]
            })))
            .mount(&server)
            .await;
        let mut client =
            OpenAIClient::new("test-key".into(), "gpt-4.1-mini".into(), 0.3, 1000, 1, 0);
        client.base_url = server.uri();
        let messages = vec![json!({"role":"user","content":"test"})];
        let resp = client.chat_completion(messages).await.unwrap();
        assert_eq!(resp, "test response content");
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
        let mut client =
            OpenAIClient::new("bad-key".into(), "gpt-4.1-mini".into(), 0.3, 1000, 1, 0);
        client.base_url = server.uri();
        let messages = vec![json!({"role":"user","content":"test"})];
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
        let client = OpenAIClient::new("k".into(), "m".into(), 0.1, 1000, 0, 0);
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

    #[test]
    fn test_openai_client_from_config() {
        let config = crate::config::AIConfig {
            provider: "openai".to_string(),
            api_key: Some("test-key".to_string()),
            model: "gpt-test".to_string(),
            base_url: "https://custom.openai.com/v1".to_string(),
            max_sample_length: 500,
            temperature: 0.7,
            max_tokens: 2000,
            retry_attempts: 2,
            retry_delay_ms: 150,
            request_timeout_seconds: 60,
            api_version: None,
        };
        let client = OpenAIClient::from_config(&config).unwrap();
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "gpt-test");
        assert_eq!(client.temperature, 0.7);
        assert_eq!(client.max_tokens, 2000);
    }

    #[test]
    fn test_openai_client_from_config_invalid_base_url() {
        let config = crate::config::AIConfig {
            provider: "openai".to_string(),
            api_key: Some("test-key".to_string()),
            model: "gpt-test".to_string(),
            base_url: "ftp://invalid.url".to_string(),
            max_sample_length: 500,
            temperature: 0.7,
            max_tokens: 1000,
            retry_attempts: 2,
            retry_delay_ms: 150,
            request_timeout_seconds: 30,
            api_version: None,
        };
        let err = OpenAIClient::from_config(&config).unwrap_err();
        // Non-http/https protocols should return protocol error message
        assert!(
            err.to_string()
                .contains("Base URL must use http or https protocol")
        );
    }

    /// §3.6 — connection refused against `127.0.0.1` on a port with no
    /// listener results in a hint-bearing error.
    #[tokio::test]
    async fn test_hosted_hint_connection_refused_loopback() {
        let port = pick_unused_port().await;
        let mut client = OpenAIClient::new("k".into(), "gpt-4.1-mini".into(), 0.0, 16, 0, 0);
        client.base_url = format!("http://127.0.0.1:{}", port);
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ollama") && msg.contains("ai.provider"),
            "expected local-provider hint, got: {msg}"
        );
    }

    /// §3.6 — connection refused against an RFC1918 address surfaces the hint.
    #[tokio::test]
    async fn test_hosted_hint_connection_refused_rfc1918() {
        // 192.168.0.1 with a low port is unlikely to listen and produces a
        // transport failure (connect refused / unreachable / timeout)
        // within the configured request timeout. The hint must be appended
        // regardless of which transport sub-kind reqwest reports.
        let client = OpenAIClient::new_with_base_url_and_timeout(
            "k".into(),
            "gpt-4.1-mini".into(),
            0.0,
            16,
            0,
            0,
            "http://192.168.0.1:1".to_string(),
            1,
        );
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ollama") && msg.contains("ai.provider"),
            "expected local-provider hint, got: {msg}"
        );
    }

    /// §3.6 — HTTP 200 with a non-OpenAI body (`{"hello":"world"}`) MUST
    /// surface the local-provider hint via the parse-shape branch.
    #[tokio::test]
    async fn test_hosted_hint_http_200_non_openai_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hello": "world" })))
            .mount(&server)
            .await;
        let mut client = OpenAIClient::new("k".into(), "gpt-4.1-mini".into(), 0.0, 16, 0, 0);
        client.base_url = server.uri();
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid API response format"),
            "expected base parse-shape message: {msg}"
        );
        assert!(
            msg.contains("ollama") && msg.contains("ai.provider"),
            "expected local-provider hint: {msg}"
        );
    }

    /// §3.6 negative — a genuine failure against a public host MUST NOT
    /// surface the local-provider hint. We use TEST-NET-1 (`192.0.2.0/24`,
    /// RFC 5737), which is reserved for documentation and never routable,
    /// so the call fails deterministically without external dependencies.
    /// `192.0.2.x` is not in any private range, so the predicate must
    /// classify the host as public and suppress the hint.
    #[tokio::test]
    async fn test_hosted_hint_not_emitted_for_public_host() {
        let client = OpenAIClient::new_with_base_url_and_timeout(
            "k".into(),
            "gpt-4.1-mini".into(),
            0.0,
            16,
            0,
            0,
            "https://192.0.2.1/v1".to_string(),
            1,
        );
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.contains("ollama"),
            "public-host failure must NOT carry the local-provider hint: {msg}"
        );
    }

    /// Helper: bind a TCP listener on `127.0.0.1:0` to obtain a port the
    /// kernel allocated, then drop the listener so the port is free
    /// (race-prone in theory but reliable in tests because no sibling test
    /// rebinds it within microseconds).
    async fn pick_unused_port() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }
}

impl OpenAIClient {
    /// Create new OpenAIClient (using default base_url)
    pub fn new(
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        retry_attempts: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self::new_with_base_url(
            api_key,
            model,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            "https://api.openai.com/v1".to_string(),
        )
    }

    /// Create a new OpenAIClient with custom base_url support
    pub fn new_with_base_url(
        api_key: String,
        model: String,
        temperature: f32,
        max_tokens: u32,
        retry_attempts: u32,
        retry_delay_ms: u64,
        base_url: String,
    ) -> Self {
        // Use default 30 second timeout for backward compatibility
        Self::new_with_base_url_and_timeout(
            api_key,
            model,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            base_url,
            30,
        )
    }

    /// Create a new OpenAIClient with custom base_url and timeout support
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
            .ok_or_else(|| crate::error::SubXError::config("Missing OpenAI API Key"))?;

        // Validate base URL format
        Self::validate_base_url(&config.base_url)?;
        crate::services::ai::security::warn_on_insecure_http_str(&config.base_url, api_key);

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
        let parsed = Url::parse(url)
            .map_err(|e| crate::error::SubXError::config(format!("Invalid base URL: {}", e)))?;

        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(crate::error::SubXError::config(
                "Base URL must use http or https protocol".to_string(),
            ));
        }

        if parsed.host().is_none() {
            return Err(crate::error::SubXError::config(
                "Base URL must contain a valid hostname".to_string(),
            ));
        }

        Ok(())
    }

    /// Send a raw chat completion request to the OpenAI Chat Completions API.
    pub async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
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
            .json(&request_body);
        let mut response = match self.make_request_with_retry(request).await {
            Ok(r) => r,
            Err(e) => return Err(maybe_attach_local_hint(e, &self.base_url)),
        };

        const MAX_AI_RESPONSE_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB
        if let Some(len) = response.content_length() {
            if len > MAX_AI_RESPONSE_BYTES {
                return Err(SubXError::AiService(format!(
                    "AI response too large: {} bytes (limit: {} bytes)",
                    len, MAX_AI_RESPONSE_BYTES
                )));
            }
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            let safe_body = crate::services::ai::error_sanitizer::sanitize_url_in_error(
                &crate::services::ai::error_sanitizer::truncate_error_body(
                    &error_text,
                    crate::services::ai::error_sanitizer::DEFAULT_ERROR_BODY_MAX_LEN,
                ),
            );
            return Err(SubXError::AiService(format!(
                "OpenAI API error {}: {}",
                status, safe_body
            )));
        }

        // Bounded chunked read to guard against oversized responses when
        // content_length() is not reported by the server.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            body.extend_from_slice(&chunk);
            if body.len() as u64 > MAX_AI_RESPONSE_BYTES {
                return Err(SubXError::AiService(format!(
                    "AI response too large: {} bytes read (limit: {} bytes)",
                    body.len(),
                    MAX_AI_RESPONSE_BYTES
                )));
            }
        }
        let response_json: Value = serde_json::from_slice(&body)
            .map_err(|e| SubXError::AiService(format!("Failed to parse AI response: {}", e)))?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                // Body parsed as JSON but the canonical OpenAI shape is
                // missing — almost always a hosted client pointed at a
                // non-OpenAI endpoint. Append the local-provider hint
                // unconditionally here (no host predicate): the parse-shape
                // failure is itself a strong signal.
                SubXError::AiService(append_local_hint("Invalid API response format"))
            })?;

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
}

#[async_trait]
impl AIProvider for OpenAIClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let messages = vec![
            json!({"role": "system", "content": Self::get_analysis_system_message()}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_match_result(&response)
    }

    async fn verify_match(&self, verification: VerificationRequest) -> Result<ConfidenceScore> {
        let prompt = self.build_verification_prompt(&verification);
        let messages = vec![
            json!({"role": "system", "content": Self::get_verification_system_message()}),
            json!({"role": "user", "content": prompt}),
        ];
        let response = self.chat_completion(messages).await?;
        self.parse_confidence_score(&response)
    }

    async fn chat_completion(&self, messages: Vec<serde_json::Value>) -> Result<String> {
        OpenAIClient::chat_completion(self, messages).await
    }
}
