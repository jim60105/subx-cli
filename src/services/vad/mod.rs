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

/// Audio loading utilities for VAD operations.
///
/// Provides direct audio file loading functionality with support for
/// various audio formats and efficient memory management.
pub mod audio_loader;
/// Audio processing utilities for VAD operations.
///
/// Provides audio loading, resampling, and format conversion functionality
/// specifically designed for voice activity detection workflows.
pub mod audio_processor;
/// Voice activity detection core functionality.
///
/// Contains the main VAD detector implementation using local processing
/// to identify speech segments in audio files.
pub mod detector;
/// VAD-based subtitle synchronization.
///
/// Provides subtitle timing synchronization using voice activity detection
/// to automatically align subtitle timing with detected speech segments.
pub mod sync_detector;

pub use audio_processor::{ProcessedAudioData, VadAudioProcessor};
pub use detector::{LocalVadDetector, SpeechSegment, VadResult};
pub use sync_detector::VadSyncDetector;

// Re-export for convenience
pub use crate::config::VadConfig;
