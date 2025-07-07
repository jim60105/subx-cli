use std::sync::Arc;
use subx_cli::config::{ConfigService, TestEnvironmentProvider};

/// Tests for Azure OpenAI environment variable handling
#[cfg(test)]
mod azure_openai_environment_tests {
    use super::*;

    #[test]
    fn test_azure_openai_environment_variable_loading() {
        use subx_cli::config::service::ProductionConfigService;

        let mut env_provider = TestEnvironmentProvider::new();
        env_provider.set_var("AZURE_OPENAI_API_KEY", "test-azure-api-key");
        env_provider.set_var("AZURE_OPENAI_ENDPOINT", "https://test.openai.azure.com");
        env_provider.set_var("AZURE_OPENAI_DEPLOYMENT_ID", "test-deployment");
        env_provider.set_var("AZURE_OPENAI_API_VERSION", "2025-01-01-preview");
        env_provider.set_var("SUBX_CONFIG_PATH", "/tmp/test_config_azure.toml");

        let service = ProductionConfigService::with_env_provider(Arc::new(env_provider))
            .expect("Failed to create config service");

        let config = service.get_config().expect("Failed to get config");

        assert_eq!(config.ai.provider, "azure-openai");
        assert_eq!(config.ai.api_key, Some("test-azure-api-key".to_string()));
        assert_eq!(config.ai.base_url, "https://test.openai.azure.com");
        assert_eq!(config.ai.deployment_id, Some("test-deployment".to_string()));
        assert_eq!(
            config.ai.api_version,
            Some("2025-01-01-preview".to_string())
        );
    }
}
