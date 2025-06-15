//! Test macros for convenient configuration service testing.
//!
//! This module provides convenient macros for creating test configurations
//! and running tests with specific configuration services.

/// Run a test with a custom configuration builder.
///
/// This macro takes a configuration builder and a test closure,
/// creates a configuration service, and runs the test with it.
///
/// # Examples
///
/// ```rust
/// use subx_cli::{test_with_config, config::{TestConfigBuilder, ConfigService}};
///
/// test_with_config!(
///     TestConfigBuilder::new().with_ai_provider("openai"),
///     |config_service: &dyn ConfigService| {
///         let config = config_service.get_config().unwrap();
///         assert_eq!(config.ai.provider, "openai");
///     }
/// );
/// ```
#[macro_export]
macro_rules! test_with_config {
    ($config_builder:expr, $test:expr) => {{
        let config_service = $config_builder.build_service();
        $test(&config_service)
    }};
}

/// Execute ProductionConfigService tests with specified environment variable mapping.
///
/// This macro creates a TestEnvironmentProvider, sets the specified environment variables,
/// then uses that provider to create a ProductionConfigService for testing.
///
/// # Arguments
/// * `$env_vars` - Environment variable mapping expression (HashMap<&str, &str>)
/// * `$test` - Test closure that receives a ProductionConfigService reference
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_production_config_with_env, std::collections::HashMap};
///
/// let env_vars = [
///     ("OPENAI_API_KEY", "sk-test-key"),
///     ("OPENAI_BASE_URL", "https://test.api.com/v1")
/// ].iter().cloned().collect::<HashMap<_, _>>();
///
/// test_production_config_with_env!(env_vars, |service| {
///     let config = service.get_config().unwrap();
///     assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
///     assert_eq!(config.ai.base_url, "https://test.api.com/v1");
/// });
/// ```
#[macro_export]
macro_rules! test_production_config_with_env {
    ($env_vars:expr, $test:expr) => {{
        use std::sync::Arc;

        let mut env_provider = $crate::config::TestEnvironmentProvider::new();

        // Convert environment variable mapping to strings and set them in the provider
        for (key, value) in $env_vars {
            env_provider.set_var(key, value);
        }

        let service =
            $crate::config::ProductionConfigService::with_env_provider(Arc::new(env_provider))
                .expect("Failed to create ProductionConfigService with environment provider");

        $test(&service)
    }};
}

/// Execute ProductionConfigService tests with OPENAI environment variables.
///
/// This macro is a convenience version of test_production_config_with_env!,
/// specifically designed for testing OPENAI_API_KEY and OPENAI_BASE_URL environment variables.
///
/// # Arguments
/// * `$api_key` - OPENAI_API_KEY value (Option<&str>)
/// * `$base_url` - OPENAI_BASE_URL value (Option<&str>)  
/// * `$test` - Test closure that receives a ProductionConfigService reference
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::test_production_config_with_openai_env;
///
/// test_production_config_with_openai_env!(
///     Some("sk-test-key"),
///     Some("https://test.api.com/v1"),
///     |service| {
///         let config = service.get_config().unwrap();
///         assert_eq!(config.ai.api_key, Some("sk-test-key".to_string()));
///         assert_eq!(config.ai.base_url, "https://test.api.com/v1");
///     }
/// );
/// ```
#[macro_export]
macro_rules! test_production_config_with_openai_env {
    ($api_key:expr, $base_url:expr, $test:expr) => {{
        use std::sync::Arc;

        let mut env_provider = $crate::config::TestEnvironmentProvider::new();

        // Set OPENAI_API_KEY (if provided)
        if let Some(api_key) = $api_key {
            env_provider.set_var("OPENAI_API_KEY", api_key);
        }

        // Set OPENAI_BASE_URL (if provided)
        if let Some(base_url) = $base_url {
            env_provider.set_var("OPENAI_BASE_URL", base_url);
        }

        let service =
            $crate::config::ProductionConfigService::with_env_provider(Arc::new(env_provider))
                .expect(
                    "Failed to create ProductionConfigService with OPENAI environment variables",
                );

        $test(&service)
    }};
}

/// Create a temporary ProductionConfigService with environment variable provider for test functions.
///
/// This macro creates a ProductionConfigService variable with specified environment variables
/// that can be used throughout the entire test function.
///
/// # Arguments
/// * `$service_name` - Service variable name
/// * `$env_vars` - Environment variable mapping expression (HashMap<&str, &str>)
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::create_production_config_service_with_env;
///
/// fn my_test() {
///     let env_vars = [("OPENAI_API_KEY", "sk-test")].iter().cloned().collect();
///     create_production_config_service_with_env!(service, env_vars);
///
///     let config = service.get_config().unwrap();
///     assert_eq!(config.ai.api_key, Some("sk-test".to_string()));
/// }
/// ```
#[macro_export]
macro_rules! create_production_config_service_with_env {
    ($service_name:ident, $env_vars:expr) => {
        use std::sync::Arc;

        let mut env_provider = $crate::config::TestEnvironmentProvider::new();

        for (key, value) in $env_vars {
            env_provider.set_var(key, value);
        }

        let $service_name =
            $crate::config::ProductionConfigService::with_env_provider(Arc::new(env_provider))
                .expect("Failed to create ProductionConfigService with environment provider");
    };
}

/// Create a ProductionConfigService with empty environment variables for testing.
///
/// This macro creates a ProductionConfigService without any environment variables,
/// used for testing default behavior.
///
/// # Arguments
/// * `$service_name` - Service variable name
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::create_production_config_service_with_empty_env;
///
/// fn my_test() {
///     create_production_config_service_with_empty_env!(service);
///
///     let config = service.get_config().unwrap();
///     assert_eq!(config.ai.api_key, None); // Expected no API key
/// }
/// ```
#[macro_export]
macro_rules! create_production_config_service_with_empty_env {
    ($service_name:ident) => {
        create_production_config_service_with_env!($service_name, std::collections::HashMap::new())
    };
}

#[cfg(test)]
mod env_macro_tests {
    use crate::config::service::ConfigService;
    use std::collections::HashMap;

    #[test]
    fn test_production_config_with_env_macro() {
        let env_vars: HashMap<&str, &str> = [
            ("OPENAI_API_KEY", "sk-macro-test"),
            ("OPENAI_BASE_URL", "https://macro.test.com/v1"),
        ]
        .iter()
        .cloned()
        .collect();

        test_production_config_with_env!(
            env_vars,
            |service: &crate::config::ProductionConfigService| {
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.api_key, Some("sk-macro-test".to_string()));
                assert_eq!(config.ai.base_url, "https://macro.test.com/v1");
            }
        );
    }

    #[test]
    fn test_production_config_with_openai_env_macro_both() {
        test_production_config_with_openai_env!(
            Some("sk-openai-macro"),
            Some("https://openai.macro.com/v1"),
            |service: &crate::config::ProductionConfigService| {
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.api_key, Some("sk-openai-macro".to_string()));
                assert_eq!(config.ai.base_url, "https://openai.macro.com/v1");
            }
        );
    }

    #[test]
    fn test_production_config_with_openai_env_macro_api_key_only() {
        test_production_config_with_openai_env!(
            Some("sk-only-key"),
            None,
            |service: &crate::config::ProductionConfigService| {
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.api_key, Some("sk-only-key".to_string()));
                // base_url should use default value
                assert_eq!(config.ai.base_url, "https://api.openai.com/v1");
            }
        );
    }

    #[test]
    fn test_production_config_with_openai_env_macro_base_url_only() {
        test_production_config_with_openai_env!(
            None,
            Some("https://only-url.com/v1"),
            |service: &crate::config::ProductionConfigService| {
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.api_key, None);
                assert_eq!(config.ai.base_url, "https://only-url.com/v1");
            }
        );
    }

    #[test]
    fn test_production_config_with_openai_env_macro_empty() {
        test_production_config_with_openai_env!(
            None,
            None,
            |service: &crate::config::ProductionConfigService| {
                let config = service.get_config().unwrap();
                assert_eq!(config.ai.api_key, None);
                assert_eq!(config.ai.base_url, "https://api.openai.com/v1");
            }
        );
    }

    #[test]
    fn test_create_production_config_service_with_env_macro() {
        let env_vars: HashMap<&str, &str> = [("OPENAI_API_KEY", "sk-create-macro")]
            .iter()
            .cloned()
            .collect();

        create_production_config_service_with_env!(service, env_vars);

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.api_key, Some("sk-create-macro".to_string()));
    }

    #[test]
    fn test_create_production_config_service_with_empty_env_macro() {
        create_production_config_service_with_empty_env!(service);

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.api_key, None);
        assert_eq!(config.ai.base_url, "https://api.openai.com/v1");
    }
}

/// Run a test with the default configuration.
///
/// This macro creates a test configuration service with default settings
/// and runs the provided test closure with it.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_with_default_config, config::ConfigService};
///
/// test_with_default_config!(|config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "openai");
/// });
/// ```
#[macro_export]
macro_rules! test_with_default_config {
    ($test:expr) => {
        test_with_config!($crate::config::TestConfigBuilder::new(), $test)
    };
}

/// Run a test with specific AI configuration.
///
/// This macro creates a test configuration service with the specified
/// AI provider and model, then runs the test closure.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_with_ai_config, config::ConfigService};
///
/// test_with_ai_config!("anthropic", "claude-3", |config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "anthropic");
///     assert_eq!(config.ai.model, "claude-3");
/// });
/// ```
#[macro_export]
macro_rules! test_with_ai_config {
    ($provider:expr, $model:expr, $test:expr) => {
        test_with_config!(
            $crate::config::TestConfigBuilder::new()
                .with_ai_provider($provider)
                .with_ai_model($model),
            $test
        )
    };
}

/// Run a test with specific AI configuration including API key.
///
/// This macro creates a test configuration service with the specified
/// AI provider, model, and API key, then runs the test closure.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_with_ai_config_and_key, config::ConfigService};
///
/// test_with_ai_config_and_key!("openai", "gpt-4.1", "test-key", |config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "openai");
///     assert_eq!(config.ai.model, "gpt-4.1");
///     assert_eq!(config.ai.api_key, Some("test-key".to_string()));
/// });
/// ```
#[macro_export]
macro_rules! test_with_ai_config_and_key {
    ($provider:expr, $model:expr, $api_key:expr, $test:expr) => {
        test_with_config!(
            $crate::config::TestConfigBuilder::new()
                .with_ai_provider($provider)
                .with_ai_model($model)
                .with_ai_api_key($api_key),
            $test
        )
    };
}

/// Run a test with specific sync configuration.
///
/// This macro creates a test configuration service with the specified
/// synchronization parameters, then runs the test closure.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_with_sync_config, config::ConfigService};
///
/// test_with_sync_config!(0.8, 45.0, |config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.sync.correlation_threshold, 0.8);
///     assert_eq!(config.sync.max_offset_seconds, 45.0);
/// });
/// ```
#[macro_export]
macro_rules! test_with_sync_config {
    ($threshold:expr, $max_offset:expr, $test:expr) => {
        test_with_config!(
            $crate::config::TestConfigBuilder::new()
                .with_vad_sensitivity($threshold)
                .with_sync_method("vad"),
            $test
        )
    };
}

/// Run a test with specific parallel processing configuration.
///
/// This macro creates a test configuration service with the specified
/// parallel processing parameters, then runs the test closure.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::{test_with_parallel_config, config::ConfigService};
///
/// test_with_parallel_config!(8, 200, |config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.general.max_concurrent_jobs, 8);
///     assert_eq!(config.parallel.task_queue_size, 200);
/// });
/// ```
#[macro_export]
macro_rules! test_with_parallel_config {
    ($max_jobs:expr, $queue_size:expr, $test:expr) => {
        test_with_config!(
            $crate::config::TestConfigBuilder::new()
                .with_max_concurrent_jobs($max_jobs)
                .with_task_queue_size($queue_size),
            $test
        )
    };
}

/// Create a temporary test configuration service for use in test functions.
///
/// This macro creates a configuration service variable that can be used
/// throughout a test function.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::create_test_config_service;
///
/// fn my_test() {
///     create_test_config_service!(service, TestConfigBuilder::new().with_ai_provider("openai"));
///     
///     let config = service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "openai");
/// }
/// ```
#[macro_export]
macro_rules! create_test_config_service {
    ($service_name:ident, $config_builder:expr) => {
        let $service_name = $config_builder.build_service();
    };
}

/// Create a temporary test configuration service with default settings.
///
/// This macro creates a configuration service variable with default settings
/// that can be used throughout a test function.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::create_default_test_config_service;
///
/// fn my_test() {
///     create_default_test_config_service!(service);
///     
///     let config = service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "openai");
/// }
/// ```
#[macro_export]
macro_rules! create_default_test_config_service {
    ($service_name:ident) => {
        create_test_config_service!($service_name, $crate::config::TestConfigBuilder::new());
    };
}

#[cfg(test)]
mod tests {
    use crate::config::{ConfigService, TestConfigBuilder};

    #[test]
    fn test_macro_with_config() {
        test_with_config!(
            TestConfigBuilder::new().with_ai_provider("test_provider"),
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.get_config().unwrap();
                assert_eq!(config.ai.provider, "test_provider");
            }
        );
    }

    #[test]
    fn test_macro_with_default_config() {
        test_with_default_config!(|config_service: &crate::config::TestConfigService| {
            let config = config_service.get_config().unwrap();
            assert_eq!(config.ai.provider, "openai");
        });
    }

    #[test]
    fn test_macro_with_ai_config() {
        test_with_ai_config!(
            "anthropic",
            "claude-3",
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.get_config().unwrap();
                assert_eq!(config.ai.provider, "anthropic");
                assert_eq!(config.ai.model, "claude-3");
            }
        );
    }

    #[test]
    fn test_macro_with_ai_config_and_key() {
        test_with_ai_config_and_key!(
            "openai",
            "gpt-4.1",
            "test-key",
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.get_config().unwrap();
                assert_eq!(config.ai.provider, "openai");
                assert_eq!(config.ai.model, "gpt-4.1");
                assert_eq!(config.ai.api_key, Some("test-key".to_string()));
            }
        );
    }

    #[test]
    fn test_macro_with_sync_config() {
        test_with_sync_config!(
            0.9,
            60.0,
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.config();
                // Test new sync configuration structure
                assert_eq!(config.sync.vad.sensitivity, 0.9);
                assert_eq!(config.sync.default_method, "vad");
            }
        );
    }

    #[test]
    fn test_macro_with_parallel_config() {
        test_with_parallel_config!(
            16,
            500,
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.get_config().unwrap();
                assert_eq!(config.general.max_concurrent_jobs, 16);
                assert_eq!(config.parallel.task_queue_size, 500);
            }
        );
    }

    #[test]
    fn test_create_test_config_service_macro() {
        create_test_config_service!(
            service,
            TestConfigBuilder::new().with_ai_provider("macro_test")
        );

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "macro_test");
    }

    #[test]
    fn test_create_default_test_config_service_macro() {
        create_default_test_config_service!(service);

        let config = service.get_config().unwrap();
        assert_eq!(config.ai.provider, "openai");
    }
}
