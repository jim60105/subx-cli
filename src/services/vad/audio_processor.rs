use crate::Result;
use crate::services::vad::audio_loader::DirectAudioLoader;
use crate::services::vad::detector::AudioInfo;
use std::path::Path;

/// Audio processor for VAD operations.
///
/// Handles loading, resampling, and format conversion of audio files
/// for voice activity detection processing.
/// Audio processor for VAD operations, optimized to use original sample rate and first channel only.
pub struct VadAudioProcessor {}

/// Processed audio data ready for VAD analysis.
///
/// Contains the audio samples and metadata after processing
/// and format conversion.
#[derive(Debug, Clone)]
pub struct ProcessedAudioData {
    /// Audio samples as 16-bit integers
    pub samples: Vec<i16>,
    /// Audio metadata and properties
    pub info: AudioInfo,
}

impl VadAudioProcessor {
    /// Create a new VAD audio processor.
    ///
    /// # Arguments
    ///
    /// * `target_sample_rate` - Desired sample rate for processing
    /// * `target_channels` - Desired number of audio channels
    ///
    /// # Returns
    ///
    /// A new `VadAudioProcessor` instance
    /// Create a new VAD audio processor.
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Load and prepare audio file for VAD processing.
    ///
    /// Performs all necessary audio processing steps including loading,
    /// resampling, and format conversion to prepare the audio for
    /// voice activity detection.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio file to process
    ///
    /// # Returns
    ///
    /// Processed audio data ready for VAD analysis
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Audio file cannot be loaded
    /// - Audio format is unsupported
    /// - Resampling fails
    /// - Format conversion fails
    ///
    /// Directly loads and prepares audio files for VAD processing, supporting multiple formats.
    /// Load and prepare audio file for VAD processing.
    ///
    /// Uses original sample rate and first channel only.
    pub async fn load_and_prepare_audio_direct(
        &self,
        audio_path: &Path,
    ) -> Result<ProcessedAudioData> {
        // 1. Load with DirectAudioLoader
        let loader = DirectAudioLoader::new()?;
        let (samples, info) = loader.load_audio_samples(audio_path)?;

        // 2. Extract first channel if multi-channel, retain original sample rate
        let mono_samples = if info.channels == 1 {
            samples
        } else {
            self.extract_first_channel(&samples, info.channels as usize)
        };
        let mono_info = AudioInfo {
            sample_rate: info.sample_rate,
            channels: 1,
            duration_seconds: info.duration_seconds,
            total_samples: mono_samples.len(),
        };
        Ok(ProcessedAudioData {
            samples: mono_samples,
            info: mono_info,
        })
    }

    // Removed resampling and multi-channel averaging methods

    /// Extract the first channel samples from interleaved multi-channel data.
    fn extract_first_channel(&self, samples: &[i16], channels: usize) -> Vec<i16> {
        samples.iter().step_by(channels).copied().collect()
    }
}
