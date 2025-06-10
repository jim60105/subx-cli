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
/// test_with_ai_config_and_key!("openai", "gpt-4", "test-key", |config_service: &dyn ConfigService| {
///     let config = config_service.get_config().unwrap();
///     assert_eq!(config.ai.provider, "openai");
///     assert_eq!(config.ai.model, "gpt-4");
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
                .with_sync_threshold($threshold)
                .with_max_offset($max_offset),
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
            "gpt-4",
            "test-key",
            |config_service: &crate::config::TestConfigService| {
                let config = config_service.get_config().unwrap();
                assert_eq!(config.ai.provider, "openai");
                assert_eq!(config.ai.model, "gpt-4");
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
                let config = config_service.get_config().unwrap();
                assert_eq!(config.sync.correlation_threshold, 0.9);
                assert_eq!(config.sync.max_offset_seconds, 60.0);
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
