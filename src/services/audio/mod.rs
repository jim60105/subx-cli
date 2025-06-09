//! Audio processing and analysis services for SubX.
//!
//! This module provides audio analysis capabilities for subtitle synchronization
//! and dialogue detection, primarily through integration with the AUS library.
//!
//! # Core Components
//!
//! - Audio data structures for representing audio content and metadata
//! - Dialogue detection and segmentation utilities
//! - Audio feature extraction and analysis tools
//! - Integration adapters for external audio processing libraries
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::services::audio::{AudioAnalyzer, AudioData};
//!
//! let analyzer = AudioAnalyzer::new();
//! let audio_data = AudioData { /* ... */ };
//! let features = analyzer.extract_features(&audio_data).unwrap();
//! ```

pub mod aus_adapter;
pub use aus_adapter::AusAdapter;

pub mod analyzer;
pub use analyzer::{AudioFeatures, AusAudioAnalyzer, FrameFeatures};

pub mod dialogue_detector;
pub use dialogue_detector::AusDialogueDetector;

/// Audio energy envelope for waveform analysis.
///
/// Represents the amplitude envelope of an audio signal over time,
/// used for dialogue detection and synchronization analysis.
#[derive(Debug, Clone)]
pub struct AudioEnvelope {
    /// Amplitude samples of the audio envelope
    pub samples: Vec<f32>,
    /// Sample rate of the envelope data
    pub sample_rate: u32,
    /// Total duration of the audio in seconds
    pub duration: f32,
}

/// Dialogue segment detected in audio.
///
/// Represents a continuous segment of speech or dialogue
/// detected through audio analysis.
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    /// Start time of the dialogue segment in seconds
    pub start_time: f32,
    /// End time of the dialogue segment in seconds
    pub end_time: f32,
    /// Intensity or confidence level of the dialogue detection
    pub intensity: f32,
}

/// Audio metadata for raw audio data.
///
/// Contains essential metadata about audio streams including
/// format information and timing details.
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: usize,
    /// Total duration in seconds
    pub duration: f32,
}

/// Raw audio sample data.
///
/// Container for raw audio samples with associated metadata,
/// used as input for audio analysis operations.
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Raw audio samples (interleaved for multi-channel)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: usize,
    /// Total duration in seconds
    pub duration: f32,
}

/// Primary audio analyzer implementation (based on AUS).
///
/// Type alias for the main audio analyzer used throughout SubX,
/// currently implemented using the AUS (Audio Understanding Service) library.
pub type AudioAnalyzer = AusAudioAnalyzer;
