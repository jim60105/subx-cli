//! 重構後的同步模組
//!
//! 提供多種語音檢測方法的統一字幕同步功能，包括：
//! - OpenAI Whisper API 雲端轉錄
//! - 本地 Voice Activity Detection (VAD)
//! - 自動方法選擇和回退機制

pub mod engine;

// 重新匯出主要類型
pub use engine::{MethodSelectionStrategy, SyncEngine, SyncMethod, SyncResult};

// 向後兼容性匯出（但標記為 deprecated）
#[allow(deprecated)]
pub use engine::OldSyncConfig;
