//! Dialogue detection module combining audio analysis and configuration.
//!
//! Provides `DialogueDetector` to extract speech segments from audio files
//! based on energy thresholds and configuration parameters.
//!
//! # Examples
//!
//! ```rust,no_run
//! use subx_cli::{init_config_manager, core::sync::dialogue::detector::DialogueDetector};
//! init_config_manager().unwrap();
//! let detector = DialogueDetector::new().unwrap();
//! // detector.detect_dialogue(&path).await;
//! ```
use crate::Result;
use crate::config::{SyncConfig, load_config};
use crate::core::sync::dialogue::{DialogueSegment, EnergyAnalyzer};
use crate::services::audio::AudioData;
use std::path::Path;

/// Dialogue detector integrating energy analysis and sync configuration.
pub struct DialogueDetector {
    energy_analyzer: EnergyAnalyzer,
    config: SyncConfig,
}

impl DialogueDetector {
    /// Creates a new `DialogueDetector` by loading sync parameters from configuration.
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

    /// Performs dialogue detection and returns a list of speech activity segments.
    pub async fn detect_dialogue(&self, audio_path: &Path) -> Result<Vec<DialogueSegment>> {
        // If not enabled, return empty list directly
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
        use crate::services::audio::{AudioAnalyzer, AusAdapter};

        // Decide whether to auto-detect sample rate based on configuration
        let sample_rate = if self.config.auto_detect_sample_rate {
            let adapter = AusAdapter::new(self.config.audio_sample_rate);
            adapter.read_audio_file(audio_path)?.sample_rate
        } else {
            self.config.audio_sample_rate
        };
        let analyzer = AudioAnalyzer::new(sample_rate);
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

    /// Calculates the ratio of speech segments to total time for activity assessment.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test speech activity ratio calculation
    #[test]
    fn test_get_speech_ratio() {
        // Initialize global configuration to allow DialogueDetector creation
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();
        let segments = vec![
            DialogueSegment::new_speech(0.0, 1.0),
            DialogueSegment::new_silence(1.0, 2.0),
            DialogueSegment::new_speech(2.0, 4.0),
        ];
        let detector = DialogueDetector::new().unwrap();
        let ratio = detector.get_speech_ratio(&segments);
        // Speech ratio is (1+2)/(1+1+2) = 3/4
        assert!((ratio - 0.75).abs() < 1e-6, "Speech ratio should be 0.75");
    }
}
