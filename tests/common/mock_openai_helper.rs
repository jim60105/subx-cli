use serde_json::{Value, json};
use std::time::Duration;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

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

    /// Return the underlying [`MockServer`] for tests that need to mount
    /// custom closure-based responders directly.
    pub fn server(&self) -> &MockServer {
        &self.mock_server
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

    /// Mock a chat completion that echoes file IDs from the captured request.
    ///
    /// The mock parses the incoming OpenAI prompt body, extracts `file_<uuid>`
    /// identifiers in the order they appear (videos first because the prompt
    /// builder always emits the video file list before the subtitle list),
    /// and synthesizes a JSON `MatchResult` that pairs them 1:1 with the
    /// supplied `confidence`.
    ///
    /// This is useful for tests that drive the matcher end-to-end and need
    /// the AI response to reference the same UUIDv7 file IDs that the
    /// matcher generated during its own discovery scan.
    pub async fn mock_chat_completion_echoing_request_ids(
        &self,
        video_count: usize,
        subtitle_count: usize,
        confidence: f64,
    ) {
        self.mock_chat_completion_echoing_request_ids_inner(
            video_count,
            subtitle_count,
            confidence,
            false,
            None,
        )
        .await;
    }

    /// Same as [`Self::mock_chat_completion_echoing_request_ids`] but also
    /// asserts the mock is hit exactly `expected_calls` times.
    pub async fn mock_chat_completion_echoing_request_ids_with_expectation(
        &self,
        video_count: usize,
        subtitle_count: usize,
        confidence: f64,
        expected_calls: u64,
    ) {
        self.mock_chat_completion_echoing_request_ids_inner(
            video_count,
            subtitle_count,
            confidence,
            false,
            Some(expected_calls),
        )
        .await;
    }

    async fn mock_chat_completion_echoing_request_ids_inner(
        &self,
        video_count: usize,
        subtitle_count: usize,
        confidence: f64,
        fan_out: bool,
        expected_calls: Option<u64>,
    ) {
        let builder = Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer mock-api-key"))
            .and(header("content-type", "application/json"))
            .respond_with(EchoIdsResponder {
                video_count,
                subtitle_count,
                confidence,
                fan_out,
            });
        let mounted = if let Some(n) = expected_calls {
            builder.expect(n)
        } else {
            builder
        };
        mounted.mount(&self.mock_server).await;
    }

    /// Mock a chat completion that pairs the first video ID against EVERY
    /// subtitle ID present in the captured request prompt.
    ///
    /// Used by the duplicate-rename / conflict-resolution tests that need
    /// the AI to suggest collapsing many subtitles onto a single video.
    pub async fn mock_chat_completion_echoing_one_to_many(
        &self,
        video_count: usize,
        subtitle_count: usize,
        confidence: f64,
    ) {
        self.mock_chat_completion_echoing_request_ids_inner(
            video_count,
            subtitle_count,
            confidence,
            true,
            None,
        )
        .await;
    }
}

struct EchoIdsResponder {
    video_count: usize,
    subtitle_count: usize,
    confidence: f64,
    fan_out: bool,
}

impl wiremock::Respond for EchoIdsResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value =
            serde_json::from_slice(&request.body).expect("OpenAI request body must be valid JSON");
        let prompt = body
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.last())
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        // Extract UUIDv7-shaped file IDs in the order they appear in the
        // prompt. The prompt builder always emits the video block first,
        // followed by the subtitle block, so we can take the first
        // `video_count` IDs as videos and the next `subtitle_count` as
        // subtitles.
        let id_pattern = regex::Regex::new(
            r"file_[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[0-9a-f]{4}-[0-9a-f]{12}",
        )
        .expect("static regex compiles");
        let ids: Vec<String> = id_pattern
            .find_iter(prompt)
            .map(|m| m.as_str().to_string())
            .collect();

        let videos: Vec<&str> = ids
            .iter()
            .take(self.video_count)
            .map(String::as_str)
            .collect();
        let subtitles: Vec<&str> = ids
            .iter()
            .skip(self.video_count)
            .take(self.subtitle_count)
            .map(String::as_str)
            .collect();

        let pairs = videos.len().min(subtitles.len());
        let matches: Vec<Value> = if self.fan_out {
            let video = videos.first().copied().unwrap_or("");
            subtitles
                .iter()
                .map(|sub| {
                    json!({
                        "video_file_id": video,
                        "subtitle_file_id": sub,
                        "confidence": self.confidence,
                        "match_factors": ["echoed_from_request"],
                    })
                })
                .collect()
        } else {
            (0..pairs)
                .map(|i| {
                    json!({
                        "video_file_id": videos[i],
                        "subtitle_file_id": subtitles[i],
                        "confidence": self.confidence,
                        "match_factors": ["echoed_from_request"],
                    })
                })
                .collect()
        };

        let content = json!({
            "matches": matches,
            "confidence": self.confidence,
            "reasoning": "Echoing IDs captured from the live request",
        })
        .to_string();

        let response_body = json!({
            "choices": [
                { "message": { "content": content }, "finish_reason": "stop" }
            ],
            "usage": { "prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150 },
            "model": "gpt-4.1-mini"
        });

        ResponseTemplate::new(200).set_body_json(response_body)
    }
}
