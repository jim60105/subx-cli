//! 重構後的同步模組，專注於 VAD 語音檢測
//!
//! 提供統一的字幕同步功能，使用本地 VAD (Voice Activity Detection)
//! 進行語音檢測和同步偏移計算。
//!
//! # 核心組件
//!
//! - [`SyncEngine`] - 基於 VAD 的同步引擎
//! - [`SyncMethod`] - 同步方法枚舉（VAD 和手動）
//! - [`SyncResult`] - 同步結果結構，包含偏移量和信心度
//!
//! # 用法
//!
//! ```no_run
//! use subx_cli::core::sync::{SyncEngine, SyncMethod};
//! use subx_cli::config::SyncConfig;
//! use std::path::Path;
//! use subx_cli::core::formats::{Subtitle, SubtitleFormatType, SubtitleMetadata};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let engine = SyncEngine::new(SyncConfig::default())?;
//! let video_path = Path::new("video.mp4");
//! let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
//! let subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);
//! let result = engine.detect_sync_offset(video_path, &subtitle, Some(SyncMethod::LocalVad)).await?;
//! # Ok(())
//! # }
//! ```

pub mod engine;

// Re-export main types
pub use engine::{MethodSelectionStrategy, SyncEngine, SyncMethod, SyncResult};

// Backward compatibility exports (marked as deprecated)
#[allow(deprecated)]
pub use engine::OldSyncConfig;
