//! Local Voice Activity Detection (VAD) module.
//!
//! This module provides local voice detection functionality based on the
//! `voice_activity_detector` crate, enabling fast and private voice activity
//! detection and subtitle synchronization in local environments.
//!
//! # Key Components
//!
//! - [`VadAudioProcessor`] - Audio processing and format conversion
//! - [`LocalVadDetector`] - Core VAD detection functionality  
//! - [`VadSyncDetector`] - Subtitle synchronization using VAD
//!
//! # Features
//!
//! - Local audio processing without external API calls
//! - Configurable sensitivity and processing parameters
//! - Support for multiple audio formats and sample rates
//! - Privacy-focused design with no data transmission

mod audio_processor;
mod detector;
mod sync_detector;

pub use audio_processor::{ProcessedAudioData, VadAudioProcessor};
pub use detector::{AudioInfo, LocalVadDetector, SpeechSegment, VadResult};
pub use sync_detector::VadSyncDetector;

// Re-export for convenience
pub use crate::config::VadConfig;
