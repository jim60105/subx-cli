## ADDED Requirements

### Requirement: Local LLM Provider Identifier

The system SHALL recognize the AI provider identifier `local` (with `ollama` accepted as an alias normalized to `local` by the single canonicalization function `normalize_ai_provider` defined in the `configuration-management` capability) as a first-class value of `config.ai.provider`. `ComponentFactory::create_ai_provider` SHALL receive only the canonical form (`local`) — the alias `ollama` SHALL never reach the factory dispatch — and SHALL dispatch this identifier to a dedicated `LocalLLMClient` implementing the `AIProvider` trait under `src/services/ai/local.rs`.

#### Scenario: `local` selects the local LLM client
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, and `ai.model = "llama3.1:8b-instruct"`
- **WHEN** `ComponentFactory::create_ai_provider` is invoked
- **THEN** it SHALL return a boxed `LocalLLMClient` configured with the supplied `base_url`, `model`, `temperature`, `max_tokens`, retry settings, and request timeout

#### Scenario: `ollama` alias is normalized to `local`
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs (invoking `normalize_ai_provider`) and configuration is persisted
- **THEN** the persisted `ai.provider` value SHALL be `"local"` and a subsequent `ComponentFactory::create_ai_provider` call SHALL return the `LocalLLMClient`

### Requirement: OpenAI-Compatible Local Chat Completions

`LocalLLMClient` SHALL issue `POST {base_url}/chat/completions` requests using the OpenAI-compatible JSON request body (`model`, `messages`, `temperature`, `max_tokens`) and SHALL parse responses through the shared `ResponseParser` so that the matching and translation pipelines see the same `MatchResult` / `ConfidenceScore` / assistant-text shapes as the hosted providers. The request URL SHALL be joined such that exactly one `/` separates `base_url` from `chat/completions`: a trailing slash on `base_url` SHALL NOT produce a `//` in the final URL, and a missing trailing slash SHALL NOT collapse the segments.

#### Scenario: Match request hits the configured local endpoint
- **GIVEN** `ai.base_url = "http://localhost:8080/v1"` and a wiremock handler that matches `POST /chat/completions` and returns a valid OpenAI-shaped JSON response
- **WHEN** the match engine calls `LocalLLMClient::analyze_content`
- **THEN** exactly one HTTP request SHALL be issued to `http://localhost:8080/v1/chat/completions` with a JSON body containing `model`, `messages`, `temperature`, and `max_tokens`, and the parsed `MatchResult` SHALL match the mock response

#### Scenario: Trailing slash on base_url does not produce a double slash
- **GIVEN** `ai.base_url = "http://localhost:11434/v1/"` (with a trailing slash) and a wiremock handler matching `POST /v1/chat/completions`
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** exactly one HTTP request SHALL be issued to `http://localhost:11434/v1/chat/completions` (single `/` between `v1` and `chat`), and the wiremock SHALL NOT observe a request path containing `//chat/completions`

#### Scenario: Translation `chat_completion` is supported
- **GIVEN** `ai.provider = "local"` and a wiremock returning a single-choice assistant message
- **WHEN** the translation engine calls `LocalLLMClient::chat_completion(messages)`
- **THEN** the returned string SHALL equal the assistant content of the first choice and SHALL NOT return the default "AI provider does not support chat_completion" error

### Requirement: Optional API Key, Required Base URL

When `ai.provider = "local"`, the system SHALL treat `ai.api_key` as optional (a missing or empty key is valid) and SHALL require `ai.base_url` to be a non-empty, syntactically valid URL. If `ai.api_key` is non-empty, `LocalLLMClient` SHALL include it as a `Bearer` token in the `Authorization` header; otherwise the header SHALL be omitted.

#### Scenario: Empty API key is accepted
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, `ai.model = "llama3.1"`, and `ai.api_key` is `None`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL succeed and `LocalLLMClient::from_config` SHALL construct the client without an `Authorization` header

#### Scenario: Missing base_url is rejected
- **GIVEN** `ai.provider = "local"` and `ai.base_url` is the empty string
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return a configuration error whose message indicates that `ai.base_url` is required for the local provider

#### Scenario: Bearer token forwarded when present
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:8000/v1"`, and `ai.api_key = Some("vllm-shared-token")`
- **WHEN** `LocalLLMClient` issues a chat-completions request
- **THEN** the request SHALL include the header `Authorization: Bearer vllm-shared-token`

### Requirement: Privacy and Offline Network Policy

When the resolved `ai.provider` is `local`, the system SHALL contact only the configured `ai.base_url`. It SHALL NOT contact any hosted AI endpoint (`api.openai.com`, `openrouter.ai`, `*.openai.azure.com`), SHALL NOT emit telemetry, and SHALL NOT silently fall back to a hosted provider on local-endpoint failure.

#### Scenario: Local provider does not fall back to hosted endpoints
- **GIVEN** `ai.provider = "local"` and the configured `base_url` returns connection-refused on every request
- **WHEN** the match engine calls `LocalLLMClient::analyze_content`
- **THEN** the engine SHALL surface a configuration/transport error from the local endpoint and SHALL NOT issue any HTTP request to a hosted provider domain

#### Scenario: Hosted-provider env vars do not switch provider away from local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the process environment has `OPENAI_API_KEY=sk-...` and `OPENROUTER_API_KEY=or-...` set
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `ai.provider` SHALL remain `"local"` and `ai.api_key` SHALL NOT be populated from `OPENAI_API_KEY` or `OPENROUTER_API_KEY`

### Requirement: Local Provider Environment Variable Overrides

`ProductionConfigService` SHALL recognize `LOCAL_LLM_BASE_URL` (mapping to `ai.base_url`) and `LOCAL_LLM_API_KEY` (mapping to `ai.api_key`) but SHALL apply them only when the canonicalized `ai.provider` (after `normalize_ai_provider`) is `"local"`. The standard `SUBX_AI_*` overrides (`SUBX_AI_PROVIDER`, `SUBX_AI_MODEL`, `SUBX_AI_BASE_URL`, `SUBX_AI_APIKEY`) SHALL continue to take precedence over file-backed configuration as defined by the `configuration-management` capability. `SUBX_AI_PROVIDER=ollama` SHALL be supported and normalized to `"local"` so that a user who sets only the env var still receives the local-provider behavior including the hosted-provider env-var carve-out.

#### Scenario: `LOCAL_LLM_BASE_URL` honored only for local provider
- **GIVEN** `ai.provider = "local"` and `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:11434/v1"`

#### Scenario: `LOCAL_LLM_*` ignored when provider is not local
- **GIVEN** `ai.provider = "openai"`, `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`, and `LOCAL_LLM_API_KEY=secret`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL NOT be set from `LOCAL_LLM_BASE_URL` and `config.ai.api_key` SHALL NOT be set from `LOCAL_LLM_API_KEY`

### Requirement: Actionable Local-Endpoint Error Mapping

`LocalLLMClient` SHALL classify common local-endpoint failures into distinct `SubXError::AiService` messages so users can distinguish "server not running" from "model not loaded" from "incompatible response". Sensitive request bodies and headers SHALL be sanitized via the existing `error_sanitizer` before being included in error messages.

When a `base_url` is included in an error message, it SHALL be a sanitized form that contains only the `scheme`, `host`, `port`, and `path` components. The sanitized form SHALL NOT include any query string, fragment, userinfo (`user:password@`), or `Authorization` / `api_key` value, in conformance with the `error-handling` capability's "Error types and messages SHALL NOT include … full request/response URLs that contain query parameters" requirement. The sanitization SHALL be performed by a dedicated helper (e.g. `sanitize_base_url(&str) -> String`) that is unit-tested independently and reused across all local-endpoint error variants.

#### Scenario: Connection refused yields a server-not-running error
- **GIVEN** `ai.base_url` points at a port where no server is listening
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message contains `local LLM endpoint unreachable` and the **sanitized** base URL (scheme + host + port + path only)

#### Scenario: HTTP 404 for an unknown model yields a model-not-found error
- **GIVEN** the local endpoint responds with HTTP 404 and a body indicating an unknown model
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message contains `local LLM model not found` and the configured `ai.model` value

#### Scenario: Non-JSON response yields a parse-shape error
- **GIVEN** the local endpoint responds with HTTP 200 and a body that is not OpenAI-compatible JSON
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned error SHALL be `SubXError::AiService` whose message indicates that the response was not OpenAI-compatible JSON

#### Scenario: Base URL with credentials and query string is sanitized in error
- **GIVEN** `ai.base_url = "http://user:secret@localhost:11434/v1?token=abc"` and the endpoint is unreachable
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the returned `SubXError::AiService` message SHALL contain `http://localhost:11434/v1` and SHALL NOT contain `user:secret`, `secret`, `token=`, or `?abc`

### Requirement: Shared Retry, Prompt, and Response Behavior

`LocalLLMClient` SHALL implement `HttpRetryClient`, `PromptBuilder`, and `ResponseParser` so that retry attempts, exponential-backoff timing, English prompt construction, and JSON-schema parsing are identical to the hosted providers.

#### Scenario: Transient failure is retried via the shared retry trait
- **GIVEN** `ai.retry_attempts = 3` and a wiremock that fails with HTTP 503 once and then succeeds
- **WHEN** `LocalLLMClient::analyze_content` is invoked
- **THEN** the client SHALL issue exactly two HTTP requests through the shared `HttpRetryClient` retry loop and SHALL return the successful `MatchResult`

#### Scenario: Cache key isolates local entries from hosted entries
- **GIVEN** the AI cache contains an entry produced by `OpenAIClient` for model `gpt-4.1-mini`
- **WHEN** `LocalLLMClient` produces a response for a local model identifier `llama3.1:8b-instruct` for the same prompt
- **THEN** the local result SHALL be stored and retrieved under a key that does not collide with the OpenAI entry
