//! Local / OpenAI-compatible LLM provider client.
//!
//! `LocalLLMClient` targets any OpenAI-compatible chat-completions endpoint
//! exposed by a local, LAN, VPN, or remote runtime (Ollama, LM Studio,
//! llama.cpp `llama-server`, vLLM, text-generation-webui, etc.). It mirrors
//! the structure of [`crate::services::ai::openrouter::OpenRouterClient`]
//! but treats the API key as optional and emits actionable, sanitized
//! error messages when the local endpoint is unreachable, returns a
//! non-OpenAI response, or signals that the requested model is not loaded.

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

/// Client for OpenAI-compatible local LLM runtimes.
///
/// The struct mirrors the field layout of the hosted-provider clients
/// (`OpenAIClient`, `OpenRouterClient`) but stores the API key as
/// `Option<String>` because most local runtimes accept unauthenticated
/// requests. The `request_timeout_seconds` value is retained so it can be
/// embedded in timeout error messages.
pub struct LocalLLMClient {
    client: Client,
    api_key: Option<String>,
    model: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    base_url: String,
    request_timeout_seconds: u64,
}

impl std::fmt::Debug for LocalLLMClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalLLMClient")
            .field("client", &self.client)
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("model", &self.model)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("retry_attempts", &self.retry_attempts)
            .field("retry_delay_ms", &self.retry_delay_ms)
            .field("base_url", &self.base_url)
            .field("request_timeout_seconds", &self.request_timeout_seconds)
            .finish()
    }
}

impl PromptBuilder for LocalLLMClient {}
impl ResponseParser for LocalLLMClient {}
impl HttpRetryClient for LocalLLMClient {
    fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }
    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

impl LocalLLMClient {
    /// Construct a new `LocalLLMClient` with explicit parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: Option<String>,
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

        // Normalize an empty/whitespace-only key to `None` so request paths
        // do not have to repeat the check.
        let api_key = api_key.and_then(|k| {
            let trimmed = k.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        Self {
            client,
            api_key,
            model,
            temperature,
            max_tokens,
            retry_attempts,
            retry_delay_ms,
            // Trim a single trailing slash so URL joining always yields one
            // separator. Both `http://h/v1/` and `http://h/v1` collapse to
            // the same stored form.
            base_url: base_url.trim_end_matches('/').to_string(),
            request_timeout_seconds,
        }
    }

    /// Create a `LocalLLMClient` from the unified `AIConfig`.
    ///
    /// Validates that `base_url` is non-empty, emits the shared insecure-HTTP
    /// warning (which already exempts loopback), and constructs an inner
    /// `reqwest::Client` honoring `request_timeout_seconds`.
    pub fn from_config(config: &crate::config::AIConfig) -> Result<Self> {
        if config.base_url.trim().is_empty() {
            return Err(SubXError::config(
                "ai.base_url is required for the local provider",
            ));
        }

        // The shared helper warns only when an api_key is configured against
        // a non-loopback HTTP host. Pass the configured key (or empty string
        // when absent) so the existing loopback exemption applies.
        let api_key_for_warning = config.api_key.clone().unwrap_or_default();
        crate::services::ai::security::warn_on_insecure_http_str(
            &config.base_url,
            &api_key_for_warning,
        );

        Ok(Self::new(
            config.api_key.clone(),
            config.model.clone(),
            config.temperature,
            config.max_tokens,
            config.retry_attempts,
            config.retry_delay_ms,
            config.base_url.clone(),
            config.request_timeout_seconds,
        ))
    }

    /// Build the `chat/completions` URL by joining the stored `base_url`
    /// with the path segment using exactly one `/` separator.
    fn chat_completions_url(&self) -> String {
        // `base_url` already had any single trailing slash stripped during
        // construction, so this format never produces `//chat/completions`.
        format!("{}/chat/completions", self.base_url)
    }

    /// Issue a chat-completions request, applying retry + actionable error
    /// mapping. Sends only OpenAI-canonical body fields.
    pub async fn chat_completion(&self, messages: Vec<Value>) -> Result<String> {
        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        let mut builder = self
            .client
            .post(self.chat_completions_url())
            .header("Content-Type", "application/json")
            .json(&request_body);
        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        // Note: `local_provider_hint()` is intentionally NOT appended here.
        // The advisory exists so hosted-provider clients can suggest the
        // local provider; from this client (which IS the local provider)
        // it would be tautological.

        let mut response = self.send_with_retry(builder).await?;

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
            return Err(self.map_http_error(response).await);
        }

        // Bounded chunked read in case the server omits Content-Length.
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| self.map_reqwest_error(e))?
        {
            body.extend_from_slice(&chunk);
            if body.len() as u64 > MAX_AI_RESPONSE_BYTES {
                return Err(SubXError::AiService(format!(
                    "AI response too large: {} bytes read (limit: {} bytes)",
                    body.len(),
                    MAX_AI_RESPONSE_BYTES
                )));
            }
        }

        let response_json: Value = serde_json::from_slice(&body).map_err(|e| {
            SubXError::AiService(format!(
                "local LLM response was not OpenAI-compatible JSON: {}",
                e
            ))
        })?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                SubXError::AiService(
                    "local LLM response was not OpenAI-compatible JSON: \
                     missing choices[0].message.content"
                        .to_string(),
                )
            })?;

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

    /// Send `request` with retry, surfacing actionable transport errors.
    async fn send_with_retry(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut attempts: u32 = 0;
        loop {
            let cloned = request.try_clone().ok_or_else(|| {
                SubXError::AiService("Request body cannot be cloned for retry".to_string())
            })?;
            match cloned.send().await {
                Ok(resp) => {
                    if resp.status().is_server_error() && attempts < self.retry_attempts {
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
                    return Ok(resp);
                }
                Err(e) if attempts < self.retry_attempts => {
                    attempts += 1;
                    log::warn!(
                        "Request attempt {} failed: {}. Retrying in {}ms...",
                        attempts,
                        e,
                        self.retry_delay_ms
                    );
                    time::sleep(Duration::from_millis(self.retry_delay_ms)).await;
                    continue;
                }
                Err(e) => return Err(self.map_reqwest_error(e)),
            }
        }
    }

    /// Map a low-level `reqwest::Error` into an actionable `SubXError`.
    fn map_reqwest_error(&self, err: reqwest::Error) -> SubXError {
        let url = sanitize_base_url(&self.base_url);
        if err.is_timeout() {
            return SubXError::AiService(format!(
                "local LLM endpoint timed out after {}s: {}",
                self.request_timeout_seconds, url
            ));
        }
        if err.is_connect() {
            return SubXError::AiService(format!("local LLM endpoint unreachable: {}", url));
        }
        // Fall back to the generic conversion (which already strips query
        // strings from any embedded URLs via `error_sanitizer`).
        err.into()
    }

    /// Map a non-2xx HTTP response into an actionable `SubXError`. Reads
    /// the body, sanitizes it, and detects the "model not found" pattern.
    async fn map_http_error(&self, response: reqwest::Response) -> SubXError {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        let safe_body = crate::services::ai::error_sanitizer::sanitize_url_in_error(
            &crate::services::ai::error_sanitizer::truncate_error_body(
                &body_text,
                crate::services::ai::error_sanitizer::DEFAULT_ERROR_BODY_MAX_LEN,
            ),
        );

        if status.as_u16() == 404 || body_indicates_model_missing(&body_text) {
            return SubXError::AiService(format!("local LLM model not found: {}", self.model));
        }

        SubXError::AiService(format!(
            "local LLM endpoint returned HTTP {}: {}",
            status, safe_body
        ))
    }
}

/// Detect common "model not loaded / no such model" body patterns emitted
/// by Ollama, LM Studio, llama.cpp, and vLLM.
fn body_indicates_model_missing(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    let mentions_model = lower.contains("model");
    if !mentions_model {
        return false;
    }
    lower.contains("not found")
        || lower.contains("not loaded")
        || lower.contains("no such model")
        || lower.contains("unknown model")
}

/// Strip userinfo, query strings, and fragments from a base URL so it can
/// be safely embedded in user-facing error messages.
///
/// Returns only `scheme://host[:port]/path`. Trailing slashes on the path
/// are preserved as authored. If the input cannot be parsed as a URL the
/// helper falls back to `"<unparseable URL>"` so credentials hidden in a
/// malformed string can never leak.
pub(crate) fn sanitize_base_url(input: &str) -> String {
    match url::Url::parse(input) {
        Ok(mut url) => {
            // Wipe credentials. The setters only fail when the URL cannot
            // carry userinfo; ignore any error and continue.
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);

            let scheme = url.scheme();
            // Render IPv6 hosts with brackets so the result remains a
            // syntactically valid URL (e.g. `http://[::1]:11434/v1`).
            let host_display = match url.host() {
                Some(url::Host::Ipv6(addr)) => format!("[{}]", addr),
                Some(_) => url.host_str().unwrap_or_default().to_string(),
                None => return "<unparseable URL>".to_string(),
            };
            let path = url.path();
            match url.port() {
                Some(port) => format!("{}://{}:{}{}", scheme, host_display, port, path),
                None => format!("{}://{}{}", scheme, host_display, path),
            }
        }
        Err(_) => "<unparseable URL>".to_string(),
    }
}

#[async_trait]
impl AIProvider for LocalLLMClient {
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

    async fn chat_completion(&self, messages: Vec<Value>) -> Result<String> {
        LocalLLMClient::chat_completion(self, messages).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_client(base_url: &str, api_key: Option<&str>) -> LocalLLMClient {
        LocalLLMClient::new(
            api_key.map(|s| s.to_string()),
            "llama3.1:8b-instruct".to_string(),
            0.3,
            1024,
            1,
            10,
            base_url.to_string(),
            120,
        )
    }

    #[test]
    fn debug_redacts_api_key() {
        let client = make_client("http://localhost:11434/v1", Some("super-secret-token"));
        let rendered = format!("{:?}", client);
        assert!(
            rendered.contains("[REDACTED]"),
            "Debug output should redact api_key, got: {rendered}"
        );
        assert!(!rendered.contains("super-secret-token"));
    }

    #[test]
    fn debug_marks_missing_api_key_as_none() {
        let client = make_client("http://localhost:11434/v1", None);
        let rendered = format!("{:?}", client);
        assert!(rendered.contains("api_key: None"), "got: {rendered}");
    }

    #[test]
    fn url_join_with_trailing_slash() {
        let client = make_client("http://localhost:11434/v1/", None);
        assert_eq!(
            client.chat_completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert!(!client.chat_completions_url().contains("//chat"));
    }

    #[test]
    fn url_join_without_trailing_slash() {
        let client = make_client("http://localhost:11434/v1", None);
        assert_eq!(
            client.chat_completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn url_join_root_base_url() {
        let client = make_client("http://localhost:11434", None);
        assert_eq!(
            client.chat_completions_url(),
            "http://localhost:11434/chat/completions"
        );
    }

    #[test]
    fn sanitize_base_url_strips_userinfo_query_and_fragment() {
        assert_eq!(
            sanitize_base_url("http://user:secret@127.0.0.1:11434/v1?token=abc#frag"),
            "http://127.0.0.1:11434/v1"
        );
    }

    #[test]
    fn sanitize_base_url_preserves_plain_localhost() {
        assert_eq!(
            sanitize_base_url("http://localhost:11434/v1"),
            "http://localhost:11434/v1"
        );
    }

    #[test]
    fn sanitize_base_url_preserves_trailing_slash() {
        // Trailing slashes on the path are preserved as authored.
        assert_eq!(
            sanitize_base_url("https://host:8080/api/v1/"),
            "https://host:8080/api/v1/"
        );
    }

    #[test]
    fn sanitize_base_url_handles_unparseable_input() {
        assert_eq!(sanitize_base_url("not a url"), "<unparseable URL>");
        assert_eq!(sanitize_base_url(""), "<unparseable URL>");
    }

    #[test]
    fn sanitize_base_url_strips_password_only() {
        assert_eq!(
            sanitize_base_url("https://:pwd@host:8080/v1"),
            "https://host:8080/v1"
        );
    }

    #[test]
    fn sanitize_base_url_preserves_ipv6_brackets() {
        assert_eq!(
            sanitize_base_url("http://[::1]:11434/v1"),
            "http://[::1]:11434/v1"
        );
        assert_eq!(
            sanitize_base_url("https://[fd00::1]:8443/v1/"),
            "https://[fd00::1]:8443/v1/"
        );
        // Userinfo on IPv6 host must still be stripped while brackets remain.
        assert_eq!(
            sanitize_base_url("http://user:pwd@[::1]:11434/v1?token=secret"),
            "http://[::1]:11434/v1"
        );
    }

    #[test]
    fn body_indicates_model_missing_detects_common_patterns() {
        assert!(body_indicates_model_missing(
            "{\"error\":\"model 'foo' not found, try pulling it first\"}"
        ));
        assert!(body_indicates_model_missing(
            "{\"error\":\"Model not loaded\"}"
        ));
        assert!(body_indicates_model_missing(
            "{\"detail\":\"no such model: bar\"}"
        ));
        assert!(body_indicates_model_missing(
            "{\"error\":\"unknown model llama99\"}"
        ));
        assert!(!body_indicates_model_missing(
            "{\"error\":\"server overloaded\"}"
        ));
        assert!(!body_indicates_model_missing(""));
    }

    fn make_config(base_url: &str, api_key: Option<&str>) -> crate::config::AIConfig {
        crate::config::AIConfig {
            provider: "local".to_string(),
            api_key: api_key.map(|s| s.to_string()),
            model: "llama3.1:8b-instruct".to_string(),
            base_url: base_url.to_string(),
            max_sample_length: 500,
            temperature: 0.3,
            max_tokens: 1024,
            retry_attempts: 2,
            retry_delay_ms: 100,
            request_timeout_seconds: 120,
            api_version: None,
        }
    }

    #[test]
    fn from_config_rejects_empty_base_url() {
        let config = make_config("", None);
        let err = LocalLLMClient::from_config(&config).unwrap_err();
        assert!(
            err.to_string().contains("ai.base_url is required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_config_rejects_whitespace_base_url() {
        let config = make_config("   ", None);
        assert!(LocalLLMClient::from_config(&config).is_err());
    }

    #[test]
    fn from_config_accepts_loopback_http() {
        let config = make_config("http://localhost:11434/v1", None);
        let client = LocalLLMClient::from_config(&config).expect("should accept loopback HTTP");
        assert!(client.api_key.is_none());
        assert_eq!(client.base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn from_config_accepts_lan_http() {
        let config = make_config("http://192.168.1.50:11434/v1", None);
        let client = LocalLLMClient::from_config(&config).expect("LAN HTTP must be accepted");
        assert_eq!(client.base_url, "http://192.168.1.50:11434/v1");
    }

    #[test]
    fn from_config_accepts_https() {
        let config = make_config("https://ollama.tailnet.ts.net/v1", Some("vllm-token"));
        let client = LocalLLMClient::from_config(&config).expect("HTTPS must be accepted");
        assert_eq!(client.base_url, "https://ollama.tailnet.ts.net/v1");
        assert_eq!(client.api_key.as_deref(), Some("vllm-token"));
    }

    #[test]
    fn from_config_normalizes_empty_api_key_to_none() {
        let config = make_config("http://localhost:11434/v1", Some(""));
        let client = LocalLLMClient::from_config(&config).unwrap();
        assert!(
            client.api_key.is_none(),
            "empty api_key should normalize to None"
        );
    }

    #[test]
    fn from_config_trims_trailing_slash_in_base_url() {
        let config = make_config("http://localhost:11434/v1/", None);
        let client = LocalLLMClient::from_config(&config).unwrap();
        assert_eq!(client.base_url, "http://localhost:11434/v1");
        assert_eq!(
            client.chat_completions_url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }
}
