use serde_json::json;
use subx_cli::config::WhisperConfig;
use subx_cli::services::whisper::WhisperApiClient;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_whisper_api_mock_success() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/audio/transcriptions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "text": "Hello world",
            "segments": [{"start": 0.5, "end": 2.0, "text": "Hello world"}],
            "words": [{"word": "Hello", "start": 0.5, "end": 1.0}]
        })))
        .mount(&mock_server)
        .await;

    let config = WhisperConfig::default();
    let client = WhisperApiClient::new("test-key".to_string(), mock_server.uri(), config).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path();
    let res = client.transcribe(path).await.unwrap();
    assert_eq!(res.text, "Hello world");
    assert!(!res.segments.is_empty());
    assert!(res.words.is_some());
}
