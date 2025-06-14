//! OpenAI Whisper API 整合模組
//!
//! 提供 Whisper API 客戶端、音訊處理與同步檢測功能，實現高精度字幕同步。

mod audio_extractor;
mod client;
mod sync_detector;

pub use audio_extractor::AudioSegmentExtractor;
pub use client::{WhisperApiClient, WhisperResponse, WhisperSegment, WhisperWord};
pub use sync_detector::WhisperSyncDetector;

pub use crate::config::WhisperConfig;
