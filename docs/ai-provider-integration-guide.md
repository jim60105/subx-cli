# AI Provider Integration Guide

Adding a new AI provider to SubX-CLI spans five layers: the provider client
itself, configuration and validation, factory registration, tests, and
documentation. This guide walks through each step, using the OpenRouter
integration as the reference implementation.

Throughout this guide, placeholders follow a consistent convention:

- `{Provider}` — Rust type name, e.g., `OpenRouter`
- `{provider}` — module name and config value, e.g., `openrouter`
- `{PROVIDER}` — environment variable prefix, e.g., `OPENROUTER`

## Prerequisites

You need familiarity with Rust async programming and the SubX-CLI codebase
structure. Have the target provider's API documentation and an API key ready
before starting.

## Current Provider Landscape

The configuration layer accepts five provider names: `openai`, `openrouter`,
`azure-openai`, `anthropic`, and `local`
(see `src/config/field_validator.rs:30-35`). However, only three have
runtime implementations with client code and factory support: **openai**,
**openrouter**, and **azure-openai**
(see `src/core/factory.rs:189-210`). When adding a new provider, you are
building both the config acceptance path and the runtime implementation.

## Step-by-Step Integration

### Step 1. Create the Provider Client

**File:** `src/services/ai/{provider}.rs` (new)

Create a module that implements the `AIProvider` trait. The trait lives in
`src/services/ai/mod.rs` and requires two async methods:
`analyze_content` and `verify_match`.

```rust
use crate::Result;
use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::AiUsageStats;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use crate::services::ai::prompts::{PromptBuilder, ResponseParser};
use crate::services::ai::retry::HttpRetryClient;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;

/// {Provider} client implementation.
#[derive(Debug)]
pub struct {Provider}Client {
    client: Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    base_url: String,
    request_timeout_seconds: u64,
}

impl {Provider}Client {
    /// Create a new client from the AI configuration section.
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        // Validate inputs, build reqwest::Client with timeout, return Self
    }

    /// Validate that the base URL is well-formed.
    fn validate_base_url(url: &str) -> crate::Result<()> {
        // URL validation logic
    }

    /// Send a chat completion request to the provider API.
    async fn chat_completion(&self, messages: Vec<Value>) -> Result<String> {
        // Build request, call make_request_with_retry, parse response,
        // emit usage stats via display_ai_usage
    }
}

#[async_trait]
impl AIProvider for {Provider}Client {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        // Build prompt via PromptBuilder, call chat_completion,
        // parse response via ResponseParser
    }

    async fn verify_match(&self, request: VerificationRequest) -> Result<ConfidenceScore> {
        // Similar flow for match verification
    }
}
```

The existing providers (`src/services/ai/openai.rs`,
`src/services/ai/openrouter.rs`) serve as concrete references. Each one
uses `PromptBuilder` and `ResponseParser` from `src/services/ai/prompts.rs`
to construct and parse AI messages, and `HttpRetryClient` from
`src/services/ai/retry.rs` for exponential backoff. Follow the same
pattern — reuse the shared prompt and retry infrastructure rather than
reimplementing it.

Every provider must call `display_ai_usage` (from `src/cli/`) after
receiving an API response. Parse token counts from the response and build
an `AiUsageStats` struct to pass to this function.

### Step 2. Register the Module

**File:** `src/services/ai/mod.rs`

Add the module declaration:

```rust
/// {Provider} AI service provider client implementation.
pub mod {provider};
```

### Step 3. Add to Field Validation

**File:** `src/config/field_validator.rs`

Add the new provider slug to the `validate_enum` call at line 32:

```rust
"ai.provider" => {
    validate_non_empty_string(value, "AI provider")?;
    validate_enum(
        value,
        &["openai", "anthropic", "local", "openrouter", "azure-openai", "{provider}"],
    )?;
}
```

Add a test case confirming the new value is accepted:

```rust
assert!(validate_field("ai.provider", "{provider}").is_ok());
```

### Step 4. Add Provider-Specific Validation

**File:** `src/config/validator.rs`

Add a match arm in `validate_ai_config` for the new provider. Each provider
validates its own required fields — at minimum: non-empty API key, valid
model name, temperature in range, positive max_tokens, and valid base URL.

```rust
"{provider}" => {
    if let Some(api_key) = &ai_config.api_key {
        if !api_key.is_empty() {
            validate_api_key(api_key)?;
        }
    }
    validate_ai_model(&ai_config.model)?;
    validate_temperature(ai_config.temperature)?;
    validate_positive_number(ai_config.max_tokens as f64)?;

    if !ai_config.base_url.is_empty() {
        validate_url_format(&ai_config.base_url)?;
    }
}
```

Update the fallthrough error message to list all supported providers. Add a
test that constructs an `AIConfig` with the new provider and verifies
`validate_ai_config` succeeds.

### Step 5. Handle Environment Variables

**File:** `src/config/service.rs`

The config service has a dedicated block for each provider's environment
variables (around line 255–297). Add a similar block for the new provider:

```rust
if let Some(api_key) = self.env_provider.get_var("{PROVIDER}_API_KEY") {
    debug!("ProductionConfigService: Found {PROVIDER}_API_KEY environment variable");
    app_config.ai.provider = "{provider}".to_string();
    app_config.ai.api_key = Some(api_key);
}
```

When the env var is set, the service automatically switches the active
provider and injects the key. If your provider needs additional env vars
(like a custom endpoint), add those as well.

Write a test using `TestEnvironmentProvider` to verify the env var is loaded
correctly:

```rust
#[test]
fn test_production_config_service_{provider}_api_key_loading() {
    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("{PROVIDER}_API_KEY", "test-api-key");
    env_provider.set_var("SUBX_CONFIG_PATH", "/tmp/test_config_{provider}.toml");

    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");
    let config = service.get_config().expect("Failed to get config");

    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### Step 6. Add Test Configuration Support

**File:** `src/config/test_service.rs`

Add a test verifying that `TestConfigService::with_ai_settings_and_key`
works with the new provider:

```rust
#[test]
fn test_config_service_with_ai_settings_and_key_{provider}() {
    let service = TestConfigService::with_ai_settings_and_key(
        "{provider}", "provider-model", "test-api-key",
    );
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.provider, "{provider}");
    assert_eq!(config.ai.model, "provider-model");
    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### Step 7. Add Builder Tests

**File:** `src/config/builder.rs`

Add a test confirming the `TestConfigBuilder` fluent API produces a valid
config for the new provider:

```rust
#[test]
fn test_builder_ai_configuration_{provider}() {
    let config = TestConfigBuilder::new()
        .with_ai_provider("{provider}")
        .with_ai_model("provider-model")
        .with_ai_api_key("test-api-key")
        .build_config();
    assert_eq!(config.ai.provider, "{provider}");
    assert_eq!(config.ai.model, "provider-model");
    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### Step 8. Register in the Factory

**File:** `src/core/factory.rs`

Import the new client and add a match arm in `create_ai_provider`:

```rust
use crate::services::ai::{provider}::{Provider}Client;

// Inside create_ai_provider match:
"{provider}" => {
    validate_ai_config(ai_config)?;
    let client = {Provider}Client::from_config(ai_config)?;
    Ok(Box::new(client))
}
```

Update the fallthrough error message to include the new provider. Add a
factory test:

```rust
#[test]
fn test_create_ai_provider_{provider}_success() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key(
        "{provider}", "provider-model", "test-api-key",
    );
    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();
    assert!(result.is_ok());
}
```

### Step 9. Update CLI Documentation

**File:** `src/cli/config_args.rs`

Add the new provider to the rustdoc examples in the config args module so
that `subx-cli config set ai.provider {provider}` appears in the help text.

### Step 10. Update Validation Tests

**File:** `src/config/validation.rs`

Add the new provider to any `validate_enum` test arrays that list accepted
providers.

### Step 11. Create Integration Tests

**File:** `tests/{provider}_integration_tests.rs` (new)

Write integration tests covering both success and failure paths:

```rust
use subx_cli::config::TestConfigService;
use subx_cli::core::ComponentFactory;

#[tokio::test]
async fn test_{provider}_client_creation() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key(
        "{provider}", "provider-model", "test-key",
    );
    let factory = ComponentFactory::new(&config_service).unwrap();
    assert!(factory.create_ai_provider().is_ok());
}

#[tokio::test]
async fn test_{provider}_empty_api_key_rejected() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key("{provider}", "provider-model", "");
    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();
    assert!(result.is_err());
}
```

### Step 12. Update Documentation

Three documentation files need updates:

- **`docs/configuration-guide.md`** — Add a TOML configuration example for
  the new provider, including all relevant fields.
- **`README.md`** — Add quick-start setup instructions (env var export +
  `config set` commands).
- **`README.zh-TW.md`** — Add the same instructions in Traditional Chinese.

## Azure OpenAI: Provider-Specific Deviations

Azure OpenAI diverges from the generic pattern in several ways. When
implementing a provider that has similar platform-specific behavior, use
Azure as a reference (see `src/services/ai/azure_openai.rs`).

The `ai.model` field serves as the Azure deployment name rather than a model
identifier. The `ai.api_version` field (an `Option<String>` on `AIConfig`)
is required, with a default of `2025-04-01-preview`. The API endpoint URL
format differs from standard OpenAI.

Azure loads five environment variables in `src/config/service.rs`:
`AZURE_OPENAI_API_KEY`, `AZURE_OPENAI_ENDPOINT`,
`AZURE_OPENAI_API_VERSION`, and `AZURE_OPENAI_DEPLOYMENT_ID` (which
overrides `ai.model`). If your provider has similar multi-field
configuration, follow the same pattern of dedicated env var blocks.

## File Change Summary

Every integration touches files across multiple layers. Use this table as a
checklist before opening a pull request.

| Layer | File | Change |
|---|---|---|
| **Client** | `src/services/ai/{provider}.rs` | New: provider implementation |
| **Client** | `src/services/ai/mod.rs` | Add module declaration |
| **Factory** | `src/core/factory.rs` | Add match arm + import |
| **Validation** | `src/config/field_validator.rs` | Add to enum list |
| **Validation** | `src/config/validator.rs` | Add validation block |
| **Config** | `src/config/service.rs` | Add env var handling |
| **Config** | `src/config/test_service.rs` | Add provider test |
| **Config** | `src/config/builder.rs` | Add builder test |
| **Config** | `src/config/validation.rs` | Update enum test arrays |
| **CLI** | `src/cli/config_args.rs` | Add doc examples |
| **Tests** | `tests/{provider}_integration_tests.rs` | New: integration tests |
| **Docs** | `docs/configuration-guide.md` | Add config section |
| **Docs** | `README.md` | Add setup instructions |
| **Docs** | `README.zh-TW.md` | Add setup instructions (Chinese) |

## Testing

A complete provider integration requires tests at three levels:

**Unit tests** live inside the provider module and the configuration files.
They cover config validation, client construction, and error handling for
invalid inputs.

**Factory tests** in `src/core/factory.rs` verify that the factory
dispatches to the correct client and rejects invalid configurations.

**Integration tests** in `tests/{provider}_integration_tests.rs` exercise
the full path from config to client creation. For tests that call the actual
API, use wiremock mocks (see `tests/common/mock_openai_helper.rs` for the
pattern).

Run the following commands to validate:

```bash
# All tests
cargo nextest run || true

# Provider-specific integration tests
cargo nextest run --test {provider}_integration_tests

# Configuration-related tests
cargo nextest run config

# Full quality check
scripts/quality_check.sh
```

## Best Practices

Validate API keys on client construction, not at request time. An empty key
should produce a clear error immediately. If the provider uses a
recognizable key format (like OpenAI's `sk-` prefix), add format validation
as well, but keep it lenient — providers change key formats over time.

Reuse the shared `HttpRetryClient` from `src/services/ai/retry.rs` for
retry logic. It implements exponential backoff with configurable attempts
and delay. Timeout values come from `AIConfig.request_timeout_seconds`.
Handle timeout and network errors by returning `SubXError::AiService` with
a descriptive message.

Call `display_ai_usage` after every successful API response. Parse the token
counts (prompt tokens, completion tokens, total tokens) from the provider's
response format and pass them as `AiUsageStats`.

Never log or print API keys. The config system handles keys through
`Option<String>` fields and environment variables. Follow the same pattern
for any additional secrets your provider requires.

Test both the success path and the failure path. At minimum, verify that a
valid config produces a working client, an empty API key is rejected, and
an invalid base URL is rejected. If the provider has unique error responses,
add tests for those as well.
