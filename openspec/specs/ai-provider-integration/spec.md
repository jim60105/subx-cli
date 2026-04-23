# AI Provider Integration

## Purpose

Abstract subtitle-matching AI calls behind a provider trait so `openai`, `openrouter`, and `azure-openai` back ends are interchangeable, with shared prompt construction, response parsing, and retry logic. Implemented in `src/services/ai/` (`mod.rs`, `openai.rs`, `openrouter.rs`, `azure_openai.rs`, `prompts.rs`, `retry.rs`) and wired up by `ComponentFactory`.

## Requirements

### Requirement: Provider Trait Abstraction

The system SHALL define an `AIProvider` trait exposing `analyze_content(AnalysisRequest) -> MatchResult` and `verify_match(VerificationRequest) -> ConfidenceScore`, and all matching code SHALL depend on the trait rather than on any concrete client.

#### Scenario: Dependency injection in the match engine
- **GIVEN** a `MatchEngine` is constructed via `MatchEngine::new(ai_client, match_config)`
- **WHEN** the engine analyzes files
- **THEN** the engine SHALL invoke `ai_client.analyze_content` through the `AIProvider` trait without referencing a concrete provider type

### Requirement: Multi-Provider Selection

The system SHALL resolve the active AI provider from `config.ai.provider` using `ComponentFactory::create_ai_provider`, supporting at minimum the providers implemented under `src/services/ai/`: `openai`, `openrouter`, and `azure-openai`.

#### Scenario: OpenAI provider selected
- **GIVEN** `ai.provider = "openai"` and a valid `ai.api_key`
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a boxed `OpenAIClient` configured with the user's API key, model, temperature, and token limits

#### Scenario: Unknown provider rejected
- **GIVEN** `ai.provider` is set to an unrecognized value
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a configuration error rather than panic

### Requirement: Shared Prompt and Response Schema

The system SHALL build analysis and verification prompts in English via the shared `build_analysis_prompt_base` / `build_verification_prompt_base` functions, and SHALL require providers to respond with JSON that deserializes into `MatchResult` / `ConfidenceScore`.

#### Scenario: Prompt contains stable contract
- **GIVEN** an `AnalysisRequest` with video file IDs and subtitle file IDs
- **WHEN** `build_analysis_prompt_base` runs
- **THEN** the generated prompt SHALL instruct the model to respond with a JSON object containing `matches[].video_file_id`, `matches[].subtitle_file_id`, `matches[].confidence`, `confidence`, and `reasoning`

#### Scenario: Unparseable response yields a typed error
- **GIVEN** a provider returns text that cannot be parsed as the expected JSON schema
- **WHEN** `parse_match_result_base` runs
- **THEN** it SHALL return `SubXError::AiService` with a message indicating `AI response parsing failed`

### Requirement: Retry with Exponential Backoff

The system SHALL retry transient AI service failures using exponential backoff with a configurable maximum attempt count, base delay, backoff multiplier, and maximum delay cap.

#### Scenario: Retry succeeds on second attempt
- **GIVEN** a `RetryConfig { max_attempts: 3, ... }` and an operation that fails once and then succeeds
- **WHEN** `retry_with_backoff` is invoked
- **THEN** the operation SHALL be called exactly twice and the final result SHALL be the successful value

#### Scenario: Attempts capped by max_attempts
- **GIVEN** an operation that always fails and `max_attempts = 2`
- **WHEN** `retry_with_backoff` is invoked
- **THEN** the operation SHALL be attempted exactly twice and the last error SHALL be returned

### Requirement: Shared HTTP Retry Abstraction

Provider clients (OpenAI, OpenRouter, Azure OpenAI) SHALL share HTTP retry, prompt-building, and response-parsing behavior through common traits (`HttpRetryClient`, `PromptBuilder`, `ResponseParser`) under `src/services/ai/` so that retry configuration, request cloning, and error semantics are consistent across providers.

#### Scenario: Retry applied uniformly across providers
- **GIVEN** any of the built-in providers and a `RetryConfig` with `max_attempts = 3`
- **WHEN** the provider issues a request that fails with a transient network error and later succeeds
- **THEN** the retry loop SHALL be driven by the shared `HttpRetryClient` trait and SHALL return the successful response regardless of which provider issued it

#### Scenario: Zero retry attempts performs a single call
- **GIVEN** `RetryConfig { max_attempts: 0, ... }`
- **WHEN** a provider dispatches a request through the shared retry client
- **THEN** the request SHALL be attempted exactly once and any error SHALL be surfaced directly to the caller

### Requirement: Azure OpenAI Endpoint and Deployment Configuration

When `ai.provider = "azure-openai"`, the system SHALL build request URLs of the form `{base_url}/openai/deployments/{model}/chat/completions?api-version={api_version}`, treating `ai.model` as the Azure deployment name. The system SHALL require a non-empty deployment name and a valid HTTP(S) endpoint, and SHALL default `ai.api_version` to `2025-04-01-preview` when unset. Implemented in `src/services/ai/azure_openai.rs`.

#### Scenario: Custom deployment and api-version
- **GIVEN** `ai.base_url = "https://example.openai.azure.com"`, `ai.model = "my-deployment"`, and `ai.api_version = Some("2024-02-15-preview")`
- **WHEN** `AzureOpenAIClient::from_config` constructs a client and issues a chat completion request
- **THEN** the outgoing request URL SHALL be `https://example.openai.azure.com/openai/deployments/my-deployment/chat/completions?api-version=2024-02-15-preview`

#### Scenario: Default api-version applied
- **GIVEN** `ai.api_version` is `None`
- **WHEN** the Azure client is constructed
- **THEN** the client SHALL use the default api-version `2025-04-01-preview`

#### Scenario: Missing deployment rejected
- **GIVEN** `ai.model` is empty or whitespace
- **WHEN** `AzureOpenAIClient::from_config` is called or configuration validation runs
- **THEN** either the client construction or `validator::validate_config` SHALL return a configuration error referencing the missing Azure OpenAI deployment name (empty or whitespace `ai.model`)

#### Scenario: Invalid endpoint rejected
- **GIVEN** `ai.base_url` is not a valid http(s) URL with a host
- **WHEN** `AzureOpenAIClient::from_config` is called
- **THEN** construction SHALL fail with a configuration error describing the invalid endpoint

### Requirement: Azure OpenAI Authentication Modes

The Azure OpenAI client SHALL support two authentication modes selected from the stored `ai.api_key` value: when the value begins (case-insensitively) with `bearer `, it SHALL be sent verbatim in an `Authorization` header; otherwise it SHALL be sent as the Azure `api-key` request header.

#### Scenario: api-key header authentication
- **GIVEN** `ai.api_key = Some("secret-key")`
- **WHEN** the client sends a request
- **THEN** the outgoing request SHALL include an `api-key: secret-key` header and SHALL NOT include an `Authorization` header

#### Scenario: Bearer token authentication
- **GIVEN** `ai.api_key = Some("Bearer eyJhbGciOi...")`
- **WHEN** the client sends a request
- **THEN** the outgoing request SHALL include an `Authorization: Bearer eyJhbGciOi...` header and SHALL NOT include an `api-key` header

### Requirement: Provider Environment Variable Overrides

The configuration loader SHALL recognize provider-specific environment variables and, when present, set `ai.provider`, `ai.api_key`, `ai.base_url`, `ai.api_version`, and `ai.model` accordingly, with the following effective precedence for `ai.api_key`: `AZURE_OPENAI_API_KEY` SHALL unconditionally overwrite any prior value; `OPENROUTER_API_KEY` SHALL overwrite any prior value below Azure; and `OPENAI_API_KEY` SHALL only populate `ai.api_key` when neither `OPENROUTER_API_KEY` nor `SUBX_AI_APIKEY` has already set it. For `ai.base_url`, `AZURE_OPENAI_ENDPOINT` SHALL be applied after `OPENAI_BASE_URL`, so Azure takes precedence for the base URL as well. Implemented in `src/config/service.rs:256-296`.

#### Scenario: OpenAI key from environment
- **GIVEN** no configured `ai.api_key` and `OPENAI_API_KEY=sk-test`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.api_key` SHALL equal `Some("sk-test")`

#### Scenario: OpenRouter selection from environment
- **GIVEN** `OPENROUTER_API_KEY=or-test`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.provider` SHALL equal `"openrouter"` and `config.ai.api_key` SHALL equal `Some("or-test")`

#### Scenario: Azure OpenAI full environment loading
- **GIVEN** `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_DEPLOYMENT_ID`, and `AZURE_OPENAI_API_VERSION` are all set
- **WHEN** the configuration is loaded
- **THEN** `ai.provider` SHALL be `"azure-openai"`, `ai.api_key` SHALL match the Azure key, `ai.base_url` SHALL match the endpoint, `ai.model` SHALL match the deployment id, and `ai.api_version` SHALL match the api-version

#### Scenario: Azure endpoint overrides OpenAI base URL
- **GIVEN** both `OPENAI_BASE_URL=https://api.openai.com/v1` and `AZURE_OPENAI_ENDPOINT=https://example.openai.azure.com` are set
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL equal `"https://example.openai.azure.com"`

### Requirement: AI Request Timeout Configuration

The system SHALL expose `ai.request_timeout_seconds` (default 120) as a validated integer in the inclusive range [10, 600]; the value SHALL govern the HTTP client request timeout used by all AI providers.

#### Scenario: Default timeout value
- **GIVEN** a default `Config`
- **WHEN** `Config::default()` is inspected
- **THEN** `config.ai.request_timeout_seconds` SHALL equal 120

#### Scenario: Out-of-range timeout rejected
- **GIVEN** the user runs `subx config set ai.request_timeout_seconds 5` or `subx config set ai.request_timeout_seconds 700`
- **WHEN** the field validator runs
- **THEN** the command SHALL fail with an error indicating the value is out of range `[10, 600]` and the persisted configuration SHALL remain unchanged

#### Scenario: Timeout applied to provider HTTP clients
- **GIVEN** `ai.request_timeout_seconds = N` in the loaded configuration
- **WHEN** an OpenAI, OpenRouter, or Azure OpenAI client is constructed from that configuration
- **THEN** the underlying `reqwest::Client` SHALL be built with a request timeout of `N` seconds (see `src/services/ai/openai.rs`, `openrouter.rs`, `azure_openai.rs` timeout wiring)

### Requirement: Content Preview Length Cap

When the match engine assembles `ContentSample.content_preview` entries for the AI request, the system SHALL take at most the first 20 lines of each subtitle file and SHALL further truncate the resulting preview to at most `ai.max_sample_length` characters (appending `...` on truncation).

#### Scenario: Preview truncated at max_sample_length
- **GIVEN** `ai.max_sample_length = 100` and a subtitle file whose first 20 lines exceed 100 characters
- **WHEN** `MatchEngine::extract_content_samples` runs
- **THEN** the produced `ContentSample.content_preview` SHALL have length `100 + 3` (including the `...` suffix) and SHALL begin with the first 100 characters of the first-20-line preview

