//! AI client factory for creating provider instances based on configuration.
//!
//! This module provides a factory pattern implementation for creating AI service
//! provider instances. It supports multiple AI providers and handles the creation
//! and configuration of appropriate client instances based on the provided configuration.
//!
//! # Supported Providers
//!
//! - **OpenAI**: GPT-3.5, GPT-4, and other OpenAI models
//! - **Anthropic**: Claude models (planned)
//! - **Google**: Gemini models (planned)
//! - **Local Models**: Local AI models via Ollama (planned)
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::services::ai::AIClientFactory;
//! use subx_cli::config::AIConfig;
//!
//! // Create OpenAI client from configuration
//! let config = AIConfig {
//!     provider: "openai".to_string(),
//!     api_key: Some("sk-...".to_string()),
//!     model: "gpt-4".to_string(),
//!     // ... other fields
//! };
//!
//! let client = AIClientFactory::create_client(&config)?;
//! let result = client.analyze_content(request).await?;
//! ```
use crate::config::AIConfig;
use crate::error::SubXError;
use crate::services::ai::{AIProvider, OpenAIClient};

/// AI client factory for creating provider instances.
///
/// The `AIClientFactory` provides a centralized way to create AI provider
/// instances based on configuration. It supports multiple AI service providers
/// and handles the complexity of client initialization and configuration.
///
/// # Design Pattern
///
/// This factory follows the Abstract Factory pattern, providing a single
/// interface for creating different types of AI clients while hiding the
/// specific implementation details from the consumer.
///
/// # Provider Selection
///
/// The factory automatically selects the appropriate provider implementation
/// based on the `provider` field in the configuration:
/// - `"openai"` - Creates an OpenAI client instance
/// - `"anthropic"` - Creates an Anthropic client (future support)
/// - `"google"` - Creates a Google AI client (future support)
/// - `"local"` - Creates a local model client (future support)
///
/// # Error Handling
///
/// The factory returns detailed errors for:
/// - Unsupported provider names
/// - Invalid configuration parameters
/// - Missing required credentials
/// - Network connectivity issues during client creation
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::services::ai::AIClientFactory;
/// use subx_cli::config::AIConfig;
///
/// // Create a configured OpenAI client
/// let config = AIConfig {
///     provider: "openai".to_string(),
///     api_key: Some("sk-your-api-key".to_string()),
///     model: "gpt-4".to_string(),
///     base_url: "https://api.openai.com/v1".to_string(),
///     max_sample_length: 2000,
///     temperature: 0.3,
///     retry_attempts: 3,
///     retry_delay_ms: 1000,
/// };
///
/// let client = AIClientFactory::create_client(&config)?;
/// // Client is ready for content analysis
/// ```
pub struct AIClientFactory;

impl AIClientFactory {
    /// Creates an AI provider instance based on the provided configuration.
    ///
    /// This method examines the `provider` field in the configuration and
    /// instantiates the appropriate AI client implementation. The returned
    /// client is fully configured and ready for use.
    ///
    /// # Arguments
    ///
    /// * `config` - AI configuration containing provider details and credentials
    ///
    /// # Returns
    ///
    /// Returns a boxed trait object implementing `AIProvider` that can be used
    /// for content analysis and subtitle matching operations.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The specified provider is not supported
    /// - Required configuration fields are missing or invalid
    /// - API credentials are invalid or missing
    /// - Network connectivity issues prevent client initialization
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use subx_cli::services::ai::AIClientFactory;
    /// use subx_cli::config::AIConfig;
    ///
    /// let config = AIConfig {
    ///     provider: "openai".to_string(),
    ///     api_key: Some("sk-key".to_string()),
    ///     model: "gpt-3.5-turbo".to_string(),
    ///     // ... other configuration
    /// };
    ///
    /// match AIClientFactory::create_client(&config) {
    ///     Ok(client) => {
    ///         // Use client for analysis
    ///         let result = client.analyze_content(request).await?;
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to create AI client: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Supported Providers
    ///
    /// Current supported providers:
    /// - `"openai"` - OpenAI GPT models with chat completion API
    pub fn create_client(config: &AIConfig) -> crate::Result<Box<dyn AIProvider>> {
        match config.provider.as_str() {
            "openai" => Ok(Box::new(OpenAIClient::from_config(config)?)),
            other => Err(SubXError::config(format!(
                "Unsupported AI provider: {}",
                other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AIConfig;

    #[test]
    fn test_ai_factory_openai_provider() {
        let config = AIConfig {
            provider: "openai".to_string(),
            api_key: Some("key".to_string()),
            model: "m".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_sample_length: 100,
            temperature: 0.1,
            retry_attempts: 1,
            retry_delay_ms: 10,
        };
        // 應成功建立 OpenAIClient 實例
        let res = AIClientFactory::create_client(&config);
        assert!(res.is_ok());
    }

    #[test]
    fn test_ai_factory_invalid_provider() {
        let config = AIConfig {
            provider: "unknown".to_string(),
            api_key: Some("key".to_string()),
            model: "m".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            max_sample_length: 100,
            temperature: 0.1,
            retry_attempts: 1,
            retry_delay_ms: 10,
        };
        // 不支援的提供商應返回錯誤
        let res = AIClientFactory::create_client(&config);
        assert!(res.is_err());
    }
}
