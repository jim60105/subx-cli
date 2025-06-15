use super::{LocalVadDetector, VadResult};
use crate::config::VadConfig;
use crate::core::formats::{Subtitle, SubtitleEntry};
use crate::core::sync::{SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
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
        _analysis_window_seconds: u32, // Ignore this parameter, process entire file
    ) -> Result<SyncResult> {
        // 1. Get expected start time of first subtitle
        let first_entry = self.get_first_subtitle_entry(subtitle)?;

        // 2. Perform VAD analysis on entire audio file
        let vad_result = self.vad_detector.detect_speech(audio_path).await?;

        // 3. 分析結果：比較第一個語音片段與第一句字幕的時間差
        let analysis_result = self.analyze_vad_result(&vad_result, first_entry)?;

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
        // 檢測第一個顯著的語音片段
        let first_speech_time = self.find_first_significant_speech(vad_result)?;

        // 計算偏移量：實際語音開始時間 - 預期字幕開始時間
        let expected_start = first_entry.start_time.as_secs_f64();
        let offset_seconds = first_speech_time - expected_start;

        // 計算信心度
        let confidence = self.calculate_confidence(vad_result);

        Ok(SyncResult {
            offset_seconds: offset_seconds as f32,
            confidence,
            method_used: SyncMethod::LocalVad,
            correlation_peak: 0.0,
            additional_info: Some(json!({
                "speech_segments_count": vad_result.speech_segments.len(),
                "first_speech_start": first_speech_time,
                "expected_subtitle_start": expected_start,
                "processing_time_ms": vad_result.processing_duration.as_millis(),
                "audio_duration": vad_result.audio_info.duration_seconds,
                "detected_segments": vad_result.speech_segments.iter().map(|s| {
                    json!({
                        "start": s.start_time,
                        "end": s.end_time,
                        "duration": s.duration,
                        "probability": s.probability
                    })
                }).collect::<Vec<_>>(),
            })),
            processing_duration: vad_result.processing_duration,
            warnings: Vec::new(),
        })
    }

    fn find_first_significant_speech(&self, vad_result: &VadResult) -> Result<f64> {
        // 尋找第一個顯著的語音片段
        for segment in &vad_result.speech_segments {
            // 檢查片段是否足夠長且機率足夠高
            if segment.duration >= 0.1 && segment.probability >= 0.5 {
                return Ok(segment.start_time);
            }
        }

        // 如果沒找到顯著的語音片段，但有語音片段，回傳第一個
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

        let mut confidence: f32 = 0.6; // 基礎本地 VAD 信心度

        // 基於語音片段數量調整信心度
        let segments_count = vad_result.speech_segments.len();
        if segments_count >= 1 {
            confidence += 0.1;
        }
        if segments_count >= 3 {
            confidence += 0.1;
        }

        // 基於第一個語音片段的品質調整信心度
        if let Some(first_segment) = vad_result.speech_segments.first() {
            // 較長的語音片段增加信心度
            if first_segment.duration >= 0.5 {
                confidence += 0.1;
            }
            if first_segment.duration >= 1.0 {
                confidence += 0.05;
            }

            // 較高的機率增加信心度
            if first_segment.probability >= 0.8 {
                confidence += 0.05;
            }
        }

        // 基於處理速度調整信心度（本地處理通常很快）
        if vad_result.processing_duration.as_secs() <= 1 {
            confidence += 0.05;
        }

        confidence.min(0.95_f32) // 本地 VAD 最高信心度限制為 95%
    }
}
