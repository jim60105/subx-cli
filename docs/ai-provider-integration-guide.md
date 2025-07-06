# AI Provider Integration Guide

This guide provides a comprehensive walkthrough for adding a new AI provider to the SubX CLI tool. This document is based on the complete integration of OpenRouter as an AI provider and covers all necessary steps, files, and considerations.

## Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Step-by-Step Integration Process](#step-by-step-integration-process)
4. [File Structure and Changes](#file-structure-and-changes)
5. [Testing Requirements](#testing-requirements)
6. [Configuration Examples](#configuration-examples)
7. [Common Pitfalls and Best Practices](#common-pitfalls-and-best-practices)

## Overview

Adding a new AI provider to SubX CLI requires changes across multiple layers of the application:

- **Core Services**: Implementing the AI provider client
- **Configuration**: Adding validation and support for the new provider
- **Factory Pattern**: Registering the provider in the factory
- **CLI Interface**: Updating help text and examples
- **Documentation**: Adding configuration guides and examples
- **Testing**: Comprehensive test coverage

## Prerequisites

- Understanding of Rust async programming
- Familiarity with the SubX CLI codebase structure
- Access to the target AI provider's API documentation
- API key or authentication method for the provider

## Step-by-Step Integration Process

### 1. Create the AI Provider Client

**File**: `src/services/ai/{provider_name}.rs`

Create a new Rust module implementing the AI provider client:

```rust
use crate::Result;
use crate::cli::display_ai_usage;
use crate::error::SubXError;
use crate::services::ai::AiUsageStats;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};
use async_trait::async_trait;

/// [Provider] client implementation
#[derive(Debug)]
pub struct [Provider]Client {
    client: reqwest::Client,
    api_key: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    retry_attempts: u32,
    retry_delay_ms: u64,
    base_url: String,
}

impl [Provider]Client {
    /// Create new client with configuration
    pub fn from_config(config: &crate::config::AIConfig) -> crate::Result<Self> {
        // Implementation details
    }

    /// Validate base URL format
    fn validate_base_url(url: &str) -> crate::Result<()> {
        // URL validation logic
    }

    async fn chat_completion(&self, messages: Vec<Value>) -> Result<String> {
        // API communication logic
    }

    async fn make_request_with_retry(&self, request: reqwest::RequestBuilder) -> reqwest::Result<reqwest::Response> {
        // Retry logic implementation
    }
}

#[async_trait]
impl AIProvider for [Provider]Client {
    async fn analyze_content(&self, request: AnalysisRequest) -> Result<MatchResult> {
        // Implementation
    }

    async fn verify_match(&self, verification: VerificationRequest) -> Result<ConfidenceScore> {
        // Implementation
    }
}
```

**Key Implementation Notes**:
- Implement the `AIProvider` trait with `analyze_content` and `verify_match` methods
- Include proper error handling and retry logic
- Add usage statistics tracking with `display_ai_usage`
- Support configurable timeouts and retry attempts
- Validate API keys and base URLs

### 2. Update Service Module Declaration

**File**: `src/services/ai/mod.rs`

Add the new provider module:

```rust
/// [Provider] AI service provider client implementation
pub mod [provider_name];
```

### 3. Update Configuration Validation

**File**: `src/config/field_validator.rs`

Add the new provider to the allowed providers list:

```rust
"ai.provider" => {
    validate_non_empty_string(value, "AI provider")?;
    validate_enum(value, &["openai", "anthropic", "local", "[provider_name]"])?;
}
```

Add validation tests:

```rust
#[test]
fn test_validate_ai_fields() {
    // Valid cases
    assert!(validate_field("ai.provider", "[provider_name]").is_ok());
    // Additional test cases...
}
```

### 4. Update Configuration Validator

**File**: `src/config/validator.rs`

Add provider-specific validation logic:

```rust
"[provider_name]" => {
    if let Some(api_key) = &ai_config.api_key {
        if !api_key.is_empty() {
            validate_api_key(api_key)?;
            // Provider-specific API key validation if needed
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

Update error messages to include the new provider:

```rust
_ => {
    return Err(SubXError::config(format!(
        "Unsupported AI provider: {}. Supported providers: openai, [provider_name], anthropic",
        ai_config.provider
    )));
}
```

Add tests for the new provider:

```rust
#[test]
fn test_validate_ai_config_[provider_name]() {
    let mut ai_config = AIConfig::default();
    ai_config.provider = "[provider_name]".to_string();
    ai_config.api_key = Some("test-api-key".to_string());
    ai_config.model = "provider-model".to_string();
    assert!(validate_ai_config(&ai_config).is_ok());
}
```

### 5. Update Configuration Service

**File**: `src/config/service.rs`

Add environment variable handling for the new provider:

```rust
// Special handling for [PROVIDER]_API_KEY environment variable
if let Some(api_key) = self.env_provider.get_var("[PROVIDER]_API_KEY") {
    debug!("ProductionConfigService: Found [PROVIDER]_API_KEY environment variable");
    app_config.ai.provider = "[provider_name]".to_string();
    app_config.ai.api_key = Some(api_key);
}
```

Add tests for environment variable loading:

```rust
#[test]
fn test_production_config_service_[provider_name]_api_key_loading() {
    use crate::config::TestEnvironmentProvider;
    use std::sync::Arc;

    let mut env_provider = TestEnvironmentProvider::new();
    env_provider.set_var("[PROVIDER]_API_KEY", "test-api-key");
    env_provider.set_var("SUBX_CONFIG_PATH", "/tmp/test_config_[provider_name].toml");

    let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
        .expect("Failed to create config service");

    let config = service.get_config().expect("Failed to get config");

    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### 6. Update Test Configuration Service

**File**: `src/config/test_service.rs`

Add test methods for the new provider:

```rust
#[test]
fn test_config_service_with_ai_settings_and_key_[provider_name]() {
    let service = TestConfigService::with_ai_settings_and_key(
        "[provider_name]",
        "provider-model",
        "test-api-key",
    );
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.provider, "[provider_name]");
    assert_eq!(config.ai.model, "provider-model");
    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### 7. Update Configuration Builder

**File**: `src/config/builder.rs`

Add tests for the new provider configuration:

```rust
#[test]
fn test_builder_ai_configuration_[provider_name]() {
    let config = TestConfigBuilder::new()
        .with_ai_provider("[provider_name]")
        .with_ai_model("provider-model")
        .with_ai_api_key("test-api-key")
        .build_config();
    assert_eq!(config.ai.provider, "[provider_name]");
    assert_eq!(config.ai.model, "provider-model");
    assert_eq!(config.ai.api_key, Some("test-api-key".to_string()));
}
```

### 8. Update Factory Pattern

**File**: `src/core/factory.rs`

Import the new provider client:

```rust
use crate::services::ai::[provider_name]::[Provider]Client;
```

Add the provider to the factory method:

```rust
pub fn create_ai_provider(ai_config: &crate::config::AIConfig) -> Result<Box<dyn AIProvider>> {
    match ai_config.provider.as_str() {
        "openai" => {
            validate_ai_config(ai_config)?;
            let client = OpenAIClient::from_config(ai_config)?;
            Ok(Box::new(client))
        }
        "[provider_name]" => {
            validate_ai_config(ai_config)?;
            let client = [Provider]Client::from_config(ai_config)?;
            Ok(Box::new(client))
        }
        other => Err(SubXError::config(format!(
            "Unsupported AI provider: {}. Supported providers: openai, [provider_name]",
            other
        ))),
    }
}
```

Add factory tests:

```rust
#[test]
fn test_create_ai_provider_[provider_name]_success() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key(
        "[provider_name]",
        "provider-model",
        "test-api-key",
    );
    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();
    assert!(result.is_ok());
}
```

### 9. Update CLI Documentation

**File**: `src/cli/config_args.rs`

Add examples for the new provider:

```rust
//! # Set AI provider
//! subx config set ai.provider openai
//! subx config set ai.provider [provider_name]
//!
//! # Set AI provider with API key
//! subx-cli config set ai.provider openai
//! subx-cli config set ai.provider [provider_name]
//! subx-cli config set ai.api_key "sk-1234567890abcdef"
//! subx-cli config set ai.api_key "test-api-key"
//! subx-cli config set ai.base_url "https://api.openai.com/v1"
//! subx-cli config set ai.model "provider-model"
```

### 10. Update Validation Tests

**File**: `src/config/validation.rs`

Add the new provider to validation tests:

```rust
#[test]
fn test_validate_enum() {
    let allowed = &["openai", "anthropic", "[provider_name]"];
    assert!(validate_enum("openai", allowed).is_ok());
    assert!(validate_enum("anthropic", allowed).is_ok());
    assert!(validate_enum("[provider_name]", allowed).is_ok());
    assert!(validate_enum("invalid", allowed).is_err());
}
```

### 11. Create Integration Tests

**File**: `tests/[provider_name]_integration_tests.rs`

Create comprehensive integration tests:

```rust
use subx_cli::config::TestConfigService;
use subx_cli::core::ComponentFactory;

#[tokio::test]
async fn test_[provider_name]_client_creation() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key(
        "[provider_name]",
        "provider-model",
        "test-key",
    );

    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_[provider_name]_config_validation() {
    let config_service = TestConfigService::default();
    config_service.set_ai_settings_and_key("[provider_name]", "provider-model", "");

    let factory = ComponentFactory::new(&config_service).unwrap();
    let result = factory.create_ai_provider();

    assert!(result.is_err());
    let error_msg = result.err().unwrap().to_string();
    assert!(
        error_msg.contains("API key cannot be empty")
            || error_msg.contains("Missing [Provider] API Key")
            || error_msg.contains("AI API key is required"),
        "Unexpected error message: {}",
        error_msg
    );
}
```

### 12. Update Documentation

**File**: `docs/configuration-guide.md`

Add configuration section for the new provider:

```markdown
### [Provider] Provider

```toml
[ai]
provider = "[provider_name]"
api_key = "your-api-key"
model = "provider-model"
base_url = "https://api.provider.com/v1"
```
```

**Files**: `README.md` and `README.zh-TW.md`

Add setup instructions for the new provider:

```markdown
# [Provider] setup
export [PROVIDER]_API_KEY="your-api-key"
subx-cli config set ai.provider [provider_name]
subx-cli config set ai.model "provider-model"

# Or OpenAI setup
export OPENAI_API_KEY="your-api-key-here"
subx-cli config set ai.provider openai
```

## File Structure and Changes

Based on the OpenRouter integration, here's the complete list of files that need to be modified:

### Core Implementation Files
- `src/services/ai/[provider_name].rs` - **NEW** - Provider client implementation
- `src/services/ai/mod.rs` - Module declaration
- `src/core/factory.rs` - Factory pattern registration

### Configuration System Files
- `src/config/field_validator.rs` - Field validation rules
- `src/config/validator.rs` - Provider-specific validation
- `src/config/service.rs` - Environment variable handling
- `src/config/test_service.rs` - Test configuration utilities
- `src/config/builder.rs` - Configuration builder tests
- `src/config/validation.rs` - General validation tests

### CLI and Documentation Files
- `src/cli/config_args.rs` - CLI documentation and examples
- `docs/configuration-guide.md` - Configuration documentation
- `README.md` - Main documentation
- `README.zh-TW.md` - Chinese documentation

### Testing Files
- `tests/[provider_name]_integration_tests.rs` - **NEW** - Integration tests

## Testing Requirements

### Unit Tests
- Configuration validation tests
- Provider client creation tests
- Error handling tests
- Environment variable tests

### Integration Tests
- Full provider integration tests
- Configuration loading tests
- API client creation tests
- Error scenario testing

### Test Commands
```bash
# Run all tests
cargo nextest run || true

# Run specific provider tests
cargo nextest run --test [provider_name]_integration_tests

# Run configuration tests
cargo nextest run config

# Check code quality
timeout 240 scripts/quality_check.sh
```

## Configuration Examples

### Environment Variable Setup
```bash
# Set API key via environment variable
export [PROVIDER]_API_KEY="your-api-key"

# Configure provider
subx-cli config set ai.provider [provider_name]
subx-cli config set ai.model "provider-model"
```

### TOML Configuration
```toml
[ai]
provider = "[provider_name]"
api_key = "your-api-key"
model = "provider-model"
base_url = "https://api.provider.com/v1"
temperature = 0.7
max_tokens = 4000
request_timeout_seconds = 120
retry_attempts = 3
retry_delay_ms = 1000
```

### Command Line Configuration
```bash
# Set provider
subx-cli config set ai.provider [provider_name]

# Set API key
subx-cli config set ai.api_key "your-api-key"

# Set model
subx-cli config set ai.model "provider-model"

# Set base URL (if different from default)
subx-cli config set ai.base_url "https://api.provider.com/v1"
```

## Common Pitfalls and Best Practices

### 1. API Key Validation
- Always validate API keys are not empty
- Consider provider-specific API key format validation
- Handle environment variables properly

### 2. Error Handling
- Implement proper retry logic with exponential backoff
- Handle timeout errors gracefully
- Provide helpful error messages for common issues

### 3. Configuration Validation
- Validate all configuration fields
- Ensure base URLs are properly formatted
- Test both valid and invalid configurations

### 4. Testing Coverage
- Test both success and failure scenarios
- Include integration tests for the full flow
- Test environment variable loading
- Test configuration validation

### 5. Documentation
- Update all relevant documentation files
- Include practical examples
- Document any provider-specific requirements

### 6. Provider-Specific Considerations
- Handle provider-specific API formats
- Implement proper authentication headers
- Consider rate limiting and quotas
- Handle provider-specific error responses

### 7. Usage Statistics
- Implement usage tracking with `display_ai_usage`
- Parse token usage from API responses
- Display helpful information to users

### 8. Security Considerations
- Never log API keys
- Use environment variables for sensitive data
- Validate all external inputs

## Conclusion

Adding a new AI provider to SubX CLI requires careful attention to multiple layers of the application. By following this guide and using the OpenRouter integration as a reference, you can successfully add support for new AI providers while maintaining code quality and consistency.

Remember to:
- Test thoroughly across all scenarios
- Update all relevant documentation
- Follow the established patterns and conventions
- Ensure proper error handling and user feedback
- Validate all configurations comprehensively

This guide should serve as a complete reference for future AI provider integrations.
