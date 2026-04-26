## ADDED Requirements

### Requirement: AI Provider Identifier Canonicalization

The configuration system SHALL provide a single canonicalization function `normalize_ai_provider(value: &str) -> String` (located in `src/config/field_validator.rs`) that lowercases and trims its input and maps the alias `"ollama"` to the canonical identifier `"local"`. All other recognized providers (`openai`, `openrouter`, `azure-openai`, `local`) SHALL pass through unchanged. Unknown values SHALL be returned unchanged so downstream allow-list validation still rejects them with the existing error.

This function SHALL be the **only** place where the `ollama -> local` alias is resolved. Every component that reads or writes `ai.provider` SHALL invoke `normalize_ai_provider` before using the value, including:

1. `subx config set ai.provider <value>` (field validator) — the persisted on-disk value SHALL be the canonical form.
2. `subx config get ai.provider` (field validator) returns the canonical form.
3. `ProductionConfigService` env-var loading — `SUBX_AI_PROVIDER=ollama` SHALL be accepted and normalized to `"local"` before any precedence or scoping decision (including the hosted-provider env-var carve-out) is made.
4. `validate_ai_config` in `src/config/validator.rs` — validation arms key off the canonicalized value.
5. `ComponentFactory::create_ai_provider` in `src/core/factory.rs` — the dispatch match arm uses the canonicalized value, so the factory only ever sees `"local"` (never `"ollama"`).

#### Scenario: `ollama` is normalized when set via CLI
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs and the configuration is persisted
- **THEN** the persisted `ai.provider` value SHALL be `"local"` and a subsequent `subx config get ai.provider` SHALL return `"local"`

#### Scenario: `SUBX_AI_PROVIDER=ollama` is normalized
- **GIVEN** the configuration file has no `ai.provider` override and `SUBX_AI_PROVIDER=ollama` is set in the environment
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `config.ai.provider` SHALL equal `"local"` and the hosted-provider env-var carve-out SHALL apply as if the user had set `ai.provider = "local"` directly

#### Scenario: Canonical values pass through unchanged
- **GIVEN** any input in `{"openai", "openrouter", "azure-openai", "local"}`
- **WHEN** `normalize_ai_provider` is invoked
- **THEN** the returned string SHALL equal the input

### Requirement: Local Provider Validation Rules

When the canonicalized `ai.provider` (after `normalize_ai_provider`) equals `"local"`, `validate_ai_config` SHALL apply a dedicated validation arm in `src/config/validator.rs` that:
- Treats `ai.api_key` as optional: a missing or empty value SHALL be accepted; a non-empty value SHALL be validated through the same `validate_api_key` helper used by other providers (no provider-specific prefix is required).
- Requires `ai.base_url` to be a non-empty string and SHALL run it through `validate_url_format`.
- Validates `ai.model` (non-empty), `ai.temperature`, and `ai.max_tokens` using the same helpers as the hosted providers.

`field_validator.rs` SHALL list both `local` and `ollama` in the allow-list for the `ai.provider` key (so that `subx config set` accepts either), and SHALL document that `ai.api_key` is optional and `ai.base_url` is required when the canonicalized provider is `local`. The persisted value after `subx config set` SHALL always be the canonical form produced by `normalize_ai_provider`.

#### Scenario: Local provider config without API key validates
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://localhost:11434/v1"`, `ai.model = "llama3.1:8b-instruct"`, and `ai.api_key = None`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

#### Scenario: Local provider rejects missing base URL
- **GIVEN** `ai.provider = "local"`, `ai.base_url = ""`, and `ai.model = "llama3.1"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return a configuration error whose message indicates that `ai.base_url` is required when `ai.provider` is `local`

#### Scenario: `ollama` alias is normalized
- **GIVEN** the user runs `subx config set ai.provider ollama`
- **WHEN** the field validator runs
- **THEN** the persisted `ai.provider` value SHALL be `"local"` (produced by `normalize_ai_provider`)

### Requirement: Local Provider Environment Variables

`ProductionConfigService` SHALL recognize the environment variables `LOCAL_LLM_BASE_URL` (mapping to `ai.base_url`) and `LOCAL_LLM_API_KEY` (mapping to `ai.api_key`), and SHALL apply them only when the canonicalized `ai.provider` (after `normalize_ai_provider` is applied to the resolved `SUBX_AI_PROVIDER` and config-file value) is `"local"`. These overrides SHALL apply with lower precedence than `SUBX_AI_BASE_URL` and `SUBX_AI_APIKEY` so that the unified `SUBX_*` namespace remains authoritative.

#### Scenario: `LOCAL_LLM_BASE_URL` honored when provider is local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `LOCAL_LLM_BASE_URL=http://localhost:8080/v1` set
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:8080/v1"`

#### Scenario: `LOCAL_LLM_*` ignored for non-local providers
- **GIVEN** the configuration file sets `ai.provider = "openai"` and the environment has `LOCAL_LLM_BASE_URL=http://localhost:11434/v1` and `LOCAL_LLM_API_KEY=secret` set
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL NOT be populated from `LOCAL_LLM_BASE_URL` and `config.ai.api_key` SHALL NOT be populated from `LOCAL_LLM_API_KEY`

#### Scenario: `SUBX_AI_BASE_URL` outranks `LOCAL_LLM_BASE_URL`
- **GIVEN** `ai.provider = "local"`, `LOCAL_LLM_BASE_URL=http://localhost:11434/v1`, and `SUBX_AI_BASE_URL=http://localhost:8080/v1`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.base_url` SHALL equal `"http://localhost:8080/v1"`

## MODIFIED Requirements

### Requirement: Compatibility Environment Variables For Third-Party Providers

In addition to `SUBX_AI_*` overrides, `ProductionConfigService` SHALL recognize industry-standard environment variables for each supported provider and apply them on top of the loaded configuration: `OPENAI_API_KEY` (sets `ai.api_key` when no key is already configured), `OPENAI_BASE_URL` (sets `ai.base_url`), `OPENROUTER_API_KEY` (sets `ai.api_key` and switches `ai.provider` to `openrouter`), and `AZURE_OPENAI_API_KEY` / `AZURE_OPENAI_ENDPOINT` / `AZURE_OPENAI_API_VERSION` / `AZURE_OPENAI_DEPLOYMENT_ID` (switch `ai.provider` to `azure-openai` and populate the Azure fields). Implemented in `src/config/service.rs`.

**Local-provider carve-out:** when the canonicalized `ai.provider` (after `normalize_ai_provider` has been applied to the resolved `SUBX_AI_PROVIDER` value or the configuration-file value) is `"local"`, `ProductionConfigService` SHALL skip the entire hosted-provider env-var application path. It SHALL NOT switch `ai.provider` away from `"local"`, SHALL NOT populate `ai.api_key`, `ai.base_url`, `ai.api_version`, or `ai.model` from any of `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, `AZURE_OPENAI_API_VERSION`, or `AZURE_OPENAI_DEPLOYMENT_ID`. This preserves the user's explicit privacy choice when they have selected a local provider.

#### Scenario: `OPENROUTER_API_KEY` switches provider
- **GIVEN** configuration file leaves `ai.provider` at its default and `OPENROUTER_API_KEY=sk-or-...`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `Config.ai.provider` SHALL equal `"openrouter"` and `Config.ai.api_key` SHALL equal `Some("sk-or-...")`

#### Scenario: `AZURE_OPENAI_*` variables populate Azure fields
- **GIVEN** `AZURE_OPENAI_API_KEY=k`, `AZURE_OPENAI_ENDPOINT=https://x.openai.azure.com/`, `AZURE_OPENAI_API_VERSION=2024-02-15-preview`, and `AZURE_OPENAI_DEPLOYMENT_ID=gpt-4o`
- **WHEN** configuration is loaded
- **THEN** `Config.ai.provider` SHALL equal `"azure-openai"`, `Config.ai.base_url` SHALL equal the endpoint, `Config.ai.api_version` SHALL equal `Some("2024-02-15-preview")`, and `Config.ai.model` SHALL equal `"gpt-4o"`

#### Scenario: `OPENAI_API_KEY` is backward-compatible fallback
- **GIVEN** the configuration file has no `ai.api_key` and `OPENAI_API_KEY=sk-...` is set
- **WHEN** configuration is loaded
- **THEN** `Config.ai.api_key` SHALL equal `Some("sk-...")`

#### Scenario: `OPENAI_API_KEY` does not leak into local provider
- **GIVEN** the configuration file sets `ai.provider = "local"`, `ai.api_key = None`, and the environment has `OPENAI_API_KEY=sk-leak`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** `config.ai.provider` SHALL equal `"local"` and `config.ai.api_key` SHALL equal `None`

#### Scenario: `OPENROUTER_API_KEY` does not switch provider away from local
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `OPENROUTER_API_KEY=or-test`
- **WHEN** the configuration is loaded
- **THEN** `config.ai.provider` SHALL equal `"local"` and SHALL NOT be switched to `"openrouter"`

#### Scenario: `AZURE_OPENAI_*` variables ignored for local provider
- **GIVEN** the configuration file sets `ai.provider = "local"` and the environment has `AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`, and `AZURE_OPENAI_DEPLOYMENT_ID` all set
- **WHEN** the configuration is loaded
- **THEN** `config.ai.provider` SHALL equal `"local"`, `config.ai.base_url` SHALL NOT be populated from `AZURE_OPENAI_ENDPOINT`, and `config.ai.model` SHALL NOT be populated from `AZURE_OPENAI_DEPLOYMENT_ID`

#### Scenario: `SUBX_AI_PROVIDER=ollama` triggers the local carve-out
- **GIVEN** the configuration file sets `ai.provider = "openai"`, the environment has `SUBX_AI_PROVIDER=ollama`, and the environment also has `OPENAI_API_KEY=sk-leak` and `OPENROUTER_API_KEY=or-leak`
- **WHEN** `ProductionConfigService` loads configuration
- **THEN** the resolved `config.ai.provider` SHALL equal `"local"` (after `normalize_ai_provider`) and neither `OPENAI_API_KEY` nor `OPENROUTER_API_KEY` SHALL populate `ai.api_key` or change the provider
