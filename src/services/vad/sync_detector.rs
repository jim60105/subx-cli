use super::{LocalVadDetector, VadResult};
use crate::config::VadConfig;
use crate::core::formats::{Subtitle, SubtitleEntry};
use crate::core::sync::{SyncMethod, SyncResult};
use crate::services::whisper::AudioSegmentExtractor;
use crate::{Result, error::SubXError};
use serde_json::json;
use std::path::Path;

pub struct VadSyncDetector {
    vad_detector: LocalVadDetector,
    audio_extractor: AudioSegmentExtractor,
}

impl VadSyncDetector {
    pub fn new(config: VadConfig) -> Result<Self> {
        Ok(Self {
            vad_detector: LocalVadDetector::new(config)?,
            audio_extractor: AudioSegmentExtractor::new()?,
        })
    }

    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        analysis_window_seconds: u32,
    ) -> Result<SyncResult> {
        // 1. 獲取第一句字幕
        let first_entry = self.get_first_subtitle_entry(subtitle)?;

        // 2. 提取音訊片段
        let audio_segment_path = self
            .audio_extractor
            .extract_segment(audio_path, first_entry.start_time, analysis_window_seconds)
            .await?;

        // 3. 使用 VAD 檢測語音活動
        let vad_result = self.vad_detector.detect_speech(&audio_segment_path).await?;

        // 4. 分析檢測結果
        let analysis_result =
            self.analyze_vad_result(&vad_result, first_entry, analysis_window_seconds)?;

        // 5. 清理臨時檔案
        let _ = tokio::fs::remove_file(&audio_segment_path).await;

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
        _first_entry: &SubtitleEntry,
        analysis_window_seconds: u32,
    ) -> Result<SyncResult> {
        // 檢測第一個顯著的語音片段
        let first_speech_time = self.find_first_significant_speech(vad_result)?;

        // 計算偏移量
        // 由於我們提取的音訊片段是以第一句字幕為中心的指定秒數片段
        // 所以需要計算相對於片段開始的實際時間
        let half_window = analysis_window_seconds as f64 / 2.0;
        let expected_position_in_segment = half_window;
        let actual_position_in_segment = first_speech_time;

        let offset_seconds = actual_position_in_segment - expected_position_in_segment;

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
                "expected_speech_start": expected_position_in_segment,
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
