use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::hosted_hint::{append_local_hint, maybe_attach_local_hint};
use crate::services::ai::prompts::{PromptBuilder, ResponseParser};
use crate::services::ai::retry::HttpRetryClient;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time;
use url::{ParseError, Url};

/// Azure OpenAI client implementation
pub struct AzureOpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    api_version: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    request_timeout_seconds: u64,
}

impl std::fmt::Debug for AzureOpenAIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AzureOpenAIClient")
            .field("client", &self.client)
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("api_version", &self.api_version)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("retry_attempts", &self.retry_attempts)
            .field("retry_delay_ms", &self.retry_delay_ms)
            .field("request_timeout_seconds", &self.request_timeout_seconds)
            .finish()
    }
}

const DEFAULT_AZURE_API_VERSION: &str = "2025-04-01-preview";

impl AzureOpenAIClient {
    /// Create a new AzureOpenAIClient with full configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_all(
        api_key: String,
        model: String,
        base_url: String,
        api_version: String,
        temperature: f32,
        max_tokens: u32,
        retry_attempts: u32,
        retry_delay_ms: u64,
        request_timeout_seconds: u64,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(request_timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");
        AzureOpenAIClient {
            client,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_version,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            request_timeout_seconds,
        }
    }

    /// Create client from AIConfig
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        let api_key = config
            .api_key
            .as_ref()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| SubXError::config("Missing Azure OpenAI API Key".to_string()))?
            .clone();
        // Use the model value as the deployment identifier; ensure it's provided
        let deployment_name = config.model.clone();
        if deployment_name.trim().is_empty() {
            return Err(SubXError::config(
                "Missing Azure OpenAI deployment name in model field".to_string(),
            ));
        }
        let api_version = config
            .api_version
            .clone()
            .unwrap_or_else(|| DEFAULT_AZURE_API_VERSION.to_string());

        // Validate base URL format, handle missing host specially
        let parsed = match Url::parse(&config.base_url) {
            Ok(u) => u,
            Err(ParseError::EmptyHost) => {
                return Err(SubXError::config(
                    "Azure OpenAI endpoint missing host".to_string(),
                ));
            }
            Err(e) => {
                return Err(SubXError::config(format!(
                    "Invalid Azure OpenAI endpoint: {}",
                    e
                )));
            }
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(SubXError::config(
                "Azure OpenAI endpoint must use http or https".to_string(),
            ));
        }
        crate::services::ai::security::warn_on_insecure_http(&parsed, &api_key);

        Ok(Self::new_with_all(
            api_key,
            config.model.clone(),
            config.base_url.clone(),
            api_version,
            config.temperature,
            config.max_tokens,
            config.retry_attempts,
            config.retry_delay_ms,
            config.request_timeout_seconds,
        ))
    }

    async fn make_request_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> crate::Result<reqwest::Response> {
        let mut attempts = 0;
        loop {
            let cloned = request.try_clone().ok_or_else(|| {
                crate::error::SubXError::AiService(
                    "Request body cannot be cloned for retry".to_string(),
                )
            })?;
            match cloned.send().await {
                Ok(resp) => {
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
                            "This appears to be a timeout error. Consider increasing 'ai.request_timeout_seconds' in config."
                        );
                    }
                    time::sleep(Duration::from_millis(self.retry_delay_ms)).await;
                }
                Err(e) => {
                    log::error!(
                        "Request failed after {} attempts. Final error: {}",
                        attempts + 1,
                        e
                    );
                    if e.is_timeout() {
                        log::error!(
                            "AI service error: Request timed out after multiple attempts. Try increasing 'ai.request_timeout_seconds' configuration."
                        );
                    } else if e.is_connect() {
                        log::error!(
                            "AI service error: Connection failed. Check network connection and Azure OpenAI endpoint settings."
                        );
                    }
                    return Err(e.into());
                }
            }
        }
    }

    /// Send a raw chat completion request to the Azure OpenAI Chat Completions API.
    pub async fn chat_completion(&self, messages: Vec<Value>) -> crate::Result<String> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.base_url, self.model, self.api_version
        );
        let mut req = self
            .client
            .post(url)
            .header("Content-Type", "application/json");
        if self.api_key.to_lowercase().starts_with("bearer ") {
            req = req.header("Authorization", self.api_key.clone());
        } else {
            req = req.header("api-key", self.api_key.clone());
        }
        let body = json!({
            "messages": messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
            "stream": false
        });
        let request = req.json(&body);
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
            let text = response.text().await?;
            let safe_body = crate::services::ai::error_sanitizer::sanitize_url_in_error(
                &crate::services::ai::error_sanitizer::truncate_error_body(
                    &text,
                    crate::services::ai::error_sanitizer::DEFAULT_ERROR_BODY_MAX_LEN,
                ),
            );
            return Err(SubXError::AiService(format!(
                "Azure OpenAI API error {}: {}",
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
        let resp_json: Value = serde_json::from_slice(&body)
            .map_err(|e| SubXError::AiService(format!("Failed to parse AI response: {}", e)))?;
        if let Some(usage) = resp_json.get("usage") {
            if let (Some(p), Some(c), Some(t)) = (
                usage.get("prompt_tokens").and_then(Value::as_u64),
                usage.get("completion_tokens").and_then(Value::as_u64),
                usage.get("total_tokens").and_then(Value::as_u64),
            ) {
                // Get model from response JSON, fallback to self.model if missing
                let model = resp_json
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(self.model.as_str())
                    .to_string();
                let stats = crate::services::ai::AiUsageStats {
                    model,
                    prompt_tokens: p as u32,
                    completion_tokens: c as u32,
                    total_tokens: t as u32,
                };
                display_ai_usage(&stats);
            }
        }
        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                SubXError::AiService(append_local_hint("Invalid API response format"))
            })?;
        Ok(content.to_string())
    }
}

impl PromptBuilder for AzureOpenAIClient {}
impl ResponseParser for AzureOpenAIClient {}
impl HttpRetryClient for AzureOpenAIClient {
    fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }

    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_azure_openai_from_config_and_url_construction() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.api_version = Some("2025-04-01-preview".to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            client.base_url, client.model, client.api_version
        );
        assert!(url.contains("deployment-name"));
    }

    #[test]
    fn test_missing_model_error() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();

        let err = AzureOpenAIClient::from_config(&config.ai)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Missing Azure OpenAI deployment name in model field"));
    }

    #[test]
    fn test_azure_openai_client_creation_with_defaults() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        // api_version defaults to DEFAULT_AZURE_API_VERSION

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        assert_eq!(
            client.api_version,
            super::DEFAULT_AZURE_API_VERSION.to_string()
        );
    }

    #[test]
    fn test_azure_openai_client_missing_api_key() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = None;
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();

        let result = AzureOpenAIClient::from_config(&config.ai);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing Azure OpenAI API Key"));
    }

    #[test]
    fn test_azure_openai_client_invalid_base_url() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "invalid-url".to_string();

        let result = AzureOpenAIClient::from_config(&config.ai);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid Azure OpenAI endpoint"));
    }

    #[test]
    fn test_azure_openai_client_invalid_url_scheme() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "ftp://example.openai.azure.com".to_string();

        let result = AzureOpenAIClient::from_config(&config.ai);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must use http or https"));
    }

    #[test]
    fn test_azure_openai_client_url_without_host() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://".to_string();

        let result = AzureOpenAIClient::from_config(&config.ai);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing host"));
    }

    #[test]
    fn test_azure_openai_with_custom_model_and_version() {
        let mock_model = "custom-model-123";
        let mock_version = "2023-12-01-preview";

        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = mock_model.to_string();
        config.ai.base_url = "https://custom.openai.azure.com".to_string();
        config.ai.api_version = Some(mock_version.to_string());

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        assert_eq!(client.model, mock_model);
        assert_eq!(client.api_version, mock_version);
    }

    #[test]
    fn test_azure_openai_with_trailing_slash_in_url() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com/".to_string(); // Trailing slash

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        assert_eq!(
            client.base_url,
            "https://example.openai.azure.com".to_string()
        );
    }

    #[test]
    fn test_azure_openai_with_custom_temperature_and_tokens() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.temperature = 0.8;
        config.ai.max_tokens = 2000;

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        assert!((client.temperature - 0.8).abs() < f32::EPSILON);
        assert_eq!(client.max_tokens, 2000);
    }

    #[test]
    fn test_azure_openai_with_custom_retry_and_timeout() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("test-api-key".to_string());
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();
        config.ai.retry_attempts = 5;
        config.ai.retry_delay_ms = 2000;
        config.ai.request_timeout_seconds = 180;

        let client = AzureOpenAIClient::from_config(&config.ai).unwrap();
        assert_eq!(client.retry_attempts, 5);
        assert_eq!(client.retry_delay_ms, 2000);
        assert_eq!(client.request_timeout_seconds, 180);
    }

    #[test]
    fn test_azure_openai_new_with_all_parameters() {
        let client = AzureOpenAIClient::new_with_all(
            "test-api-key".to_string(),
            "gpt-test".to_string(),
            "https://example.openai.azure.com".to_string(),
            "2025-04-01-preview".to_string(),
            0.7,
            4000,
            3,
            1000,
            120,
        );
        assert!(format!("{:?}", client).contains("AzureOpenAIClient"));
    }

    #[test]
    fn test_azure_openai_error_handling_empty_api_key() {
        let mut config = Config::default();
        config.ai.provider = "azure-openai".to_string();
        config.ai.api_key = Some("".to_string()); // Empty string
        config.ai.model = "deployment-name".to_string();
        config.ai.base_url = "https://example.openai.azure.com".to_string();

        let err = AzureOpenAIClient::from_config(&config.ai)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Missing Azure OpenAI API Key"));
    }

    /// §3.6 — connection refused against `127.0.0.1` MUST surface the hint.
    #[tokio::test]
    async fn test_hosted_hint_connection_refused_loopback() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let client = AzureOpenAIClient::new_with_all(
            "k".into(),
            "dep".into(),
            format!("http://127.0.0.1:{}", port),
            "2025-04-01-preview".into(),
            0.0,
            16,
            0,
            0,
            1,
        );
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ollama") && msg.contains("ai.provider"),
            "expected local-provider hint: {msg}"
        );
    }

    /// §3.6 — HTTP 200 with non-OpenAI body must surface the hint via the
    /// parse-shape branch.
    #[tokio::test]
    async fn test_hosted_hint_http_200_non_openai_body() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            // Azure URL shape:
            // {base}/openai/deployments/{model}/chat/completions
            .and(path("/openai/deployments/dep/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "hello": "world" })))
            .mount(&server)
            .await;
        let client = AzureOpenAIClient::new_with_all(
            "k".into(),
            "dep".into(),
            server.uri(),
            "2025-04-01-preview".into(),
            0.0,
            16,
            0,
            0,
            5,
        );
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid API response format")
                && msg.contains("ollama")
                && msg.contains("ai.provider"),
            "expected hint-bearing parse-shape error: {msg}"
        );
    }

    /// §3.6 negative — a public host MUST NOT surface the hint. We use
    /// TEST-NET-1 (RFC 5737) so the test is hermetic.
    #[tokio::test]
    async fn test_hosted_hint_not_emitted_for_public_host() {
        let client = AzureOpenAIClient::new_with_all(
            "k".into(),
            "dep".into(),
            "https://192.0.2.1".into(),
            "2025-04-01-preview".into(),
            0.0,
            16,
            0,
            0,
            1,
        );
        let err = client
            .chat_completion(vec![json!({"role":"user","content":"x"})])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.contains("ollama"),
            "public-host failure must NOT carry the hint: {msg}"
        );
    }
}

#[async_trait]
impl AIProvider for AzureOpenAIClient {
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult> {
        let prompt = self.build_analysis_prompt(&request);
        let messages = vec![
            json!({"role": "system", "content": "You are a professional subtitle matching assistant that can analyze the correspondence between video and subtitle files."}),
            json!({"role": "user", "content": prompt}),
        ];
        let resp = self.chat_completion(messages).await?;
        self.parse_match_result(&resp)
    }

    async fn verify_match(
        &self,
        verification: VerificationRequest,
    ) -> crate::Result<ConfidenceScore> {
        let prompt = self.build_verification_prompt(&verification);
        let messages = vec![
            json!({"role": "system", "content": "Please evaluate the confidence level of subtitle matching and provide a score between 0-1."}),
            json!({"role": "user", "content": prompt}),
        ];
        let resp = self.chat_completion(messages).await?;
        self.parse_confidence_score(&resp)
    }

    async fn chat_completion(&self, messages: Vec<Value>) -> crate::Result<String> {
        AzureOpenAIClient::chat_completion(self, messages).await
    }
}
