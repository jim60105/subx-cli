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

// Prompt building and response parsing methods (copied from OpenAIClient)
impl OpenRouterClient {
    /// Build content analysis prompt
    pub fn build_analysis_prompt(&self, request: &AnalysisRequest) -> String {
        let mut prompt = String::new();
        prompt.push_str("Please analyze the matching relationship between the following video and subtitle files. Each file has a unique ID that you must use in your response.\n\n");

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
            "Please provide matching suggestions based on filename patterns, content similarity, and other factors.\n\
            Response format must be JSON using the file IDs:\n\
            {\n\
              \"matches\": [\n\
                {\n\
                  \"video_file_id\": \"file_abc123456789abcd\",\n\
                  \"subtitle_file_id\": \"file_def456789abcdef0\",\n\
                  \"confidence\": 0.95,\n\
                  \"match_factors\": [\"filename_similarity\", \"content_correlation\"]\n\
                }\n\
              ],\n\
              \"confidence\": 0.9,\n\
              \"reasoning\": \"Explanation for the matching decisions\"\n\
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
