//! Integration tests for the `local` AI provider.
//!
//! Covers tasks §4.1 – §4.10 of the `add-local-llm-provider` OpenSpec change.
//!
//! Layout (see comments on each `#[tokio::test]` function):
//!   §4.1  end-to-end match flow via wiremock
//!   §4.2  `chat_completion` round-trip via translation-style invocation
//!   §4.3  retry behavior (503 once, then 200)
//!   §4.4  error mapping branches (refused / 404 / non-JSON / 500)
//!   §4.5  carve-out: `OPENAI_API_KEY` does NOT leak into local
//!   §4.6  trailing-slash URL joining
//!   §4.7  sanitized base URL with credentials in error messages
//!   §4.8  LAN address loads + factory constructs successfully
//!   §4.9  hosted provider with HTTP base URL is rejected with hint
//!   §4.10 hosted provider receiving non-OpenAI body emits hint (text + JSON)

use std::fs;
use std::sync::Arc;

use serde_json::json;
use subx_cli::cli::MatchArgs;
use subx_cli::commands::match_command;
use subx_cli::config::{
    Config, ProductionConfigService, TestConfigBuilder, TestConfigService, TestEnvironmentProvider,
    service::ConfigService, validator::validate_ai_config,
};
use subx_cli::core::factory::ComponentFactory;
use subx_cli::services::ai::AIProvider;
use subx_cli::services::ai::local::LocalLLMClient;
use tempfile::TempDir;
use wiremock::matchers::{method, path as wm_path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::test_data_generators::MatchResponseGenerator;

/// Serialize tests that touch `XDG_CONFIG_HOME` (for the match cache
/// directory) so they don't race against each other or other suites.
static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ───────────────────────────────────────────────────────────────────────
// Helpers
// ───────────────────────────────────────────────────────────────────────

/// Build a `TestConfigService` configured for the local provider with no
/// API key. The standard `with_mock_ai_server` helper would force an
/// `api_key = "mock-api-key"`; the local provider's defining property is
/// that the key is optional, so we set fields directly here.
fn build_local_service(base_url: &str, model: &str) -> TestConfigService {
    let mut config = TestConfigBuilder::new()
        .with_ai_provider("local")
        .with_ai_model(model)
        .with_ai_base_url(base_url)
        .with_ai_retry(2, 10)
        .build_config();
    config.ai.api_key = None;
    TestConfigService::new(config)
}

/// Build a `LocalLLMClient` directly from a synthesized `AIConfig`, with
/// no API key and tight retry/timeout for fast tests.
fn make_local_client(base_url: &str) -> LocalLLMClient {
    let mut config = Config::default();
    config.ai.provider = "local".to_string();
    config.ai.model = "llama3.1:8b-instruct".to_string();
    config.ai.base_url = base_url.to_string();
    config.ai.api_key = None;
    config.ai.retry_attempts = 2;
    config.ai.retry_delay_ms = 5;
    config.ai.request_timeout_seconds = 10;
    LocalLLMClient::from_config(&config.ai).expect("LocalLLMClient::from_config")
}

/// Mount an unauthenticated POST `/chat/completions` mock returning the
/// supplied JSON body, with an exact expected call count.
async fn mount_chat_completion(
    server: &MockServer,
    response: serde_json::Value,
    expect: u64,
    request_path: &str,
) {
    Mock::given(method("POST"))
        .and(wm_path(request_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(expect)
        .mount(server)
        .await;
}

fn openai_envelope(content: &str) -> serde_json::Value {
    json!({
        "choices": [
            { "message": { "content": content }, "finish_reason": "stop" }
        ],
        "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 },
        "model": "llama3.1:8b-instruct"
    })
}

// ───────────────────────────────────────────────────────────────────────
// §4.1 — Match flow end-to-end via wiremock
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_1_match_flow_end_to_end_via_local_provider() {
    let _guard = ENV_MUTEX.lock().await;
    let tmp = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }

    fs::write(tmp.path().join("movie.mp4"), b"video").unwrap();
    fs::write(tmp.path().join("movie.srt"), b"sub").unwrap();

    use subx_cli::core::matcher::FileDiscovery;
    let files = FileDiscovery::new()
        .scan_directory(tmp.path(), false)
        .unwrap();
    let video = files.iter().find(|f| f.name.ends_with(".mp4")).unwrap();
    let subtitle = files.iter().find(|f| f.name.ends_with(".srt")).unwrap();

    let server = MockServer::start().await;
    let body = MatchResponseGenerator::successful_match_with_ids(&video.id, &subtitle.id);
    mount_chat_completion(&server, openai_envelope(&body), 1, "/chat/completions").await;

    let config_service = build_local_service(&server.uri(), "llama3.1:8b-instruct");

    let args = MatchArgs {
        input_paths: vec![],
        recursive: false,
        path: Some(tmp.path().to_path_buf()),
        dry_run: true,
        confidence: 80,
        backup: false,
        copy: false,
        move_files: false,
        no_extract: false,
    };

    match_command::execute(args, &config_service)
        .await
        .expect("match flow against local provider mock");

    let received = server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        1,
        "expected exactly one /chat/completions call"
    );
    let req = &received[0];
    assert_eq!(req.url.path(), "/chat/completions");
    // Local provider with no api_key MUST NOT send Authorization.
    assert!(
        req.headers.get("authorization").is_none(),
        "local provider with api_key=None must not send Authorization header"
    );
    let payload: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
    assert_eq!(payload["model"], "llama3.1:8b-instruct");
}

// ───────────────────────────────────────────────────────────────────────
// §4.2 — chat_completion round-trip
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_2_chat_completion_round_trips_assistant_content() {
    let server = MockServer::start().await;
    mount_chat_completion(
        &server,
        openai_envelope("Bonjour le monde"),
        1,
        "/chat/completions",
    )
    .await;

    let client = make_local_client(&server.uri());
    let messages = vec![
        json!({"role": "system", "content": "Translate to French."}),
        json!({"role": "user", "content": "Hello world"}),
    ];
    let out = AIProvider::chat_completion(&client, messages)
        .await
        .expect("chat_completion ok");
    assert_eq!(out, "Bonjour le monde");
}

// ───────────────────────────────────────────────────────────────────────
// §4.3 — Retry behavior (503 once, then 200)
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_3_retries_once_on_503_then_succeeds() {
    let server = MockServer::start().await;

    // First call: 503. `up_to_n_times(1)` ensures only the first match.
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(503).set_body_string("transient"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // Second call: 200.
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_envelope("ok")))
        .expect(1)
        .mount(&server)
        .await;

    let client = make_local_client(&server.uri());
    let result =
        AIProvider::chat_completion(&client, vec![json!({"role": "user", "content": "ping"})])
            .await;
    assert!(
        result.is_ok(),
        "retry should succeed on second attempt: {:?}",
        result
    );
    assert_eq!(result.unwrap(), "ok");

    let received = server.received_requests().await.unwrap();
    assert_eq!(
        received.len(),
        2,
        "expected exactly two HTTP requests (1 retry)"
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.4 — Error mapping branches
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_4a_connection_refused_maps_to_unreachable() {
    // Port 1 has no listener and connect() fails immediately.
    let mut config = Config::default();
    config.ai.provider = "local".to_string();
    config.ai.model = "llama3.1:8b-instruct".to_string();
    config.ai.base_url = "http://127.0.0.1:1/v1".to_string();
    config.ai.api_key = None;
    config.ai.retry_attempts = 0;
    config.ai.retry_delay_ms = 1;
    config.ai.request_timeout_seconds = 5;
    let client = LocalLLMClient::from_config(&config.ai).unwrap();

    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("local LLM endpoint unreachable"),
        "expected `local LLM endpoint unreachable` prefix; got: {msg}"
    );
    assert!(msg.contains("http://127.0.0.1:1/v1"), "got: {msg}");
}

#[tokio::test]
async fn s4_4b_http_404_with_model_body_maps_to_model_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_string(r#"{"error":"model not found: llama3.1:8b-instruct"}"#),
        )
        .mount(&server)
        .await;

    let client = make_local_client(&server.uri());
    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("local LLM model not found"),
        "expected `local LLM model not found` prefix; got: {msg}"
    );
}

#[tokio::test]
async fn s4_4c_http_200_non_json_body_maps_to_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
        .mount(&server)
        .await;

    let client = make_local_client(&server.uri());
    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("local LLM response was not OpenAI-compatible JSON"),
        "expected non-JSON parse error prefix; got: {msg}"
    );
}

#[tokio::test]
async fn s4_4d_http_500_includes_sanitized_body() {
    let server = MockServer::start().await;
    // 500 with no retry attempts so the body lands directly in the error.
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("kaboom inside backend"))
        .mount(&server)
        .await;

    let mut config = Config::default();
    config.ai.provider = "local".to_string();
    config.ai.model = "llama3.1:8b-instruct".to_string();
    config.ai.base_url = server.uri();
    config.ai.api_key = None;
    config.ai.retry_attempts = 0; // do not retry past 500 for this branch
    config.ai.retry_delay_ms = 1;
    config.ai.request_timeout_seconds = 10;
    let client = LocalLLMClient::from_config(&config.ai).unwrap();

    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("local LLM endpoint returned HTTP 500"),
        "expected `local LLM endpoint returned HTTP 500` prefix; got: {msg}"
    );
    assert!(
        msg.contains("kaboom inside backend"),
        "expected sanitized body in error: {msg}"
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.5 — Carve-out: `OPENAI_API_KEY` does NOT leak into local
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_5_openai_api_key_does_not_leak_into_local_provider() {
    // Sandbox SUBX_CONFIG_PATH inside a TempDir so no user config is read.
    let _guard = ENV_MUTEX.lock().await;
    let tmp = TempDir::new().unwrap();
    let cfg_path = tmp.path().join("subx-config.toml");
    fs::write(
        &cfg_path,
        r#"[ai]
provider = "local"
base_url = "http://localhost:11434/v1"
model = "llama3.1:8b-instruct"
"#,
    )
    .unwrap();

    let mut env = TestEnvironmentProvider::new();
    env.set_var("SUBX_CONFIG_PATH", cfg_path.to_str().unwrap());
    env.set_var("OPENAI_API_KEY", "sk-leak");
    let svc = ProductionConfigService::with_env_provider(Arc::new(env)).unwrap();

    let cfg = svc.get_config().expect("config loads");
    assert_eq!(cfg.ai.provider, "local");
    assert!(
        cfg.ai.api_key.is_none(),
        "OPENAI_API_KEY MUST NOT leak into local; got: {:?}",
        cfg.ai.api_key
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.6 — Trailing-slash URL joining
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_6_trailing_slash_base_url_yields_single_slash_path() {
    let server = MockServer::start().await;
    // Mock matches the canonical /v1/chat/completions path. If the client
    // produced /v1//chat/completions wiremock would fail the expectation.
    Mock::given(method("POST"))
        .and(wm_path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_envelope("ok")))
        .expect(1)
        .mount(&server)
        .await;

    let base = format!("{}/v1/", server.uri()); // explicit trailing slash
    let client = make_local_client(&base);
    let _ = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .expect("chat_completion ok");

    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    let p = received[0].url.path();
    assert_eq!(
        p, "/v1/chat/completions",
        "path must be canonical, got: {p}"
    );
    assert!(!p.contains("//"), "no doubled slashes; got: {p}");
}

// ───────────────────────────────────────────────────────────────────────
// §4.7 — Sanitized base URL with credentials
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_7_credentials_are_stripped_from_error_messages() {
    let mut config = Config::default();
    config.ai.provider = "local".to_string();
    config.ai.model = "llama3.1:8b-instruct".to_string();
    config.ai.base_url = "http://user:secret@127.0.0.1:65500/v1?token=abc".to_string();
    config.ai.api_key = None;
    config.ai.retry_attempts = 0;
    config.ai.retry_delay_ms = 1;
    config.ai.request_timeout_seconds = 5;
    let client = LocalLLMClient::from_config(&config.ai).unwrap();

    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("http://127.0.0.1:65500/v1"),
        "sanitized URL must appear; got: {msg}"
    );
    assert!(
        !msg.contains("secret"),
        "password must not leak; got: {msg}"
    );
    assert!(
        !msg.contains("user:secret"),
        "userinfo must not leak; got: {msg}"
    );
    assert!(
        !msg.contains("token="),
        "query string must not leak; got: {msg}"
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.8 — LAN address loads successfully (no actual network call)
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_8_lan_base_url_loads_validates_and_constructs() {
    let svc = build_local_service("http://192.168.50.50:11434/v1", "llama3.1:8b-instruct");
    let cfg = svc.get_config().unwrap();
    validate_ai_config(&cfg.ai).expect("validator accepts LAN HTTP for local");

    let factory = ComponentFactory::new(&svc).unwrap();
    let provider = factory.create_ai_provider();
    assert!(
        provider.is_ok(),
        "ComponentFactory must construct local provider for LAN URL: {:?}",
        provider.err()
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.9 — Hosted provider with HTTP base URL is rejected with hint
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_9_hosted_provider_with_http_url_is_rejected_with_local_hint() {
    let mut config = Config::default();
    config.ai.provider = "openai".to_string();
    config.ai.api_key = Some("sk-test-1234567890".to_string());
    config.ai.base_url = "http://localhost:11434/v1".to_string();
    config.ai.model = "gpt-4.1-mini".to_string();

    let err = validate_ai_config(&config.ai)
        .expect_err("HTTP base_url for hosted provider must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("local"),
        "error must mention `local`; got: {msg}"
    );
    assert!(
        msg.contains("ollama"),
        "error must mention `ollama`; got: {msg}"
    );
}

// ───────────────────────────────────────────────────────────────────────
// §4.10 — Hosted provider hint emission (text + JSON modes)
//
// The hosted-client side of hint emission (§3.5) lives in
// `src/services/ai/hosted_hint.rs`. Both arms below verify the
// local-provider hint surfaces in hosted-provider error envelopes.
// ───────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn s4_10_text_mode_hosted_provider_non_openai_body_emits_hint() {
    // Hosted clients require HTTPS; Azure OpenAI is the easiest hosted
    // arm to point at HTTP wiremock since its config path is structured
    // around `endpoint`/`deployment_id`. However, the validator now
    // rejects http:// for all hosted providers, so we drive the client
    // directly rather than through the factory.
    use subx_cli::services::ai::openai::OpenAIClient;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"hello":"world"})))
        .mount(&server)
        .await;

    let mut config = Config::default();
    config.ai.provider = "openai".to_string();
    config.ai.api_key = Some("sk-test-1234567890".to_string());
    config.ai.base_url = server.uri();
    config.ai.model = "gpt-4.1-mini".to_string();
    config.ai.retry_attempts = 0;
    config.ai.retry_delay_ms = 1;
    config.ai.request_timeout_seconds = 10;

    let client = OpenAIClient::from_config(&config.ai).unwrap();
    let err = AIProvider::chat_completion(&client, vec![json!({"role":"user","content":"x"})])
        .await
        .expect_err("non-OpenAI body must surface as error");
    let msg = err.to_string();
    assert!(
        msg.contains("local") && msg.contains("ollama"),
        "expected local-provider hint in hosted error; got: {msg}"
    );
}

#[tokio::test]
async fn s4_10_json_mode_hosted_provider_non_openai_body_emits_hint() {
    // JSON-mode envelope test — invokes the binary directly so we can
    // observe the structured error envelope that the global `--output
    // json` flag produces.
    use std::process::Command;

    let _guard = ENV_MUTEX.lock().await;
    let tmp = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }
    fs::write(tmp.path().join("movie.mp4"), b"video").unwrap();
    fs::write(tmp.path().join("movie.srt"), b"sub").unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wm_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"hello":"world"})))
        .mount(&server)
        .await;

    let bin = env!("CARGO_BIN_EXE_subx-cli");
    let output = Command::new(bin)
        .args([
            "--output",
            "json",
            "match",
            "--dry-run",
            tmp.path().to_str().unwrap(),
        ])
        .env("SUBX_AI_PROVIDER", "openai")
        .env("SUBX_AI_BASE_URL", server.uri())
        .env("SUBX_AI_APIKEY", "sk-test-1234567890")
        .env("SUBX_AI_MODEL", "gpt-4.1-mini")
        .output()
        .expect("run subx-cli");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    assert!(
        combined.contains("local") && combined.contains("ollama"),
        "expected local-provider hint in JSON envelope; got stdout=`{stdout}` stderr=`{stderr}`"
    );
}
