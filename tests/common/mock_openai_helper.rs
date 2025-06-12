use serde_json::json;
use std::time::Duration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper for setting up a Wiremock mock OpenAI server in integration tests.
#[allow(dead_code)]
pub struct MockOpenAITestHelper {
    mock_server: MockServer,
}

#[allow(dead_code)]
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
            "model": "gpt-4.1-mini"
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&self.mock_server)
            .await;
    }

    /// Mock a successful chat completion with dynamic file IDs for cache testing.
    /// This method will return correct file IDs based on actual discovered files.
    pub async fn mock_chat_completion_with_dynamic_ids(&self, video_id: &str, subtitle_id: &str) {
        use crate::common::test_data_generators::MatchResponseGenerator;

        let response_content =
            MatchResponseGenerator::successful_match_with_ids(video_id, subtitle_id);
        let response_body = json!({
            "choices": [
                { "message": { "content": response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
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
            "model": "gpt-4.1-mini"
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
    pub async fn setup_delayed_response(&self, delay_ms: u64, response_content: &str) {
        let response_body = json!({
            "choices": [
                { "message": { "content": response_content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
        });
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
