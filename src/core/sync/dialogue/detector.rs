use crate::config::{load_config, SyncConfig};
use crate::core::sync::dialogue::{DialogueSegment, EnergyAnalyzer};
use crate::services::audio::AudioData;
use crate::Result;
use std::path::Path;

/// 主對話檢測器，整合能量分析與配置
pub struct DialogueDetector {
    energy_analyzer: EnergyAnalyzer,
    config: SyncConfig,
}

impl DialogueDetector {
    /// 建立對話檢測器，從配置讀取參數
    pub fn new() -> Result<Self> {
        let config = load_config()?.sync;
        let energy_analyzer = EnergyAnalyzer::new(
            config.dialogue_detection_threshold,
            config.min_dialogue_duration_ms,
        );
        Ok(Self {
            energy_analyzer,
            config,
        })
    }

    /// 執行對話檢測，回傳語音活動片段清單
    pub async fn detect_dialogue(&self, audio_path: &Path) -> Result<Vec<DialogueSegment>> {
        // 若未啟用，直接回傳空列表
        if !self.config.enable_dialogue_detection {
            return Ok(Vec::new());
        }
        let audio_data = self.load_audio(audio_path).await?;
        let segments = self
            .energy_analyzer
            .analyze(&audio_data.samples, audio_data.sample_rate);
        Ok(self.optimize_segments(segments))
    }

    async fn load_audio(&self, audio_path: &Path) -> Result<AudioData> {
        use crate::services::audio::AudioAnalyzer;
        let analyzer = AudioAnalyzer::new(self.config.audio_sample_rate);
        analyzer.load_audio_data(audio_path).await
    }

    fn optimize_segments(&self, segments: Vec<DialogueSegment>) -> Vec<DialogueSegment> {
        let mut optimized = Vec::new();
        let mut current: Option<DialogueSegment> = None;
        let gap = self.config.dialogue_merge_gap_ms as f64 / 1000.0;
        for seg in segments {
            if let Some(mut prev) = current.take() {
                if prev.is_speech && seg.is_speech && seg.start_time - prev.end_time < gap {
                    prev.end_time = seg.end_time;
                    current = Some(prev);
                } else {
                    optimized.push(prev);
                    current = Some(seg);
                }
            } else {
                current = Some(seg);
            }
        }
        if let Some(last) = current {
            optimized.push(last);
        }
        optimized
    }

    /// 計算語音佔比，以評估語音活動程度
    pub fn get_speech_ratio(&self, segments: &[DialogueSegment]) -> f32 {
        let total: f64 = segments.iter().map(|s| s.duration()).sum();
        let speech: f64 = segments
            .iter()
            .filter(|s| s.is_speech)
            .map(|s| s.duration())
            .sum();
        if total > 0.0 {
            (speech / total) as f32
        } else {
            0.0
        }
    }
}
