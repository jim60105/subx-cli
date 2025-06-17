use super::audio_processor::VadAudioProcessor;
use crate::config::VadConfig;
use crate::{Result, error::SubXError};
use std::path::Path;
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
        // Clone config to avoid moving while initializing audio_processor
        let cfg_clone = config.clone();
        Ok(Self {
            config,
            audio_processor: VadAudioProcessor::new(cfg_clone.sample_rate, 1)?,
        })
    }

    /// Detect speech activity in an audio file.
    ///
    /// Processes the entire audio file to identify speech segments
    /// with timestamps and confidence scores.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio file to analyze
    ///
    /// # Returns
    ///
    /// VAD analysis results including speech segments and metadata
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Audio file cannot be loaded
    /// - VAD processing fails
    /// - Audio format is unsupported
    pub async fn detect_speech(&self, audio_path: &Path) -> Result<VadResult> {
        let start_time = Instant::now();

        // 1. Load and preprocess audio
        let audio_data = self
            .audio_processor
            .load_and_prepare_audio_direct(audio_path)
            .await?;

        // 2. Create VAD instance
        let vad = VoiceActivityDetector::builder()
            .sample_rate(self.config.sample_rate)
            .chunk_size(self.config.chunk_size)
            .build()
            .map_err(|e| SubXError::audio_processing(format!("Failed to create VAD: {}", e)))?;

        // 3. Execute speech detection
        let speech_segments = self.detect_speech_segments(vad, &audio_data.samples)?;

        let processing_duration = start_time.elapsed();

        Ok(VadResult {
            speech_segments,
            processing_duration,
            audio_info: audio_data.info,
        })
    }

    fn detect_speech_segments(
        &self,
        vad: VoiceActivityDetector,
        samples: &[i16],
    ) -> Result<Vec<SpeechSegment>> {
        let mut segments = Vec::new();
        let chunk_duration_seconds = self.config.chunk_size as f64 / self.config.sample_rate as f64;

        // Use label functionality to identify speech and non-speech segments
        let labels: Vec<LabeledAudio<i16>> = samples
            .iter()
            .copied()
            .label(
                vad,
                self.config.sensitivity,
                self.config.padding_chunks as usize,
            )
            .collect();

        let mut current_speech_start: Option<f64> = None;
        let mut chunk_index = 0;

        for label in labels {
            let chunk_start_time = chunk_index as f64 * chunk_duration_seconds;

            match label {
                LabeledAudio::Speech(_chunk) => {
                    if current_speech_start.is_none() {
                        current_speech_start = Some(chunk_start_time);
                    }
                }
                LabeledAudio::NonSpeech(_chunk) => {
                    if let Some(start_time) = current_speech_start.take() {
                        let end_time = chunk_start_time;
                        let duration = end_time - start_time;

                        // Filter out speech segments that are too short
                        if duration >= self.config.min_speech_duration_ms as f64 / 1000.0 {
                            segments.push(SpeechSegment {
                                start_time,
                                end_time,
                                probability: self.config.sensitivity, // Use configured sensitivity as probability
                                duration,
                            });
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

            if duration >= self.config.min_speech_duration_ms as f64 / 1000.0 {
                segments.push(SpeechSegment {
                    start_time,
                    end_time,
                    probability: self.config.sensitivity,
                    duration,
                });
            }
        }

        // Merge close segments
        Ok(self.merge_close_segments(segments))
    }

    fn merge_close_segments(&self, segments: Vec<SpeechSegment>) -> Vec<SpeechSegment> {
        if segments.is_empty() {
            return segments;
        }

        let mut merged = Vec::new();
        let mut current = segments[0].clone();
        let merge_threshold = self.config.speech_merge_gap_ms as f64 / 1000.0;

        for segment in segments.into_iter().skip(1) {
            if segment.start_time - current.end_time <= merge_threshold {
                // Merge segments
                current.end_time = segment.end_time;
                current.duration = current.end_time - current.start_time;
                current.probability = current.probability.max(segment.probability);
            } else {
                // Store current segment, start new segment
                merged.push(current);
                current = segment;
            }
        }

        merged.push(current);
        merged
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
    /// Confidence probability of speech detection (0.0-1.0)
    pub probability: f32,
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
