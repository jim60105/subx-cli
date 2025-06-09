//! AI 客戶端工廠，用於根據配置建立對應提供商實例
use crate::config::AIConfig;
use crate::error::SubXError;
use crate::services::ai::{AIProvider, OpenAIClient};

/// AI 客戶端工廠
pub struct AIClientFactory;

impl AIClientFactory {
    /// 根據 AIConfig 建立對應的 AIProvider 實例
    pub fn create_client(config: &AIConfig) -> crate::Result<Box<dyn AIProvider>> {
        match config.provider.as_str() {
            "openai" => Ok(Box::new(OpenAIClient::from_config(config)?)),
            other => Err(SubXError::config(format!("不支援的 AI 提供商: {}", other))),
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
