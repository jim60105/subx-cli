//! AI service integration for subtitle matching and content analysis.
//!
//! This module provides AI service abstractions and data structures for
//! intelligent subtitle-video file matching using content analysis.
//!
//! # Core Components
//!
//! - `AIProvider` trait for implementing different AI service backends
//! - Request/response data structures for content analysis
//! - Confidence scoring and match verification utilities
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::services::ai::{AIProvider, AnalysisRequest};
//!
//! async fn analyze_content(provider: Box<dyn AIProvider>, request: AnalysisRequest) -> subx_cli::Result<()> {
//!     let result = provider.analyze_content(request).await?;
//!     println!("Match confidence: {}", result.confidence);
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// AI provider trait for content analysis and subtitle matching.
///
/// This trait defines the interface for AI services that can analyze
/// video and subtitle content to determine optimal matches.
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Analyze multimedia files and subtitle files for matching results.
    ///
    /// # Arguments
    ///
    /// * `request` - Analysis request containing files and content samples
    ///
    /// # Returns
    ///
    /// A `MatchResult` containing potential matches with confidence scores
    async fn analyze_content(&self, request: AnalysisRequest) -> crate::Result<MatchResult>;

    /// Verify file matching confidence.
    ///
    /// # Arguments
    ///
    /// * `verification` - Verification request for existing matches
    ///
    /// # Returns
    ///
    /// A confidence score for the verification request
    async fn verify_match(
        &self,
        verification: VerificationRequest,
    ) -> crate::Result<ConfidenceScore>;
}

/// Analysis request structure for AI content analysis.
///
/// Contains all necessary information for AI services to analyze
/// and match video files with subtitle files.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct AnalysisRequest {
    /// List of video file paths to analyze
    pub video_files: Vec<String>,
    /// List of subtitle file paths to analyze
    pub subtitle_files: Vec<String>,
    /// Content samples from subtitle files for analysis
    pub content_samples: Vec<ContentSample>,
}

/// Subtitle content sample for AI analysis.
///
/// Represents a sample of subtitle content that helps AI services
/// understand the content and context for matching purposes.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct ContentSample {
    /// Filename of the subtitle file
    pub filename: String,
    /// Preview of the subtitle content
    pub content_preview: String,
    /// Size of the subtitle file in bytes
    pub file_size: u64,
}

/// AI analysis result containing potential file matches.
///
/// The primary result structure returned by AI services containing
/// matched files with confidence scores and reasoning.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MatchResult {
    /// List of potential file matches
    pub matches: Vec<FileMatch>,
    /// Overall confidence score for the analysis (0.0 to 1.0)
    pub confidence: f32,
    /// AI reasoning explanation for the matches
    pub reasoning: String,
}

/// Individual file match information.
///
/// Represents a single video-subtitle file pairing suggested by the AI
/// with associated confidence metrics and reasoning factors.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FileMatch {
    /// Path to the matched video file
    pub video_file: String,
    /// Path to the matched subtitle file
    pub subtitle_file: String,
    /// Confidence score for this specific match (0.0 to 1.0)
    pub confidence: f32,
    /// List of factors that contributed to this match
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
pub mod factory;
pub mod openai;
pub mod prompts;
pub mod retry;

pub use cache::AICache;
pub use factory::AIClientFactory;
pub use openai::OpenAIClient;
pub use retry::{RetryConfig, retry_with_backoff};
