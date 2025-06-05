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
#[derive(Debug, Serialize, Clone)]
pub struct AnalysisRequest {
    pub video_files: Vec<String>,
    pub subtitle_files: Vec<String>,
    pub content_samples: Vec<ContentSample>,
}

/// 字幕內容採樣
#[derive(Debug, Serialize, Clone)]
pub struct ContentSample {
    pub filename: String,
    pub content_preview: String,
    pub file_size: u64,
    pub language_hint: Option<String>,
}

/// 匹配結果
#[derive(Debug, Deserialize, Clone)]
pub struct MatchResult {
    pub matches: Vec<FileMatch>,
    pub confidence: f32,
    pub reasoning: String,
}

/// 單筆檔案匹配資訊
#[derive(Debug, Deserialize, Clone)]
pub struct FileMatch {
    pub video_file: String,
    pub subtitle_file: String,
    pub confidence: f32,
    pub match_factors: Vec<String>,
}

/// 信心度分數
#[derive(Debug, Deserialize, Clone)]
pub struct ConfidenceScore {
    pub score: f32,
    pub factors: Vec<String>,
}

/// 驗證請求結構
#[derive(Debug, Serialize, Clone)]
pub struct VerificationRequest {
    pub video_file: String,
    pub subtitle_file: String,
    pub match_factors: Vec<String>,
}

pub mod cache;
pub mod openai;
pub mod prompts;
pub mod retry;

pub use cache::AICache;
pub use openai::OpenAIClient;
pub use retry::{retry_with_backoff, RetryConfig};
