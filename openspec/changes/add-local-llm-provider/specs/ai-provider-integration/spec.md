## MODIFIED Requirements

### Requirement: Multi-Provider Selection

The system SHALL resolve the active AI provider from `config.ai.provider` using `ComponentFactory::create_ai_provider`, supporting at minimum the providers implemented under `src/services/ai/`: `openai`, `openrouter`, `azure-openai`, and `local`. The identifier `ollama` SHALL be accepted as an alias for `local` and normalized to `local` at validation time.

#### Scenario: OpenAI provider selected
- **GIVEN** `ai.provider = "openai"` and a valid `ai.api_key`
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a boxed `OpenAIClient` configured with the user's API key, model, temperature, and token limits

#### Scenario: Local provider selected
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, and `ai.model = "llama3.1:8b-instruct"`
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a boxed `LocalLLMClient` (from `src/services/ai/local.rs`) configured with the supplied `base_url`, `model`, `temperature`, `max_tokens`, retry settings, and request timeout, regardless of whether `ai.api_key` is set

#### Scenario: Unknown provider rejected
- **GIVEN** `ai.provider` is set to a value that is not in `{openai, openrouter, azure-openai, local}` (or the `ollama` alias)
- **WHEN** `ComponentFactory::create_ai_provider` is called
- **THEN** it SHALL return a configuration error rather than panic, and the error message SHALL list `openai`, `openrouter`, `azure-openai`, and `local` as supported providers

### Requirement: Provider Environment Variable Overrides

The configuration loader SHALL recognize provider-specific environment variables and, when present, set `ai.provider`, `ai.api_key`, `ai.base_url`, `ai.api_version`, and `ai.model` accordingly, with the following effective precedence for `ai.api_key`: `AZURE_OPENAI_API_KEY` SHALL unconditionally overwrite any prior value; `OPENROUTER_API_KEY` SHALL overwrite any prior value below Azure; and `OPENAI_API_KEY` SHALL only populate `ai.api_key` when neither `OPENROUTER_API_KEY` nor `SUBX_AI_APIKEY` has already set it. For `ai.base_url`, `AZURE_OPENAI_ENDPOINT` SHALL be applied after `OPENAI_BASE_URL`, so Azure takes precedence for the base URL as well. **Exception:** when the resolved `ai.provider` (after applying `SUBX_AI_PROVIDER` and the configuration file, and after running the value through `normalize_ai_provider`) is `local`, the loader SHALL skip all hosted-provider compatibility variables (`OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_VERSION`, `AZURE_OPENAI_DEPLOYMENT_ID`) so they do not switch the provider away from `local` or populate any `ai.*` field. Implemented in `src/config/service.rs`.

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

#### Scenario: Hosted compatibility env vars are inert when provider is local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the process environment has `OPENAI_API_KEY=sk-leak`, `OPENROUTER_API_KEY=or-leak`, `AZURE_OPENAI_API_KEY=az-leak`, `AZURE_OPENAI_ENDPOINT=https://x.openai.azure.com`, and `AZURE_OPENAI_DEPLOYMENT_ID=gpt-4o`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `ai.provider` SHALL remain `"local"`, `ai.api_key` SHALL NOT be populated from any of those variables, `ai.base_url` SHALL NOT be populated from `AZURE_OPENAI_ENDPOINT`, and `ai.model` SHALL NOT be populated from `AZURE_OPENAI_DEPLOYMENT_ID`

### Requirement: Shared HTTP Retry Abstraction

Provider clients (OpenAI, OpenRouter, Azure OpenAI, Local LLM) SHALL share HTTP retry, prompt-building, and response-parsing behavior through common traits (`HttpRetryClient`, `PromptBuilder`, `ResponseParser`) under `src/services/ai/` so that retry configuration, request cloning, and error semantics are consistent across providers.

#### Scenario: Retry applied uniformly across providers
- **GIVEN** any of the built-in providers (`openai`, `openrouter`, `azure-openai`, `local`) and a `RetryConfig` with `max_attempts = 3`
- **WHEN** the provider issues a request that fails with a transient network error and later succeeds
- **THEN** the retry loop SHALL be driven by the shared `HttpRetryClient` trait and SHALL return the successful response regardless of which provider issued it

#### Scenario: Zero retry attempts performs a single call
- **GIVEN** `RetryConfig { max_attempts: 0, ... }`
- **WHEN** a provider dispatches a request through the shared retry client
- **THEN** the request SHALL be attempted exactly once and any error SHALL be surfaced directly to the caller
