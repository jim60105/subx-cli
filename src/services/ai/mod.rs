//! SubX AI 服務模組

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// AI 提供商 Trait 定義
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// 分析多媒體檔案和字幕檔案的匹配結果
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult>;
    /// 驗證檔案匹配的信心度
    async fn verify_match(
        &self,
        verification: VerificationRequest,
    ) -> crate::Result<ConfidenceScore>;
}

/// 分析請求結構
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct AnalysisRequest {
    pub video_files: Vec<String>,
    pub subtitle_files: Vec<String>,
    pub content_samples: Vec<ContentSample>,
}

/// 字幕內容採樣
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct ContentSample {
    pub filename: String,
    pub content_preview: String,
    pub file_size: u64,
}

/// 匹配結果
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MatchResult {
    pub matches: Vec<FileMatch>,
    pub confidence: f32,
    pub reasoning: String,
}

/// 單筆檔案匹配資訊
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FileMatch {
    pub video_file: String,
    pub subtitle_file: String,
    pub confidence: f32,
    pub match_factors: Vec<String>,
}

/// 信心度分數
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ConfidenceScore {
    pub score: f32,
    pub factors: Vec<String>,
}

/// 驗證請求結構
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct VerificationRequest {
    pub video_file: String,
    pub subtitle_file: String,
    pub match_factors: Vec<String>,
}

/// AI 使用統計資訊
#[derive(Debug, Clone)]
pub struct AiUsageStats {
    /// 使用的模型名稱
    pub model: String,
    /// Prompt tokens 使用量
    pub prompt_tokens: u32,
    /// Completion tokens 使用量
    pub completion_tokens: u32,
    /// 總 tokens 使用量
    pub total_tokens: u32,
}

/// AI 回應內容及使用統計
#[derive(Debug, Clone)]
pub struct AiResponse {
    /// 回應內容文字
    pub content: String,
    /// 使用統計資訊
    pub usage: Option<AiUsageStats>,
}

pub mod cache;
pub mod openai;
pub mod prompts;
pub mod retry;

pub use cache::AICache;
pub use openai::OpenAIClient;
pub use retry::{retry_with_backoff, RetryConfig};
