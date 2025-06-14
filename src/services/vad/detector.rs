use super::audio_processor::VadAudioProcessor;
use crate::config::VadConfig;
use crate::{Result, error::SubXError};
use std::path::Path;
use std::time::{Duration, Instant};
use voice_activity_detector::{IteratorExt, LabeledAudio, VoiceActivityDetector};

/// 本地語音活動檢測器
pub struct LocalVadDetector {
    config: VadConfig,
    audio_processor: VadAudioProcessor,
}

impl LocalVadDetector {
    pub fn new(config: VadConfig) -> Result<Self> {
        Ok(Self {
            config,
            audio_processor: VadAudioProcessor::new(config.sample_rate, 1)?,
        })
    }

    /// 檢測音訊檔案中的語音活動
    pub async fn detect_speech(&self, audio_path: &Path) -> Result<VadResult> {
        let start_time = Instant::now();

        // 1. 載入和預處理音訊
        let audio_data = self
            .audio_processor
            .load_and_prepare_audio(audio_path)
            .await?;

        // 2. 建立 VAD 實例
        let mut vad = VoiceActivityDetector::builder()
            .sample_rate(self.config.sample_rate)
            .chunk_size(self.config.chunk_size)
            .build()
            .map_err(|e| SubXError::audio_processing(format!("Failed to create VAD: {}", e)))?;

        // 3. 執行語音檢測
        let speech_segments = self.detect_speech_segments(&mut vad, &audio_data.samples)?;

        let processing_duration = start_time.elapsed();

        Ok(VadResult {
            speech_segments,
            processing_duration,
            audio_info: audio_data.info,
        })
    }

    fn detect_speech_segments(
        &self,
        vad: &mut VoiceActivityDetector,
        samples: &[i16],
    ) -> Result<Vec<SpeechSegment>> {
        let mut segments = Vec::new();
        let chunk_duration_seconds = self.config.chunk_size as f64 / self.config.sample_rate as f64;

        // 使用 label 功能來標識語音和非語音片段
        let labels: Vec<LabeledAudio<i16>> =
            samples
                .iter()
                .copied()
                .label(vad, self.config.sensitivity, self.config.padding_chunks);

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

                        // 過濾太短的語音片段
                        if duration >= self.config.min_speech_duration_ms as f64 / 1000.0 {
                            segments.push(SpeechSegment {
                                start_time,
                                end_time,
                                probability: self.config.sensitivity, // 使用配置的敏感度作為機率
                                duration,
                            });
                        }
                    }
                }
            }

            chunk_index += 1;
        }

        // 處理最後一個語音片段（如果存在）
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

        // 合併相近的語音片段
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
                // 合併片段
                current.end_time = segment.end_time;
                current.duration = current.end_time - current.start_time;
                current.probability = current.probability.max(segment.probability);
            } else {
                // 儲存當前片段，開始新片段
                merged.push(current);
                current = segment;
            }
        }

        merged.push(current);
        merged
    }
}

#[derive(Debug, Clone)]
pub struct VadResult {
    pub speech_segments: Vec<SpeechSegment>,
    pub processing_duration: Duration,
    pub audio_info: AudioInfo,
}

#[derive(Debug, Clone)]
pub struct SpeechSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub probability: f32,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f64,
    pub total_samples: usize,
}
