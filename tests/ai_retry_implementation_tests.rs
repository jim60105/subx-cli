use reqwest::Client;
/// Additional tests for HttpRetryClient trait implementations to increase coverage
/// This test file focuses on edge cases and error handling scenarios
use subx_cli::services::ai::retry::HttpRetryClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;

/// Mock implementation of HttpRetryClient for testing
struct MockRetryClient {
    retry_attempts: u32,
    retry_delay_ms: u64,
}

impl HttpRetryClient for MockRetryClient {
    fn retry_attempts(&self) -> u32 {
        self.retry_attempts
    }

    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }
}

/// Test HttpRetryClient trait implementation success case
#[tokio::test]
async fn test_http_retry_client_success() {
    let mock_server = MockServer::start().await;

    // Configure mock to succeed immediately
    Mock::given(method("POST"))
        .and(path("/success"))
        .respond_with(ResponseTemplate::new(200).set_body_string("immediate success"))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 100,
    };

    let http_client = Client::new();
    let request = http_client.post(&format!("{}/success", mock_server.uri()));

    let result = client.make_request_with_retry(request).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "immediate success");
}

/// Test HttpRetryClient with retry behavior
#[tokio::test]
async fn test_http_retry_client_with_retries() {
    let mock_server = MockServer::start().await;

    // Configure mock to always fail
    Mock::given(method("POST"))
        .and(path("/retry_fail"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Configure mock to succeed
    Mock::given(method("POST"))
        .and(path("/retry_success"))
        .respond_with(ResponseTemplate::new(200).set_body_string("retry success"))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 50,
    };

    let http_client = Client::new();

    // Test failure case - should fail after retries
    let request_fail = http_client.post(&format!("{}/retry_fail", mock_server.uri()));
    let result_fail = client.make_request_with_retry(request_fail).await;
    assert!(result_fail.is_err());

    // Test success case - should succeed immediately
    let request_success = http_client.post(&format!("{}/retry_success", mock_server.uri()));
    let result_success = client.make_request_with_retry(request_success).await;
    assert!(result_success.is_ok());
    let response = result_success.unwrap();
    assert_eq!(response.status(), 200);
}

/// Test HttpRetryClient exhausted retries
#[tokio::test]
async fn test_http_retry_client_exhausted_retries() {
    let mock_server = MockServer::start().await;

    // Configure mock to always fail
    Mock::given(method("POST"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 2,
        retry_delay_ms: 50,
    };

    let http_client = Client::new();
    let request = http_client.post(&format!("{}/fail", mock_server.uri()));

    let result = client.make_request_with_retry(request).await;
    assert!(result.is_err());
}

/// Test HttpRetryClient HTTP error status handling
#[tokio::test]
async fn test_http_retry_client_http_error_status() {
    let mock_server = MockServer::start().await;

    // Configure mock to return 404
    Mock::given(method("POST"))
        .and(path("/notfound"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 50,
    };

    let http_client = Client::new();
    let request = http_client.post(&format!("{}/notfound", mock_server.uri()));

    let result = client.make_request_with_retry(request).await;
    assert!(result.is_err());
}

/// Test error_for_status behavior in retry loop
#[tokio::test]
async fn test_error_for_status_in_retry_loop() {
    let mock_server = MockServer::start().await;

    // Configure mock to return server errors
    Mock::given(method("POST"))
        .and(path("/server_error"))
        .respond_with(ResponseTemplate::new(502)) // Bad Gateway
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/success_after_error"))
        .respond_with(ResponseTemplate::new(200).set_body_string("finally success"))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 30,
    };

    let http_client = Client::new();

    // Test error case
    let request_error = http_client.post(&format!("{}/server_error", mock_server.uri()));
    let result_error = client.make_request_with_retry(request_error).await;
    assert!(result_error.is_err());

    // Test success case
    let request_success = http_client.post(&format!("{}/success_after_error", mock_server.uri()));
    let result_success = client.make_request_with_retry(request_success).await;
    assert!(result_success.is_ok());
    let response = result_success.unwrap();
    assert_eq!(response.status(), 200);
}

/// Test zero retry attempts
#[tokio::test]
async fn test_zero_retry_attempts() {
    let mock_server = MockServer::start().await;

    // Configure mock to fail
    Mock::given(method("POST"))
        .and(path("/zero_retry"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 0,
        retry_delay_ms: 100,
    };

    let http_client = Client::new();
    let request = http_client.post(&format!("{}/zero_retry", mock_server.uri()));

    let result = client.make_request_with_retry(request).await;
    assert!(result.is_err());
}

/// Test request cloning behavior
#[tokio::test]
async fn test_request_cloning_with_body() {
    let mock_server = MockServer::start().await;

    // Configure mock to succeed
    Mock::given(method("POST"))
        .and(path("/clone_test_success"))
        .respond_with(ResponseTemplate::new(200).set_body_string("clone success"))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 50,
    };

    let http_client = Client::new();

    // Test with headers and JSON body that needs cloning
    let request_with_body = http_client
        .post(&format!("{}/clone_test_success", mock_server.uri()))
        .header("X-Test", "clone-test")
        .json(&serde_json::json!({"test": "data"}));

    let result = client.make_request_with_retry(request_with_body).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status(), 200);
}

/// Test network connection errors
#[tokio::test]
async fn test_network_connection_errors() {
    let client = MockRetryClient {
        retry_attempts: 2,
        retry_delay_ms: 50,
    };

    let http_client = Client::new();

    // Try to connect to an invalid URL
    let request = http_client.post("http://invalid-url-that-does-not-exist.local/test");

    let result = client.make_request_with_retry(request).await;
    assert!(result.is_err());
}

/// Test very short retry delay
#[tokio::test]
async fn test_very_short_retry_delay() {
    let mock_server = MockServer::start().await;

    // Configure mock to fail
    Mock::given(method("POST"))
        .and(path("/short_delay_fail"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    // Configure mock to succeed
    Mock::given(method("POST"))
        .and(path("/short_delay_success"))
        .respond_with(ResponseTemplate::new(200).set_body_string("short delay success"))
        .mount(&mock_server)
        .await;

    let client = MockRetryClient {
        retry_attempts: 3,
        retry_delay_ms: 1, // Very short delay
    };

    let http_client = Client::new();

    // Test timing with very short delay
    let start_time = std::time::Instant::now();
    let request_fail = http_client.post(&format!("{}/short_delay_fail", mock_server.uri()));
    let result_fail = client.make_request_with_retry(request_fail).await;
    let elapsed = start_time.elapsed();

    assert!(result_fail.is_err());
    // Even with 1ms delay, there should be some minimal elapsed time for retries
    assert!(elapsed.as_millis() >= 1);

    // Test success case
    let request_success = http_client.post(&format!("{}/short_delay_success", mock_server.uri()));
    let result_success = client.make_request_with_retry(request_success).await;
    assert!(result_success.is_ok());
}

/// Test large retry delay configuration
#[tokio::test]
async fn test_large_retry_delay() {
    let client = MockRetryClient {
        retry_attempts: 1,
        retry_delay_ms: 5000, // Large delay
    };

    // Verify configuration values are properly stored and returned
    assert_eq!(client.retry_attempts(), 1);
    assert_eq!(client.retry_delay_ms(), 5000);
}

/// Test trait method return values
#[tokio::test]
async fn test_trait_method_values() {
    let client = MockRetryClient {
        retry_attempts: 42,
        retry_delay_ms: 1337,
    };

    // Test that trait methods return configured values
    assert_eq!(client.retry_attempts(), 42);
    assert_eq!(client.retry_delay_ms(), 1337);
}
