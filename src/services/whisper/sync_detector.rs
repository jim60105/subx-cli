use super::{AudioSegmentExtractor, WhisperApiClient, WhisperResponse};
use crate::config::WhisperConfig;
use crate::core::formats::{Subtitle, SubtitleEntry};
use crate::core::sync::{SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

/// Whisper API 基於雲端的同步檢測器
pub struct WhisperSyncDetector {
    client: WhisperApiClient,
    extractor: AudioSegmentExtractor,
    config: WhisperConfig,
}

impl WhisperSyncDetector {
    /// 建立檢測器
    pub fn new(api_key: String, base_url: String, config: WhisperConfig) -> Result<Self> {
        Ok(Self {
            client: WhisperApiClient::new(api_key.clone(), base_url, config.clone())?,
            extractor: AudioSegmentExtractor::new()?,
            config,
        })
    }

    /// 檢測字幕與音訊之間的偏移
    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        analysis_window_seconds: u32,
    ) -> Result<SyncResult> {
        let first = subtitle
            .entries
            .first()
            .ok_or_else(|| SubXError::audio_extraction("No subtitle entries found"))?;

        let seg = self
            .extractor
            .extract_segment(audio_path, first.start_time, analysis_window_seconds)
            .await?;
        let prep = self.extractor.prepare_for_whisper(&seg).await?;
        let transcription = self.client.transcribe(&prep).await?;

        let result = self.analyze_transcription(&transcription, first)?;

        let _ = tokio::fs::remove_file(seg).await;
        let _ = tokio::fs::remove_file(prep).await;
        Ok(result)
    }

    fn analyze_transcription(
        &self,
        transcription: &WhisperResponse,
        first_entry: &SubtitleEntry,
    ) -> Result<SyncResult> {
        let first_time = self.find_first_speech_segment(transcription)?;
        let half = Duration::from_secs(self.config.timeout_seconds as u64) / 2;
        let expected = half.as_secs_f64();
        let actual = first_time;
        let offset = actual - expected;

        let confidence = self.calculate_confidence(transcription, first_entry);
        Ok(SyncResult {
            offset_seconds: offset as f32,
            confidence,
            method_used: SyncMethod::WhisperApi,
            correlation_peak: 0.0,
            additional_info: Some(json!({
                "transcribed_text": transcription.text.trim(),
                "detected_speech_start": first_time,
                "expected_speech_start": expected,
                "subtitle_text": first_entry.text,
                "segments_count": transcription.segments.len(),
                "words_count": transcription.words.as_ref().map(|w| w.len()).unwrap_or(0),
            })),
        })
    }

    fn find_first_speech_segment(&self, transcription: &WhisperResponse) -> Result<f64> {
        if let Some(words) = &transcription.words {
            if let Some(w) = words.first() {
                return Ok(w.start);
            }
        }
        if let Some(seg) = transcription.segments.first() {
            return Ok(seg.start);
        }
        Err(SubXError::audio_extraction(
            "No speech segments found in transcription",
        ))
    }

    fn calculate_confidence(
        &self,
        transcription: &WhisperResponse,
        first_entry: &SubtitleEntry,
    ) -> f32 {
        let mut conf = 0.8;
        if !transcription.segments.is_empty() {
            conf += 0.1;
        }
        if transcription.words.is_some() {
            conf += 0.05;
        }

        let similarity = self.calculate_text_similarity(&transcription.text, &first_entry.text);
        conf += similarity * 0.05;
        conf.min(1.0)
    }

    fn calculate_text_similarity(&self, transcribed: &str, subtitle: &str) -> f32 {
        let trans_lower = transcribed.to_lowercase();
        let sub_lower = subtitle.to_lowercase();
        let a: Vec<&str> = trans_lower.split_whitespace().collect();
        let b: Vec<&str> = sub_lower.split_whitespace().collect();
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let common = b.iter().filter(|w| a.contains(w)).count();
        common as f32 / b.len() as f32
    }
}
