//! 本地語音活動檢測 (Voice Activity Detection) 模組
//!
//! 此模組提供基於 `voice_activity_detector` crate 的本地語音檢測功能，
//! 用於在本地環境中進行快速、私密的語音活動檢測和字幕同步。

mod audio_processor;
mod detector;
mod sync_detector;

pub use audio_processor::{ProcessedAudioData, VadAudioProcessor};
pub use detector::{AudioInfo, LocalVadDetector, SpeechSegment, VadResult};
pub use sync_detector::VadSyncDetector;

// Re-export for convenience
pub use crate::config::VadConfig;
