//! Refactored synchronization module.
//!
//! Provides unified subtitle synchronization functionality with multiple
//! voice detection methods, including:
//! - OpenAI Whisper API cloud transcription
//! - Local Voice Activity Detection (VAD)
//! - Automatic method selection and fallback mechanisms
//!
//! # Key Components
//!
//! - [`SyncEngine`] - Main synchronization engine with method abstraction
//! - [`SyncMethod`] - Enumeration of available synchronization methods
//! - [`SyncResult`] - Results structure with timing and confidence data
//!
//! # Usage
//!
//! ```rust
//! use subx_cli::core::sync::{SyncEngine, SyncMethod};
//! use subx_cli::config::SyncConfig;
//!
//! let engine = SyncEngine::new(SyncConfig::default())?;
//! let result = engine.detect_sync_offset(video_path, &subtitle, Some(SyncMethod::LocalVad)).await?;
//! ```

pub mod engine;

// Re-export main types
pub use engine::{MethodSelectionStrategy, SyncEngine, SyncMethod, SyncResult};

// Backward compatibility exports (marked as deprecated)
#[allow(deprecated)]
pub use engine::OldSyncConfig;
