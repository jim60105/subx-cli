## MODIFIED Requirements

### Requirement: Local Provider Validation Rules

When the canonicalized `ai.provider` (after `normalize_ai_provider`) equals `"local"`, `validate_ai_config` SHALL apply a dedicated validation arm in `src/config/validator.rs` that:
- Treats `ai.api_key` as optional: a missing or empty value SHALL be accepted; a non-empty value SHALL be validated through the same `validate_api_key` helper used by other providers (no provider-specific prefix is required).
- Requires `ai.base_url` to be a non-empty string and SHALL run it through `validate_url_format`.
- Validates `ai.model` (non-empty), `ai.temperature`, and `ai.max_tokens` using the same helpers as the hosted providers.
- Accepts BOTH `http://` and `https://` schemes for `ai.base_url`. The `local` provider is endpoint-agnostic and may target any reachable host (loopback, LAN, VPN, public). The HTTPS-required rule documented for hosted providers in the `ai-provider-integration` capability **in `subx-core`** SHALL NOT apply to `local`.

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

#### Scenario: Local provider accepts HTTP base URL
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "http://192.168.1.50:11434/v1"`, and `ai.model = "llama3.1"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

#### Scenario: Local provider accepts HTTPS base URL on a non-loopback host
- **GIVEN** `ai.provider = "local"`, `ai.base_url = "https://ollama.tailnet.ts.net/v1"`, and `ai.model = "qwen2.5:7b"`
- **WHEN** `validate_ai_config` runs
- **THEN** it SHALL return `Ok(())`

#### Scenario: The cross-repository reference resolves
- **GIVEN** a reader of this requirement holding only a `subx-cli` checkout
- **WHEN** they follow the reference to the HTTPS-required rule
- **THEN** the reference SHALL name `subx-core` as the repository holding `ai-provider-integration`, so that the absence of that capability from `subx-cli/openspec/specs/` reads as expected rather than as a missing file
