use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::prompts::{
    build_analysis_prompt_base, build_verification_prompt_base, parse_confidence_score_base,
    parse_match_result_base,
};
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time;
use url::Url;

/// Azure OpenAI client implementation
#[derive(Debug)]
pub struct AzureOpenAIClient {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    deployment_id: String,
    api_version: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    request_timeout_seconds: u64,
}

const DEFAULT_AZURE_API_VERSION: &str = "2025-04-01-preview";

impl AzureOpenAIClient {
    /// Create a new AzureOpenAIClient with full configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_all(
        api_key: String,
        model: String,
        base_url: String,
        deployment_id: String,
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
            deployment_id,
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
        let deployment_id = config
            .deployment_id
            .clone()
            .ok_or_else(|| SubXError::config("Missing Azure OpenAI deployment ID".to_string()))?;
        let api_version = config
            .api_version
            .clone()
            .unwrap_or_else(|| DEFAULT_AZURE_API_VERSION.to_string());

        // Validate base URL format
        let parsed = Url::parse(&config.base_url)
            .map_err(|e| SubXError::config(format!("Invalid Azure OpenAI endpoint: {}", e)))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(SubXError::config(
                "Azure OpenAI endpoint must use http or https".to_string(),
            ));
        }
        if parsed.host().is_none() {
            return Err(SubXError::config(
                "Azure OpenAI endpoint missing host".to_string(),
            ));
        }

        Ok(Self::new_with_all(
            api_key,
            config.model.clone(),
            config.base_url.clone(),
            deployment_id,
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
    ) -> reqwest::Result<reqwest::Response> {
        let mut attempts = 0;
        loop {
            match request.try_clone().unwrap().send().await {
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
                    return Err(e);
                }
            }
        }
    }

    async fn chat_completion(&self, messages: Vec<Value>) -> crate::Result<String> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.base_url, self.deployment_id, self.api_version
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
        let response = self.make_request_with_retry(request).await?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(SubXError::AiService(format!(
                "Azure OpenAI API error {}: {}",
                status, text
            )));
        }
        let resp_json: Value = response.json().await?;
        if let Some(usage) = resp_json.get("usage") {
            if let (Some(p), Some(c), Some(t)) = (
                usage.get("prompt_tokens").and_then(Value::as_u64),
                usage.get("completion_tokens").and_then(Value::as_u64),
                usage.get("total_tokens").and_then(Value::as_u64),
            ) {
                let stats = crate::services::ai::AiUsageStats {
                    model: self.model.clone(),
                    prompt_tokens: p as u32,
                    completion_tokens: c as u32,
                    total_tokens: t as u32,
                };
                display_ai_usage(&stats);
            }
        }
        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| SubXError::AiService("Invalid API response format".to_string()))?;
        Ok(content.to_string())
    }

    fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        build_analysis_prompt_base(request)
    }

    fn build_verification_prompt(&self, request: &VerificationRequest) -> String {
        build_verification_prompt_base(request)
    }

    fn parse_match_result(&self, response: &str) -> crate::Result<MatchResult> {
        parse_match_result_base(response)
    }

    fn parse_confidence_score(&self, response: &str) -> crate::Result<ConfidenceScore> {
        parse_confidence_score_base(response)
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
}
