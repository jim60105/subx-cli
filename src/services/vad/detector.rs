use super::audio_processor::VadAudioProcessor;
use crate::config::VadConfig;
use crate::{Result, error::SubXError};
use log::{debug, trace, warn};
use std::time::{Duration, Instant};
use voice_activity_detector::{IteratorExt, LabeledAudio, VoiceActivityDetector};

/// Local voice activity detector.
///
/// Provides voice activity detection using local processing without
/// external API calls. Uses the `voice_activity_detector` crate for
/// speech detection and analysis.
pub struct LocalVadDetector {
    config: VadConfig,
    audio_processor: VadAudioProcessor,
}

impl LocalVadDetector {
    /// Create a new local VAD detector.
    ///
    /// # Arguments
    ///
    /// * `config` - VAD configuration parameters
    ///
    /// # Returns
    ///
    /// A new `LocalVadDetector` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the audio processor cannot be initialized
    pub fn new(config: VadConfig) -> Result<Self> {
        debug!("Initializing LocalVadDetector with config: {:?}", config);
        Ok(Self {
            config,
            audio_processor: VadAudioProcessor::new()?,
        })
    }

    /// Detect speech activity in a ProcessedAudioData (for partial audio analysis).
    ///
    /// # Arguments
    ///
    /// * `audio_data` - Pre-processed audio data (can be cropped)
    ///
    /// # Returns
    ///
    /// VAD analysis results including speech segments and metadata
    pub async fn detect_speech_from_data(
        &self,
        mut audio_data: crate::services::vad::audio_processor::ProcessedAudioData,
    ) -> Result<VadResult> {
        debug!(
            "Starting speech detection for ProcessedAudioData: sample_rate={}, duration={}",
            audio_data.info.sample_rate, audio_data.info.duration_seconds
        );
        // New: return error directly if audio data is empty
        if audio_data.samples.is_empty() {
            return Err(SubXError::audio_processing(
                "Audio data is empty".to_string(),
            ));
        }
        let start_time = Instant::now();

        // 1.5. Resample if needed (always to 16000 if not 8000/16000)
        if audio_data.info.sample_rate != 8000 && audio_data.info.sample_rate != 16000 {
            debug!(
                "Resampling audio from {}Hz to 16000Hz...",
                audio_data.info.sample_rate
            );
            use crate::services::vad::resample::resample_to_target_rate;
            let resampled =
                resample_to_target_rate(&audio_data.samples, audio_data.info.sample_rate, 16000)?;
            let new_len = resampled.len();
            audio_data.samples = resampled;
            audio_data.info.sample_rate = 16000;
            audio_data.info.duration_seconds = new_len as f64 / 16000.0;
            audio_data.info.total_samples = new_len;
            debug!(
                "Resampling complete: new sample_rate=16000, total_samples={}, duration={:.3}s",
                new_len, audio_data.info.duration_seconds
            );
        }

        // 2. Calculate chunk size and create VAD with actual sample rate
        let chunk_size = self.calculate_chunk_size(audio_data.info.sample_rate);
        debug!(
            "Calculated VAD chunk_size={} for sample_rate={}",
            chunk_size, audio_data.info.sample_rate
        );
        let vad = VoiceActivityDetector::builder()
            .sample_rate(audio_data.info.sample_rate)
            .chunk_size(chunk_size)
            .build()
            .map_err(|e| {
                warn!("Failed to create VAD instance: {}", e);
                SubXError::audio_processing(format!("Failed to create VAD: {}", e))
            })?;

        // 3. Execute speech detection
        trace!("Running speech segment detection");
        let speech_segments =
            self.detect_speech_segments(vad, &audio_data.samples, audio_data.info.sample_rate)?;

        let processing_duration = start_time.elapsed();
        debug!(
            "Speech detection completed in {:?} seconds, segments found: {}",
            processing_duration,
            speech_segments.len()
        );

        Ok(VadResult {
            speech_segments,
            processing_duration,
            audio_info: audio_data.info,
        })
    }

    fn detect_speech_segments(
        &self,
        mut vad: VoiceActivityDetector,
        samples: &[i16],
        sample_rate: u32,
    ) -> Result<Vec<SpeechSegment>> {
        trace!(
            "Detecting speech segments: samples={}, sample_rate={}",
            samples.len(),
            sample_rate
        );
        let mut segments = Vec::new();
        let chunk_size = self.calculate_chunk_size(sample_rate);
        let chunk_duration_seconds = chunk_size as f64 / sample_rate as f64;

        // Use label functionality to identify speech and non-speech segments
        let vad_threshold = 1.0 - self.config.sensitivity;
        debug!(
            "VAD threshold set to {} (sensitivity={})",
            vad_threshold, self.config.sensitivity
        );
        let labels: Vec<LabeledAudio<i16>> = samples
            .iter()
            .copied()
            .label(&mut vad, vad_threshold, self.config.padding_chunks as usize)
            .collect();
        trace!("Labeling complete, total chunks: {}", labels.len());

        let mut current_speech_start: Option<f64> = None;
        let mut chunk_index = 0;

        for label in labels {
            let chunk_start_time = chunk_index as f64 * chunk_duration_seconds;
            match label {
                LabeledAudio::Speech(_chunk) => {
                    if current_speech_start.is_none() {
                        trace!(
                            "Speech started at {:.3}s (chunk #{})",
                            chunk_start_time, chunk_index
                        );
                        current_speech_start = Some(chunk_start_time);
                    }
                }
                LabeledAudio::NonSpeech(_chunk) => {
                    if let Some(start_time) = current_speech_start.take() {
                        let end_time = chunk_start_time;
                        let duration = end_time - start_time;
                        trace!(
                            "Speech ended at {:.3}s (duration {:.3}s)",
                            end_time, duration
                        );
                        // Filter out speech segments that are too short
                        if duration >= self.config.min_speech_duration_ms as f64 / 1000.0 {
                            trace!(
                                "Detected speech segment: start={:.3}s, end={:.3}s, duration={:.3}s",
                                start_time, end_time, duration
                            );
                            segments.push(SpeechSegment {
                                start_time,
                                end_time,
                                duration,
                            });
                        } else {
                            trace!(
                                "Discarded short segment: start={:.3}s, end={:.3}s, duration={:.3}s",
                                start_time, end_time, duration
                            );
                        }
                    }
                }
            }
            chunk_index += 1;
        }

        // Handle the last speech segment (if exists)
        if let Some(start_time) = current_speech_start {
            let end_time = chunk_index as f64 * chunk_duration_seconds;
            let duration = end_time - start_time;
            trace!(
                "Final speech segment: start={:.3}s, end={:.3}s, duration={:.3}s",
                start_time, end_time, duration
            );
            if duration >= self.config.min_speech_duration_ms as f64 / 1000.0 {
                trace!(
                    "Detected speech segment: start={:.3}s, end={:.3}s, duration={:.3}s",
                    start_time, end_time, duration
                );
                segments.push(SpeechSegment {
                    start_time,
                    end_time,
                    duration,
                });
            } else {
                trace!(
                    "Discarded short final segment: start={:.3}s, end={:.3}s, duration={:.3}s",
                    start_time, end_time, duration
                );
            }
        }

        debug!("Speech segments detected: {}", segments.len());
        Ok(segments)
    }

    /// Dynamically calculates the optimal VAD chunk size for a given audio sample rate using the Silero VAD V5 model.
    ///
    /// This function selects a chunk size (in samples) compatible with the Silero VAD V5 model's strict requirements.
    /// For 8 kHz audio, only a 256-sample window is supported. For 16 kHz audio, only a 512-sample window is supported.
    /// For sample rates that are a multiple of 16 kHz (e.g., 32 kHz, 48 kHz), a 512-sample window is also used, as required by the model.
    ///
    /// # Arguments
    ///
    /// - `sample_rate`: The audio sample rate in Hz (e.g., 8000, 16000)
    ///
    /// # Returns
    ///
    /// The chunk size in number of samples, as required by the Silero VAD V5 model.
    ///
    /// # Model Reference
    ///
    /// This logic follows the requirements of the [Silero VAD V5 model.](https://github.com/snakers4/silero-vad/releases/tag/v5.0)
    ///
    /// # Panics
    ///
    /// This function will panic if the sample rate is not supported by the model.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::services::vad::LocalVadDetector;
    /// let detector = LocalVadDetector::new(Default::default()).unwrap();
    /// assert_eq!(detector.calculate_chunk_size(8000), 256);
    /// assert_eq!(detector.calculate_chunk_size(16000), 512);
    /// ```
    pub fn calculate_chunk_size(&self, sample_rate: u32) -> usize {
        trace!("Calculating chunk size for sample_rate={}", sample_rate);
        let chunk_size = match sample_rate {
            8000 => 256,
            16000 => 512,
            _ => panic!(
                "Unsupported VAD sample_rate={}. Only 8kHz/256, 16kHz/512 are allowed.",
                sample_rate
            ),
        };
        debug!(
            "Final chunk_size for sample_rate {}: {}",
            sample_rate, chunk_size
        );
        chunk_size
    }

    /// Get the internal VadAudioProcessor instance (for advanced use, e.g. partial audio cropping)
    pub fn audio_processor(&self) -> &VadAudioProcessor {
        &self.audio_processor
    }
}

/// VAD detection result containing speech segments and processing metadata.
///
/// Represents the complete result of voice activity detection analysis,
/// including identified speech segments, timing information, and audio metadata.
#[derive(Debug, Clone)]
pub struct VadResult {
    /// Detected speech segments with timing and confidence
    pub speech_segments: Vec<SpeechSegment>,
    /// Time taken to process the audio file
    pub processing_duration: Duration,
    /// Original audio file information
    pub audio_info: AudioInfo,
}

/// Individual speech segment identified by VAD.
///
/// Represents a continuous segment of detected speech with timing
/// and confidence information.
#[derive(Debug, Clone)]
pub struct SpeechSegment {
    /// Start time of the speech segment in seconds
    pub start_time: f64,
    /// End time of the speech segment in seconds
    pub end_time: f64,
    /// Duration of the speech segment in seconds
    pub duration: f64,
}

/// Audio file metadata and properties.
///
/// Contains technical information about the processed audio file
/// including format, duration, and sample information.
#[derive(Debug, Clone)]
pub struct AudioInfo {
    /// Audio sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u16,
    /// Total duration of audio in seconds
    pub duration_seconds: f64,
    /// Total number of audio samples
    pub total_samples: usize,
}
