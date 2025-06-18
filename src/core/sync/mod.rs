//! Refactored sync module focused on VAD (Voice Activity Detection).
//!
//! Provides unified subtitle synchronization functionality using local
//! VAD (Voice Activity Detection) for voice detection and sync offset calculation.
//!
//! # Core Components
//!
//! - [`SyncEngine`] - VAD-based sync engine
//! - [`SyncMethod`] - Sync method enumeration (VAD and manual)
//! - [`SyncResult`] - Sync result structure containing offset and confidence
//!
//! # Usage
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
