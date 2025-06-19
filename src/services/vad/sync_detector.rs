use super::{LocalVadDetector, VadResult};
use crate::config::VadConfig;
use crate::core::formats::{Subtitle, SubtitleEntry};
use crate::core::sync::{SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
use log::debug;
use serde_json::json;
use std::path::Path;

/// VAD-based subtitle synchronization detector.
///
/// Uses Voice Activity Detection to analyze audio files and calculate
/// subtitle timing offsets by comparing detected speech segments with
/// subtitle timing information.
pub struct VadSyncDetector {
    vad_detector: LocalVadDetector,
}

impl VadSyncDetector {
    /// Create a new VAD sync detector.
    ///
    /// # Arguments
    ///
    /// * `config` - VAD configuration parameters
    ///
    /// # Returns
    ///
    /// A new `VadSyncDetector` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the VAD detector cannot be initialized
    pub fn new(config: VadConfig) -> Result<Self> {
        Ok(Self {
            vad_detector: LocalVadDetector::new(config)?,
        })
    }

    /// Detect synchronization offset between audio and subtitle.
    ///
    /// Analyzes the entire audio file using VAD to identify speech segments
    /// and compares them with subtitle timing to calculate the offset.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio file to analyze
    /// * `subtitle` - Subtitle data with timing information
    /// * `_analysis_window_seconds` - Ignored parameter (processes entire file)
    ///
    /// # Returns
    ///
    /// Synchronization result with detected offset and confidence
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Audio analysis fails
    /// - Subtitle has no entries
    /// - VAD processing fails
    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        analysis_window_seconds: u32,
    ) -> Result<SyncResult> {
        debug!(
            "[VadSyncDetector] Starting sync offset detection | audio_path: {:?}, subtitle entries: {}",
            audio_path,
            subtitle.entries.len()
        );
        // 1. Get expected start time of first subtitle
        let first_entry = self.get_first_subtitle_entry(subtitle)?;
        debug!(
            "[VadSyncDetector] First subtitle entry: start_time = {:.3}, end_time = {:.3}",
            first_entry.start_time.as_secs_f64(),
            first_entry.end_time.as_secs_f64()
        );

        // 2. 載入音訊並裁切（如有指定分析秒數）
        debug!(
            "[VadSyncDetector] Loading and cropping audio for VAD analysis: {:?}",
            audio_path
        );
        let mut audio_data = self
            .vad_detector
            .audio_processor()
            .load_and_prepare_audio_direct(audio_path)
            .await?;
        if analysis_window_seconds > 0 {
            let sample_rate = audio_data.info.sample_rate;
            let max_samples = (sample_rate as usize * analysis_window_seconds as usize)
                .min(audio_data.samples.len());
            audio_data.samples.truncate(max_samples);
            audio_data.info.duration_seconds = audio_data.samples.len() as f64 / sample_rate as f64;
            audio_data.info.total_samples = audio_data.samples.len();
            debug!(
                "[VadSyncDetector] Cropped audio to first {} seconds ({} samples)",
                analysis_window_seconds, max_samples
            );
        }

        // 3. 執行 VAD 分析
        debug!(
            "[VadSyncDetector] Performing VAD analysis on (possibly cropped) audio file: {:?}",
            audio_path
        );
        let vad_result = self
            .vad_detector
            .detect_speech_from_data(audio_data)
            .await?;
        debug!(
            "[VadSyncDetector] VAD analysis complete | speech_segments: {}, processing_time_ms: {}",
            vad_result.speech_segments.len(),
            vad_result.processing_duration.as_millis()
        );

        // 4. Analyze results: compare first speech segment with first subtitle timing
        debug!("[VadSyncDetector] Analyzing VAD result and subtitle alignment...");
        let analysis_result = self.analyze_vad_result(&vad_result, first_entry)?;

        debug!(
            "[VadSyncDetector] Sync offset detection finished | offset_seconds: {:.3}, confidence: {:.3}",
            analysis_result.offset_seconds, analysis_result.confidence
        );
        Ok(analysis_result)
    }

    fn get_first_subtitle_entry<'a>(&self, subtitle: &'a Subtitle) -> Result<&'a SubtitleEntry> {
        subtitle
            .entries
            .first()
            .ok_or_else(move || SubXError::audio_processing("No subtitle entries found"))
    }

    fn analyze_vad_result(
        &self,
        vad_result: &VadResult,
        first_entry: &SubtitleEntry,
    ) -> Result<SyncResult> {
        // Detect first significant speech segment
        let first_speech_time = self.find_first_significant_speech(vad_result)?;
        debug!(
            "[VadSyncDetector] Detected first significant speech segment: first_speech_time = {:.3} (seconds)",
            first_speech_time
        );
        debug!(
            "[VadSyncDetector] Speech segments count: {} | First segment: start = {:.3}, duration = {:.3}",
            vad_result.speech_segments.len(),
            vad_result
                .speech_segments
                .first()
                .map(|s| s.start_time)
                .unwrap_or(-1.0),
            vad_result
                .speech_segments
                .first()
                .map(|s| s.duration)
                .unwrap_or(-1.0)
        );

        // Calculate offset: actual speech start time - expected subtitle start time
        let expected_start = first_entry.start_time.as_secs_f64();
        debug!(
            "[VadSyncDetector] Expected subtitle start time: expected_start = {:.3} (seconds)",
            expected_start
        );
        let offset_seconds = first_speech_time - expected_start;
        debug!(
            "[VadSyncDetector] Calculated offset_seconds = {:.3} (speech - subtitle)",
            offset_seconds
        );

        // Calculate confidence
        let confidence = self.calculate_confidence(vad_result);
        debug!(
            "[VadSyncDetector] Calculated confidence score: {:.3}",
            confidence
        );

        let additional_info = Some(json!({
            "speech_segments_count": vad_result.speech_segments.len(),
            "first_speech_start": first_speech_time,
            "expected_subtitle_start": expected_start,
            "processing_time_ms": vad_result.processing_duration.as_millis(),
            "audio_duration": vad_result.audio_info.duration_seconds,
            "detected_segments": vad_result.speech_segments.iter().map(|s| {
                json!({
                    "start": s.start_time,
                    "end": s.end_time,
                    "duration": s.duration
                })
            }).collect::<Vec<_>>(),
        }));

        Ok(SyncResult {
            offset_seconds: offset_seconds as f32,
            confidence,
            method_used: SyncMethod::LocalVad,
            correlation_peak: 0.0,
            additional_info,
            processing_duration: vad_result.processing_duration,
            warnings: Vec::new(),
        })
    }

    fn find_first_significant_speech(&self, vad_result: &VadResult) -> Result<f64> {
        // Find the first significant speech segment
        for segment in &vad_result.speech_segments {
            // Check if segment is long enough
            if segment.duration >= 0.1 {
                return Ok(segment.start_time);
            }
        }

        // If no significant speech segment found but speech segments exist, return first one
        if let Some(first_segment) = vad_result.speech_segments.first() {
            return Ok(first_segment.start_time);
        }

        Err(SubXError::audio_processing(
            "No significant speech segments found in audio",
        ))
    }

    fn calculate_confidence(&self, vad_result: &VadResult) -> f32 {
        if vad_result.speech_segments.is_empty() {
            return 0.0;
        }

        let mut confidence: f32 = 0.6; // Base local VAD confidence

        // Adjust confidence based on speech segment count
        let segments_count = vad_result.speech_segments.len();
        if segments_count >= 1 {
            confidence += 0.1;
        }
        if segments_count >= 3 {
            confidence += 0.1;
        }

        // Adjust confidence based on first speech segment quality
        if let Some(first_segment) = vad_result.speech_segments.first() {
            // Longer speech segments increase confidence
            if first_segment.duration >= 0.5 {
                confidence += 0.1;
            }
            if first_segment.duration >= 1.0 {
                confidence += 0.05;
            }
        }

        // Adjust confidence based on processing speed (local processing is usually fast)
        if vad_result.processing_duration.as_secs() <= 1 {
            confidence += 0.05;
        }

        confidence.min(0.95_f32) // Local VAD maximum confidence limit is 95%
    }
}
