//! Free AI provider using obfuscated OpenRouter API key and built-in free model.

use crate::config::AIConfig;
use crate::error::SubXError;
use crate::services::ai::openrouter::OpenRouterClient;
use crate::services::ai::{
    AIProvider, AnalysisRequest, ConfidenceScore, MatchResult, VerificationRequest,
};

/// Free AI provider using obfuscated OpenRouter API key and built-in free model.
#[derive(Debug)]
pub struct FreeProvider {
    openrouter_client: OpenRouterClient,
}

impl FreeProvider {
    /// Get the compile-time obfuscated OpenRouter API key.
    fn get_api_key() -> String {
        // Read the OPENROUTER_KEY environment variable at compile time if set, else fallback to empty
        option_env!("OPENROUTER_KEY").unwrap_or("").to_string()
    }

    /// Hardcoded base URL for the free provider (immutable).
    const HARDCODED_BASE_URL: &'static str = "https://openrouter.ai/api/v1";
    /// Hardcoded model for the free provider (immutable).
    const HARDCODED_MODEL: &'static str = "deepseek/deepseek-r1-0528:free";
    /// Hardcoded temperature for the free provider (immutable).
    const HARDCODED_TEMPERATURE: f32 = 0.3;
    /// Hardcoded max tokens for the free provider (immutable).
    const HARDCODED_MAX_TOKENS: u32 = 10000;

    /// Display usage notice and BYOK suggestions.
    fn display_usage_notice() {
        eprintln!("📢 注意：您正在使用 free AI 提供商");
        eprintln!(
            "   • 此服務基於 OpenRouter 免費模型 ({})",
            Self::HARDCODED_MODEL
        );
        eprintln!("   • 使用此服務表示您同意 OpenRouter 使用條款：https://openrouter.ai/terms");
        eprintln!("   • 雖然開發者無意記錄您的訊息，但您的內容可能被 OpenRouter 及其協力者看見");
        eprintln!("   • 建議使用自己的 API Key (BYOK) 以獲得更好的隱私保護：");
        eprintln!("     export OPENAI_API_KEY=\"your-api-key\"");
        eprintln!("     subx-cli config set ai.provider openai");
        eprintln!("     subx-cli config set ai.model \"gpt-4o-mini\"");
        eprintln!();
    }

    /// Validate and ignore mutable config values for the free provider.
    fn validate_config_immutability(config: &AIConfig) -> Result<(), SubXError> {
        if config.provider == "free" {
            if !config.base_url.is_empty() && config.base_url != Self::HARDCODED_BASE_URL {
                eprintln!("⚠️  警告：free 提供商不支援自訂 base_url，將使用預設值");
            }
            if config.model != Self::HARDCODED_MODEL {
                eprintln!("⚠️  警告：free 提供商不支援自訂 model，將使用預設免費模型");
            }
        }
        Ok(())
    }

    /// Create a new FreeProvider from configuration.
    pub fn from_config(config: &AIConfig) -> Result<Self, SubXError> {
        Self::validate_config_immutability(config)?;
        Self::display_usage_notice();
        let client = OpenRouterClient::new_with_base_url_and_timeout(
            Self::get_api_key(),
            Self::HARDCODED_MODEL.to_string(),
            Self::HARDCODED_TEMPERATURE,
            Self::HARDCODED_MAX_TOKENS,
            config.retry_attempts,
            config.retry_delay_ms,
            Self::HARDCODED_BASE_URL.to_string(),
            config.request_timeout_seconds,
        );
        Ok(Self {
            openrouter_client: client,
        })
    }
}

#[async_trait::async_trait]
impl AIProvider for FreeProvider {
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult> {
        Self::display_usage_notice();
        self.openrouter_client.analyze_content(request).await
    }

    async fn verify_match(
        &self,
        verification: VerificationRequest,
    ) -> crate::Result<ConfidenceScore> {
        Self::display_usage_notice();
        self.openrouter_client.verify_match(verification).await
    }
}
