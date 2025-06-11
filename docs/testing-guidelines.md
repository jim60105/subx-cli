# SubX Testing Guidelines

## Overview

This document outlines the testing principles, practices, and guidelines for the SubX project. Following the architectural principles established in **Backlog #21**, our testing approach emphasizes safety, isolation, and parallel execution while maintaining comprehensive coverage.

## Table of Contents

- [Core Testing Principles](#core-testing-principles)
- [Architecture and Design](#architecture-and-design)
- [Configuration Testing](#configuration-testing)
- [Dependency Injection in Tests](#dependency-injection-in-tests)
- [Testing Macros and Utilities](#testing-macros-and-utilities)
- [Test Organization](#test-organization)
- [Best Practices](#best-practices)
- [Common Patterns](#common-patterns)
- [Performance Guidelines](#performance-guidelines)
- [Troubleshooting](#troubleshooting)

## Core Testing Principles

### 1. Safety First
- **No `unsafe` code** in tests or production code
- **No global state mutation** - all tests must be completely isolated
- **Memory safety** - leverage Rust's ownership system fully

### 2. Complete Isolation
- **No shared state** between tests
- **No side effects** that affect other tests
- **Parallel execution** - all tests must run safely in parallel
- **Deterministic behavior** - tests must produce consistent results

### 3. Dependency Injection
- **ConfigService abstraction** - use `TestConfigService` for all configuration needs
- **Service injection** - inject dependencies rather than creating global instances
- **Mock-friendly design** - enable easy mocking and testing

### 4. Comprehensive Coverage
- **Unit tests** for individual components
- **Integration tests** for component interactions
- **End-to-end tests** for complete workflows
- **Error path testing** for robustness

### ⚠️ Critical Anti-Patterns to Avoid

The following practices **must never** be used in tests as they violate safety and isolation principles:

```rust
// ❌ NEVER DO THESE IN TESTS:

// Global environment variable mutation
std::env::set_var("ANY_VAR", "value");
std::env::remove_var("ANY_VAR");

// Global state modification
static mut GLOBAL_STATE: Option<Config> = None;

// Shared mutable state
static SHARED_CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| Mutex::new(Config::default()));

// File system pollution in global locations
std::fs::write("/tmp/global_config.toml", content);
```

**Why these are prohibited:**
- Create race conditions in parallel test execution
- Cause non-deterministic test results
- Violate test isolation principles
- Make tests depend on execution order
- Break the dependency injection architecture

## Architecture and Design

### Configuration Service Architecture

The SubX project uses a dependency injection pattern for configuration management:

```rust
pub trait ConfigService: Send + Sync {
    fn get_config(&self) -> Result<Config>;
    fn reload(&self) -> Result<()>;
}
```

#### Production vs Test Implementations

- **`ProductionConfigService`** - Loads configuration from files, environment variables
- **`TestConfigService`** - Provides isolated, predictable configuration for testing

### Testing Stack

```
┌─────────────────────────────────────────────────────────┐
│                    Test Layer                           │
├─────────────────────────────────────────────────────────┤
│ Integration Tests │ Unit Tests │ End-to-End Tests       │
├─────────────────────────────────────────────────────────┤
│              Test Utilities & Helpers                  │
├─────────────────────────────────────────────────────────┤
│         TestConfigService & Mock Implementations       │
├─────────────────────────────────────────────────────────┤
│               Dependency Injection Layer               │
├─────────────────────────────────────────────────────────┤
│                Production Code                          │
└─────────────────────────────────────────────────────────┘
```

## Configuration Testing

### Basic Configuration Testing

✅ **Correct Approach** - Using dependency injection:

```rust
use subx_cli::config::TestConfigService;

#[test]
fn test_ai_configuration() {
    let config_service = TestConfigService::with_ai_settings_and_key(
        "openai",
        "gpt-4",
        "sk-test-key"
    );
    
    let config = config_service.get_config().unwrap();
    assert_eq!(config.ai.provider, "openai");
    assert_eq!(config.ai.model, "gpt-4");
    assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
}
```

❌ **Incorrect Approach** - See [Critical Anti-Patterns](#️-critical-anti-patterns-to-avoid) section above.

### Configuration Priority Testing

```rust
#[test]
fn test_configuration_precedence() {
    let test_service = TestConfigService::with_ai_settings_and_key(
        "openai", 
        "gpt-4", 
        "explicit-key"
    );

    let config = test_service.get_config().unwrap();
    
    // Explicit configuration should take precedence
    assert_eq!(config.ai.api_key, Some("explicit-key".to_string()));
    assert_eq!(config.ai.provider, "openai");
}
```

### Configuration Validation Testing

```rust
#[test]
fn test_invalid_configuration() {
    let mut config = Config::default();
    config.ai.temperature = 2.5; // Invalid temperature

    let test_service = TestConfigService::new(config);
    let result = test_service.get_config();
    
    assert!(result.is_err());
    // Verify specific error type if needed
}
```

## Dependency Injection in Tests

### Service Injection Pattern

```rust
use std::sync::Arc;
use subx_cli::config::{ConfigService, TestConfigService};

#[test]
async fn test_command_with_dependency_injection() {
    // Create isolated configuration
    let config_service: Arc<dyn ConfigService> = Arc::new(
        TestConfigService::with_ai_settings("openai", "gpt-4")
    );
    
    // Inject into command
    let result = some_command_function(&args, &*config_service).await;
    
    assert!(result.is_ok());
}
```

### Test Helper Pattern

```rust
use crate::common::cli_helpers::CLITestHelper;

#[test]
async fn test_cli_workflow() {
    let helper = CLITestHelper::with_ai_settings("openai", "gpt-4");
    
    // Create test files
    helper.create_subtitle_file("test.srt", "Test content").await.unwrap();
    helper.create_subtitle_file("reference.srt", "Reference content").await.unwrap();
    
    // Execute command with isolated environment
    let result = helper.execute_match_command(&["test.srt", "reference.srt"]).await;
    
    assert!(result.is_ok());
}
```

### Mock Service Implementation

```rust
#[derive(Debug)]
struct MockConfigService {
    config: Config,
    reload_count: Arc<Mutex<usize>>,
}

impl MockConfigService {
    fn new(config: Config) -> Self {
        Self {
            config,
            reload_count: Arc::new(Mutex::new(0)),
        }
    }
    
    fn get_reload_count(&self) -> usize {
        *self.reload_count.lock().unwrap()
    }
}

impl ConfigService for MockConfigService {
    fn get_config(&self) -> Result<Config> {
        Ok(self.config.clone())
    }
    
    fn reload(&self) -> Result<()> {
        *self.reload_count.lock().unwrap() += 1;
        Ok(())
    }
}

#[test]
fn test_service_reload_behavior() {
    let mock_service = MockConfigService::new(Config::default());
    
    assert_eq!(mock_service.get_reload_count(), 0);
    
    mock_service.reload().unwrap();
    assert_eq!(mock_service.get_reload_count(), 1);
}
```

## Test Organization

### Directory Structure

```
tests/
├── common/                     # Shared test utilities
│   ├── cli_helpers.rs         # CLI testing helpers
│   ├── file_managers.rs       # File operation helpers
│   ├── mock_generators.rs     # Mock data generators
│   └── validators.rs          # Common validation utilities
├── integration_tests.rs       # Main integration tests
├── config_integration_tests.rs # Configuration integration tests
├── parallel_processing_integration_tests.rs
└── cli/                       # CLI-specific tests
    ├── config_args_tests.rs
    └── ui_tests.rs
```

### Unit Test Location

Unit tests should be located in the same file as the code they test:

```rust
// src/config/service.rs

impl ProductionConfigService {
    // Implementation...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config_service_creation() {
        let service = ProductionConfigService::new();
        assert!(service.is_ok());
    }
}
```

### Integration Test Organization

```rust
// tests/config_integration_tests.rs

use std::sync::Arc;
use subx_cli::config::{ConfigService, TestConfigService};

mod config_loading {
    use super::*;

    #[test]
    fn test_default_configuration_loading() {
        // Test implementation...
    }
    
    #[test]
    fn test_custom_configuration_loading() {
        // Test implementation...
    }
}

mod config_validation {
    use super::*;
    
    #[test]
    fn test_configuration_validation() {
        // Test implementation...
    }
}
```

## Best Practices

### 1. Test Naming

Use descriptive test names that clearly indicate what is being tested:

```rust
#[test]
fn test_config_service_with_openai_api_key() { /* ... */ }

#[test]
fn test_match_command_with_invalid_file_paths() { /* ... */ }

#[test]
fn test_subtitle_format_conversion_srt_to_vtt() { /* ... */ }
```

### 2. Test Data Management

Use the test helper utilities for consistent test data:

```rust
#[test]
async fn test_subtitle_parsing() {
    let helper = CLITestHelper::new();
    
    // Use helper to create predictable test files
    let srt_content = helper.generate_srt_content(vec![
        ("00:00:01,000", "00:00:03,000", "First subtitle"),
        ("00:00:04,000", "00:00:06,000", "Second subtitle"),
    ]);
    
    helper.create_subtitle_file("test.srt", &srt_content).await.unwrap();
    
    // Test parsing logic...
}
```

### 3. Error Testing

Always test error conditions:

```rust
#[test]
fn test_invalid_configuration_returns_error() {
    let mut config = Config::default();
    config.ai.temperature = -1.0; // Invalid value
    
    let service = TestConfigService::new(config);
    let result = service.get_config();
    
    assert!(result.is_err());
    
    match result.unwrap_err() {
        SubXError::Config(msg) => {
            assert!(msg.contains("temperature"));
        }
        _ => panic!("Expected ConfigError"),
    }
}
```

### 4. Async Testing

Use appropriate async testing patterns:

```rust
#[tokio::test]
async fn test_async_operation() {
    let config_service = Arc::new(TestConfigService::with_defaults());
    
    let result = async_function_under_test(&*config_service).await;
    
    assert!(result.is_ok());
}
```

### 5. Resource Cleanup

Use RAII and Drop traits for automatic cleanup:

```rust
#[test]
fn test_with_temporary_resources() {
    let helper = CLITestHelper::new(); // Automatically cleaned up on drop
    
    // Test logic that uses temporary files...
    
    // No manual cleanup needed - Drop trait handles it
}
```

## Common Patterns

### Pattern 1: Configuration Service Injection

```rust
async fn test_function_with_config_injection<T: ConfigService>(
    config_service: &T
) -> Result<()> {
    let config = config_service.get_config()?;
    // Use config for test logic...
    Ok(())
}

#[tokio::test]
async fn test_with_custom_config() {
    let config_service = TestConfigService::with_ai_settings("openai", "gpt-4");
    let result = test_function_with_config_injection(&config_service).await;
    assert!(result.is_ok());
}
```

### Pattern 2: Test Data Builders

```rust
struct TestConfigBuilder {
    config: Config,
}

impl TestConfigBuilder {
    fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }
    
    fn with_ai_provider(mut self, provider: &str) -> Self {
        self.config.ai.provider = provider.to_string();
        self
    }
    
    fn with_api_key(mut self, key: &str) -> Self {
        self.config.ai.api_key = Some(key.to_string());
        self
    }
    
    fn build(self) -> TestConfigService {
        TestConfigService::new(self.config)
    }
}

#[test]
fn test_with_builder_pattern() {
    let config_service = TestConfigBuilder::new()
        .with_ai_provider("openai")
        .with_api_key("sk-test-key")
        .build();
    
    let config = config_service.get_config().unwrap();
    assert_eq!(config.ai.provider, "openai");
}
```

### Pattern 3: Parameterized Testing

```rust
#[test]
fn test_multiple_ai_providers() {
    let test_cases = vec![
        ("openai", "gpt-4"),
        ("openai", "gpt-3.5-turbo"),
        ("anthropic", "claude-3"),
    ];
    
    for (provider, model) in test_cases {
        let config_service = TestConfigService::with_ai_settings(provider, model);
        let config = config_service.get_config().unwrap();
        
        assert_eq!(config.ai.provider, provider);
        assert_eq!(config.ai.model, model);
    }
}
```

## Performance Guidelines

### 1. Parallel Test Execution

All tests must be designed for parallel execution following the principles in [Critical Anti-Patterns](#️-critical-anti-patterns-to-avoid):

```rust
// ✅ Safe for parallel execution - uses isolated configuration
#[test]
fn test_isolated_configuration() {
    let config_service = TestConfigService::with_defaults();
    // Test logic using only injected dependencies
}
```

### 2. Resource Efficiency

Minimize resource usage in tests:

```rust
#[test]
fn test_efficient_resource_usage() {
    // Use minimal configuration
    let config_service = TestConfigService::with_defaults();
    
    // Avoid creating unnecessary large test data
    let small_test_data = "minimal test content";
    
    // Use appropriate data structures
    let result = process_small_data(small_test_data, &config_service);
    assert!(result.is_ok());
}
```

### 3. Test Execution Time

Keep individual tests fast:

```rust
#[test]
fn test_fast_operation() {
    let start = std::time::Instant::now();
    
    // Test logic should complete quickly
    let config_service = TestConfigService::with_defaults();
    let result = fast_operation(&config_service);
    
    assert!(result.is_ok());
    assert!(start.elapsed() < std::time::Duration::from_millis(100));
}
```

## Troubleshooting

### Common Issues and Solutions

#### Issue: Test Failures in Parallel Execution

**Symptom**: Tests pass individually but fail when run in parallel.

**Root Cause**: Violation of [Critical Anti-Patterns](#️-critical-anti-patterns-to-avoid) - likely global state mutation.

**Solution**: Use dependency injection with isolated configuration:

```rust
// ✅ Correct approach - isolated state
#[test]
fn test_with_isolated_config() {
    let mut config = Config::default();
    config.some_value = "test".to_string();
    let config_service = TestConfigService::new(config);
    // Test logic...
}
```

#### Issue: Inconsistent Test Results

**Symptom**: Tests produce different results on different runs.

**Solution**: Eliminate non-deterministic behavior:

```rust
// Problem: Non-deterministic behavior
#[test]
fn problematic_test() {
    let random_value = rand::random::<u32>(); // ❌ Non-deterministic
    // Test logic using random_value...
}

// Solution: Use fixed test data
#[test]
fn fixed_test() {
    let fixed_value = 12345u32; // ✅ Deterministic
    // Test logic using fixed_value...
}
```

#### Issue: Configuration Loading Errors

**Symptom**: Configuration-related tests fail with loading errors.

**Root Cause**: Attempting to use production configuration services in tests.

**Solution**: Use `TestConfigService` for all test scenarios:

```rust
// ✅ Correct approach - test configuration service
#[test]
fn test_with_proper_config() {
    let config_service = TestConfigService::with_defaults();
    let config = config_service.get_config().unwrap();
    // Test logic...
}
```

### Debugging Test Issues

1. **Run tests individually**: `cargo test test_name -- --exact`
2. **Enable debug logging**: `RUST_LOG=debug cargo test`
3. **Check for anti-patterns**: Review [Critical Anti-Patterns](#️-critical-anti-patterns-to-avoid) section
4. **Verify isolation**: Ensure tests don't create shared files or state
5. **Test in parallel**: `cargo test -- --test-threads=8` to catch race conditions

### Performance Debugging

```rust
#[test]
fn test_with_performance_monitoring() {
    let start = std::time::Instant::now();
    
    let config_service = TestConfigService::with_defaults();
    
    let config_load_time = start.elapsed();
    println!("Config loading took: {:?}", config_load_time);
    
    let operation_start = std::time::Instant::now();
    let result = test_operation(&config_service);
    let operation_time = operation_start.elapsed();
    
    println!("Operation took: {:?}", operation_time);
    
    assert!(result.is_ok());
    assert!(operation_time < std::time::Duration::from_millis(500));
}
```

## Conclusion

Following these guidelines ensures that SubX maintains high code quality, safety, and testability. The dependency injection architecture enables comprehensive testing without sacrificing safety or performance.

### Key Takeaways

1. **Always use `TestConfigService`** for configuration in tests
2. **Follow the [Critical Anti-Patterns](#️-critical-anti-patterns-to-avoid)** guidelines religiously
3. **Design for parallel execution** from the start
4. **Use dependency injection** throughout the testing architecture
5. **Maintain comprehensive coverage** with isolated, fast tests

### Resources

- [Backlog #21: Eliminate Unsafe Config Manager](../.github/plans/backlogs/21-eliminate-unsafe-config-manager-backlog.md)
- [Technical Architecture Documentation](./tech-architecture.md)
- [Rustdoc Guidelines](./rustdoc-guidelines.md)

For questions or improvements to these guidelines, please refer to the project's issue tracker or discuss with the development team.

## Testing Macros and Utilities

SubX provides convenient testing macros that simplify test configuration while enforcing architectural best practices. These macros automatically use dependency injection and ensure test isolation.

### Configuration Testing Macros

#### Basic Configuration Macros

**`test_with_default_config!`** - Run a test with default configuration:

```rust
use subx_cli::test_with_default_config;

#[test]
fn test_with_defaults() {
    test_with_default_config!(|config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4o-mini");
    });
}
```

**`test_with_ai_config!`** - Run a test with specific AI settings:

```rust
use subx_cli::test_with_ai_config;

#[test]
fn test_with_anthropic() {
    test_with_ai_config!("anthropic", "claude-3", |config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "anthropic");
        assert_eq!(config.ai.model, "claude-3");
    });
}
```

**`test_with_ai_config_and_key!`** - Run a test with AI settings and API key:

```rust
use subx_cli::test_with_ai_config_and_key;

#[test]
fn test_with_full_ai_config() {
    test_with_ai_config_and_key!("openai", "gpt-4", "sk-test-key", |config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
        assert_eq!(config.ai.model, "gpt-4");
        assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
    });
}
```

#### Specialized Configuration Macros

**`test_with_sync_config!`** - Test with synchronization settings:

```rust
use subx_cli::test_with_sync_config;

#[test]
fn test_sync_configuration() {
    test_with_sync_config!(0.8, 45.0, |config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.sync.correlation_threshold, 0.8);
        assert_eq!(config.sync.max_offset_seconds, 45.0);
    });
}
```

**`test_with_parallel_config!`** - Test with parallel processing settings:

```rust
use subx_cli::test_with_parallel_config;

#[test]
fn test_parallel_configuration() {
    test_with_parallel_config!(8, 200, |config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.general.max_concurrent_jobs, 8);
        assert_eq!(config.parallel.task_queue_size, 200);
    });
}
```

### Service Creation Macros

For more complex tests that need to access the configuration service multiple times:

**`create_default_test_config_service!`** - Create a service variable:

```rust
use subx_cli::create_default_test_config_service;

#[test]
fn test_multiple_operations() {
    create_default_test_config_service!(service);
    
    // First operation
    let config1 = service.get_config().unwrap();
    assert_eq!(config1.ai.provider, "openai");
    
    // Second operation
    service.reload().unwrap();
    let config2 = service.get_config().unwrap();
    assert_eq!(config2.ai.provider, "openai");
}
```

**`create_test_config_service!`** - Create a service with custom configuration:

```rust
use subx_cli::{create_test_config_service, config::TestConfigBuilder};

#[test]
fn test_custom_service() {
    create_test_config_service!(
        service,
        TestConfigBuilder::new().with_ai_provider("custom")
    );
    
    let config = service.get_config().unwrap();
    assert_eq!(config.ai.provider, "custom");
}
```

### Advanced Configuration Macro

**`test_with_config!`** - Use with custom `TestConfigBuilder`:

```rust
use subx_cli::{test_with_config, config::TestConfigBuilder};

#[test]
fn test_complex_configuration() {
    test_with_config!(
        TestConfigBuilder::new()
            .with_ai_provider("openai")
            .with_ai_model("gpt-4")
            .with_sync_threshold(0.9)
            .with_max_offset(30.0),
        |config_service| {
            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.provider, "openai");
            assert_eq!(config.ai.model, "gpt-4");
            assert_eq!(config.sync.correlation_threshold, 0.9);
            assert_eq!(config.sync.max_offset_seconds, 30.0);
        }
    );
}
```

### Benefits of Using Testing Macros

1. **Automatic Best Practices**: Macros enforce dependency injection patterns
2. **Reduced Boilerplate**: Less repetitive configuration setup code
3. **Type Safety**: Compile-time validation of configuration parameters
4. **Consistency**: Standardized testing patterns across the codebase
5. **Isolation**: Guaranteed test isolation without global state

### When to Use Each Macro

| Macro | Use Case |
|-------|----------|
| `test_with_default_config!` | Testing basic functionality with standard settings |
| `test_with_ai_config!` | Testing AI-related features with specific providers |
| `test_with_ai_config_and_key!` | Testing authenticated AI operations |
| `test_with_sync_config!` | Testing synchronization algorithms |
| `test_with_parallel_config!` | Testing parallel processing features |
| `create_*_service!` | Complex tests needing service reuse |
| `test_with_config!` | Custom configurations with multiple parameters |

### Macro vs Manual Configuration

Both manual configuration and macros are valid approaches. Choose based on your needs:

```rust
// Manual configuration (more explicit)
#[test]
fn test_manual_config() {
    let config_service = TestConfigService::with_ai_settings_and_key(
        "openai", 
        "gpt-4", 
        "sk-test-key"
    );
    let config = config_service.get_config().unwrap();
    assert_eq!(config.ai.provider, "openai");
}

// Using macro (more concise)
#[test]
fn test_macro_config() {
    test_with_ai_config_and_key!("openai", "gpt-4", "sk-test-key", |config_service| {
        let config = config_service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
    });
}
```

**Recommendation**: Use macros for simple, common configurations and manual configuration for complex, custom setups.
