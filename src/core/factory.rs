//! Component factory for creating configured instances of core components.
//!
//! This module provides a centralized factory for creating instances of core
//! components with proper configuration injection, eliminating the need for
//! global configuration access within individual components.

use crate::services::ai::openai::OpenAIClient;
use crate::services::ai::openrouter::OpenRouterClient;
use crate::services::vad::{LocalVadDetector, VadAudioProcessor, VadSyncDetector};
use crate::{
    Result,
    config::{Config, ConfigService},
    core::{file_manager::FileManager, matcher::engine::MatchEngine},
    error::SubXError,
    services::ai::AIProvider,
};

/// Component factory for creating configured instances.
///
/// This factory provides a centralized way to create core components
/// with proper configuration injection, ensuring consistent component
/// initialization across the application.
///
/// # Examples
///
/// ```rust
/// use subx_cli::core::ComponentFactory;
/// use subx_cli::config::ProductionConfigService;
/// use std::sync::Arc;
///
/// # async fn example() -> subx_cli::Result<()> {
/// let config_service = Arc::new(ProductionConfigService::new()?);
/// let factory = ComponentFactory::new(config_service.as_ref())?;
///
/// // Create components with proper configuration
/// let match_engine = factory.create_match_engine()?;
/// let file_manager = factory.create_file_manager();
/// let ai_provider = factory.create_ai_provider()?;
/// # Ok(())
/// # }
/// ```
pub struct ComponentFactory {
    config: Config,
}

impl ComponentFactory {
    /// Create a new component factory with the given configuration service.
    ///
    /// # Arguments
    ///
    /// * `config_service` - Configuration service to load configuration from
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn new(config_service: &dyn ConfigService) -> Result<Self> {
        let config = config_service.get_config()?;
        Ok(Self { config })
    }

    /// Create a match engine with AI configuration.
    ///
    /// Returns a properly configured MatchEngine instance using
    /// the AI configuration section.
    ///
    /// # Errors
    ///
    /// Returns an error if AI provider creation fails.
    pub fn create_match_engine(&self) -> Result<MatchEngine> {
        let ai_provider = self.create_ai_provider()?;
        let match_config = crate::core::matcher::MatchConfig {
            confidence_threshold: 0.8, // Default value, can be configurable
            max_sample_length: self.config.ai.max_sample_length,
            enable_content_analysis: true,
            backup_enabled: self.config.general.backup_enabled,
            relocation_mode: crate::core::matcher::engine::FileRelocationMode::None,
            conflict_resolution: crate::core::matcher::engine::ConflictResolution::AutoRename,
            ai_model: self.config.ai.model.clone(),
        };
        Ok(MatchEngine::new(ai_provider, match_config))
    }

    /// Create a file manager with general configuration.
    ///
    /// Returns a properly configured FileManager instance using
    /// the general configuration section.
    pub fn create_file_manager(&self) -> FileManager {
        // For now, FileManager doesn't take configuration in its constructor
        // This will be updated when FileManager is refactored to accept config
        FileManager::new()
    }

    /// Create an AI provider with AI configuration.
    ///
    /// Returns a properly configured AI provider instance based on
    /// the provider type specified in the AI configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider type is unsupported or
    /// provider creation fails.
    pub fn create_ai_provider(&self) -> Result<Box<dyn AIProvider>> {
        create_ai_provider(&self.config.ai)
    }

    /// Get a reference to the current configuration.
    ///
    /// Returns a reference to the configuration used by this factory.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Create a VAD sync detector with VAD configuration.
    ///
    /// Returns a properly configured VadSyncDetector instance using the VAD settings.
    ///
    /// # Errors
    ///
    /// Returns an error if VAD sync detector creation fails.
    pub fn create_vad_sync_detector(&self) -> Result<VadSyncDetector> {
        VadSyncDetector::new(self.config.sync.vad.clone())
    }

    /// Create a local VAD detector for audio processing.
    ///
    /// Returns a properly configured LocalVadDetector instance.
    ///
    /// # Errors
    ///
    /// Returns an error if local VAD detector initialization fails.
    pub fn create_vad_detector(&self) -> Result<LocalVadDetector> {
        LocalVadDetector::new(self.config.sync.vad.clone())
    }

    /// Create an audio processor for VAD operations.
    ///
    /// Returns a properly configured VadAudioProcessor instance.
    ///
    /// # Errors
    ///
    /// Returns an error if audio processor initialization fails.
    pub fn create_audio_processor(&self) -> Result<VadAudioProcessor> {
        VadAudioProcessor::new()
    }
}

/// Create an AI provider from AI configuration.
///
/// This function creates the appropriate AI provider based on the
/// provider type specified in the configuration.
///
/// # Arguments
///
/// * `ai_config` - AI configuration containing provider settings
///
/// # Errors
///
/// Returns an error if the provider type is unsupported or creation fails.
/// Validate AI configuration parameters.
fn validate_ai_config(ai_config: &crate::config::AIConfig) -> Result<()> {
    if ai_config.api_key.as_deref().unwrap_or("").trim().is_empty() {
        return Err(SubXError::config(
            "AI API key is required. Set ai.api_key in configuration or use environment variable."
                .to_string(),
        ));
    }
    if ai_config.model.trim().is_empty() {
        return Err(SubXError::config(
            "AI model is required. Set ai.model in configuration.".to_string(),
        ));
    }
    if ai_config.temperature < 0.0 || ai_config.temperature > 2.0 {
        return Err(SubXError::config(
            "AI temperature must be between 0.0 and 2.0.".to_string(),
        ));
    }
    if ai_config.max_tokens == 0 {
        return Err(SubXError::config(
            "AI max_tokens must be greater than 0.".to_string(),
        ));
    }
    Ok(())
}

/// Create an AI provider from AI configuration.
///
/// This function creates the appropriate AI provider based on the
/// provider type specified in the configuration.
pub fn create_ai_provider(ai_config: &crate::config::AIConfig) -> Result<Box<dyn AIProvider>> {
    match ai_config.provider.as_str() {
        "openai" => {
            validate_ai_config(ai_config)?;
            let client = OpenAIClient::from_config(ai_config)?;
            Ok(Box::new(client))
        }
        "openrouter" => {
            validate_ai_config(ai_config)?;
            let client = OpenRouterClient::from_config(ai_config)?;
            Ok(Box::new(client))
        }
        other => Err(SubXError::config(format!(
            "Unsupported AI provider: {}. Supported providers: openai, openrouter",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_service::TestConfigService;

    #[test]
    fn test_component_factory_creation() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service);
        assert!(factory.is_ok());
    }

    #[test]
    fn test_factory_creation() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service);
        assert!(factory.is_ok());
    }

    #[test]
    fn test_create_file_manager() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();

        let _file_manager = factory.create_file_manager();
        // Basic validation that file manager was created
        // FileManager doesn't expose config yet, so just verify creation succeeds
    }

    #[test]
    fn test_unsupported_ai_provider() {
        let mut config = crate::config::Config::default();
        config.ai.provider = "unsupported".to_string();

        let result: Result<Box<dyn AIProvider>> = create_ai_provider(&config.ai);
        assert!(result.is_err());

        match result {
            Err(e) => {
                let error_msg = e.to_string();
                assert!(error_msg.contains("Unsupported AI provider"));
            }
            Ok(_) => panic!("Expected error for unsupported provider"),
        }
    }

    #[test]
    fn test_create_vad_sync_detector() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_vad_sync_detector();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_vad_detector() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_vad_detector();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_audio_processor() {
        let config_service = TestConfigService::default();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_audio_processor();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_openai_success() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "test-api-key");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_missing_api_key() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("API key is required"));
    }

    #[test]
    fn test_create_ai_provider_unsupported_provider() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("unsupported-provider", "model", "key");
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("Unsupported AI provider"));
    }

    #[test]
    fn test_create_ai_provider_with_custom_base_url() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key("openai", "gpt-4.1-mini", "test-api-key");
        config_service.config_mut().ai.base_url = "https://custom-api.com/v1".to_string();
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_ai_provider_openrouter_success() {
        let config_service = TestConfigService::default();
        config_service.set_ai_settings_and_key(
            "openrouter",
            "deepseek/deepseek-r1-0528:free",
            "test-openrouter-key",
        );
        let factory = ComponentFactory::new(&config_service).unwrap();
        let result = factory.create_ai_provider();
        assert!(result.is_ok());
    }
}
