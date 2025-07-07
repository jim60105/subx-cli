use serde_json::json;
use std::time::Duration;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper for setting up a Wiremock mock Azure OpenAI server in integration tests.
#[allow(dead_code)]
pub struct MockAzureOpenAITestHelper {
    mock_server: MockServer,
    deployment_id: String,
    api_version: String,
}

#[allow(dead_code)]
impl MockAzureOpenAITestHelper {
    /// Start a new mock Azure OpenAI server instance.
    pub async fn new() -> Self {
        let mock_server = MockServer::start().await;
        Self {
            mock_server,
            deployment_id: "test-deployment".to_string(),
            api_version: "2025-04-01-preview".to_string(),
        }
    }

    /// Start a new mock Azure OpenAI server with custom deployment ID and API version.
    pub async fn new_with_deployment(deployment_id: &str, api_version: &str) -> Self {
        let mock_server = MockServer::start().await;
        Self {
            mock_server,
            deployment_id: deployment_id.to_string(),
            api_version: api_version.to_string(),
        }
    }

    /// Return the base URL of the mock server.
    pub fn base_url(&self) -> String {
        self.mock_server.uri()
    }

    /// Get the deployment ID used by this helper.
    pub fn deployment_id(&self) -> &str {
        &self.deployment_id
    }

    /// Get the API version used by this helper.
    pub fn api_version(&self) -> &str {
        &self.api_version
    }

    /// Mock a successful chat completion response for Azure OpenAI endpoint.
    pub async fn mock_chat_completion_success(&self, response_content: &str) {
        let response_body = json!({
            "choices": [
                {
                    "message": { "content": response_content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Mock a successful chat completion with api-key authentication.
    pub async fn mock_chat_completion_success_with_api_key(
        &self,
        response_content: &str,
        expected_api_key: &str,
    ) {
        let response_body = json!({
            "choices": [
                {
                    "message": { "content": response_content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("api-key", expected_api_key))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Mock a successful chat completion with Bearer token authentication.
    pub async fn mock_chat_completion_success_with_bearer_token(
        &self,
        response_content: &str,
        expected_token: &str,
    ) {
        let response_body = json!({
            "choices": [
                {
                    "message": { "content": response_content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("authorization", expected_token))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Mock a successful chat completion with dynamic file IDs for cache testing.
    pub async fn mock_chat_completion_with_dynamic_ids(&self, video_id: &str, subtitle_id: &str) {
        use crate::common::test_data_generators::MatchResponseGenerator;

        let response_content =
            MatchResponseGenerator::successful_match_with_ids(video_id, subtitle_id);
        let response_body = json!({
            "choices": [
                { "message": { "content": response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Mock a chat completion response with exact expected number of calls.
    pub async fn mock_chat_completion_with_expectation(
        &self,
        response_content: &str,
        expected_calls: usize,
    ) {
        let response_body = json!({
            "choices": [
                { "message": { "content": response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .expect(expected_calls as u64)
            .mount(&self.mock_server)
            .await;
    }

    /// Verify that all expectations registered on the mock server have been met.
    pub async fn verify_expectations(&self) {
        // Retrieve received requests to trigger expectation verification on server drop.
        let _ = self.mock_server.received_requests().await;
    }

    /// Setup an error response with given status code and error message.
    pub async fn setup_error_response(&self, status: u16, error_message: &str) {
        let response_body = json!({
            "error": { "message": error_message }
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(status).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Setup a delayed chat completion response to simulate network latency.
    pub async fn setup_delayed_response(&self, delay_ms: u64, response_content: &str) {
        let response_body = json!({
            "choices": [
                { "message": { "content": response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(delay_ms))
                    .set_body_json(response_body),
            )
            .mount(&self.mock_server)
            .await;
    }

    /// Setup multiple retryable error responses followed by success.
    pub async fn setup_retry_scenario(
        &self,
        error_count: usize,
        final_response_content: &str,
    ) {
        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        // Set up error responses that will be consumed first
        for _ in 0..error_count {
            Mock::given(method("POST"))
                .and(path(&path_pattern))
                .and(query_param("api-version", &self.api_version))
                .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                    "error": { "message": "Internal server error" }
                })))
                .expect(1)
                .mount(&self.mock_server)
                .await;
        }

        // Set up the final success response
        let response_body = json!({
            "choices": [
                { "message": { "content": final_response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-test"
        });

        Mock::given(method("POST"))
            .and(path(&path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .expect(1)
            .mount(&self.mock_server)
            .await;
    }

    /// Mock timeout error followed by success for retry testing
    pub async fn mock_chat_completion_timeout_then_success(&self) {
        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        // First call times out (simulate with very long delay)
        Mock::given(method("POST"))
            .and(path(&path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
            .expect(1)
            .mount(&self.mock_server)
            .await;
        
        // Second call succeeds
        let response_body = json!({
            "choices": [{
                "message": {
                    "content": "1"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "model": "gpt-test"
        });

        Mock::given(method("POST"))
            .and(path(&path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .expect(1)
            .mount(&self.mock_server)
            .await;
    }

    /// Mock connection error followed by success for retry testing  
    pub async fn mock_chat_completion_connection_error_then_success(&self) {
        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        // First call returns connection error (502 Bad Gateway)
        Mock::given(method("POST"))
            .and(path(&path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(502).set_body_json(json!({"error": {"message": "Bad Gateway"}})))
            .expect(1)
            .mount(&self.mock_server)
            .await;
        
        // Second call succeeds
        let response_body = json!({
            "choices": [{
                "message": {
                    "content": "1"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "model": "gpt-test"
        });

        Mock::given(method("POST"))
            .and(path(&path_pattern))
            .and(query_param("api-version", &self.api_version))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .expect(1)
            .mount(&self.mock_server)
            .await;
    }

    /// Mock chat completion with Bearer token authentication
    pub async fn mock_chat_completion_with_bearer_auth(&self, response_content: &str) {
        let response_body = json!({
            "choices": [{
                "message": {
                    "content": response_content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "model": "gpt-test"
        });

        let path_pattern = format!(
            "/openai/deployments/{}/chat/completions",
            self.deployment_id
        );

        Mock::given(method("POST"))
            .and(path(path_pattern))
            .and(query_param("api-version", &self.api_version))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }
}
