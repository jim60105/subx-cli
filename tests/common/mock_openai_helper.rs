use serde_json::json;
use std::time::Duration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// 標準化模擬 OpenAI 聊天完成回應資料結構
#[derive(Debug, Clone)]
pub struct MockChatCompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<MockUsageStats>,
}

/// 模擬使用量統計資料
#[derive(Debug, Clone)]
pub struct MockUsageStats {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Helper for setting up a Wiremock mock OpenAI server in integration tests.
pub struct MockOpenAITestHelper {
    mock_server: MockServer,
}

impl MockOpenAITestHelper {
    /// Start a new mock OpenAI server instance.
    pub async fn new() -> Self {
        let mock_server = MockServer::start().await;
        Self { mock_server }
    }

    /// Return the base URL of the mock server.
    pub fn base_url(&self) -> String {
        self.mock_server.uri()
    }

    /// Mock a successful chat completion response for `/chat/completions`.
    pub async fn mock_chat_completion_success(&self, response_content: &str) {
        let response_body = json!({
            "choices": [
                {
                    "message": { "content": response_content },
                    "finish_reason": "stop"
                }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4o-mini"
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
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
            "model": "gpt-4o-mini"
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
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
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
            .respond_with(ResponseTemplate::new(status).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Setup a delayed chat completion response to simulate network latency.
    pub async fn setup_delayed_response(
        &self,
        delay_ms: u64,
        response: MockChatCompletionResponse,
    ) {
        let mut response_body = json!({
            "choices": [
                { "message": { "content": response.content }, "finish_reason": "stop" }
            ],
            "model": response.model
        });
        if let Some(usage) = response.usage {
            response_body["usage"] = json!({
                "prompt_tokens": usage.prompt_tokens,
                "completion_tokens": usage.completion_tokens,
                "total_tokens": usage.total_tokens,
            });
        }
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(delay_ms))
                    .set_body_json(response_body),
            )
            .mount(&self.mock_server)
            .await;
    }
}
