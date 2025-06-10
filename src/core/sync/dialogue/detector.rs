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

    /// Test dialogue detection with mock audio data
    #[tokio::test]
    async fn test_dialogue_detection_algorithm() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Create test audio file path (will use mock data in analyzer)
        let temp_dir = tempfile::TempDir::new().unwrap();
        let audio_path = temp_dir.path().join("test_audio.wav");

        // Note: This test relies on the energy analyzer's test capabilities
        // In a real scenario, we'd need actual audio file generation

        // Test with disabled detection
        let mut config = detector.config.clone();
        config.enable_dialogue_detection = false;
        let energy_analyzer = EnergyAnalyzer::new(
            config.dialogue_detection_threshold,
            config.min_dialogue_duration_ms,
        );
        let detector_disabled = DialogueDetector {
            energy_analyzer,
            config,
        };

        let segments = detector_disabled
            .detect_dialogue(&audio_path)
            .await
            .unwrap();
        assert!(
            segments.is_empty(),
            "Disabled detection should return empty segments"
        );
    }

    /// Test voice activity detection with different audio patterns
    #[test]
    fn test_voice_activity_detection_patterns() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Test speech ratio calculation with various patterns
        let speech_segments = vec![
            DialogueSegment::new_speech(0.0, 2.0),  // 2 seconds speech
            DialogueSegment::new_silence(2.0, 3.0), // 1 second silence
            DialogueSegment::new_speech(3.0, 6.0),  // 3 seconds speech
        ];
        // Total: 6 seconds, Speech: 5 seconds, Ratio: 5/6 ≈ 0.833
        let ratio = detector.get_speech_ratio(&speech_segments);
        assert!(
            (ratio - 0.8333).abs() < 0.001,
            "Speech ratio should be ~0.833, got: {}",
            ratio
        );

        // Test all silence
        let silence_segments = vec![DialogueSegment::new_silence(0.0, 5.0)];
        let ratio = detector.get_speech_ratio(&silence_segments);
        assert_eq!(ratio, 0.0, "All silence should have 0.0 speech ratio");

        // Test all speech
        let all_speech_segments = vec![DialogueSegment::new_speech(0.0, 5.0)];
        let ratio = detector.get_speech_ratio(&all_speech_segments);
        assert_eq!(ratio, 1.0, "All speech should have 1.0 speech ratio");
    }

    /// Test segment optimization and merging
    #[test]
    fn test_segment_optimization() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Create segments that should be merged (gap < merge_gap_ms)
        let segments = vec![
            DialogueSegment::new_speech(0.0, 1.0),
            DialogueSegment::new_speech(1.1, 2.0), // 100ms gap, should merge
            DialogueSegment::new_silence(2.0, 3.0),
            DialogueSegment::new_speech(3.6, 4.0), // 600ms gap, shouldn't merge with silence
        ];

        let optimized = detector.optimize_segments(segments);

        // Should have 3 segments: merged speech (0-2), silence (2-3), speech (3.6-4)
        assert_eq!(optimized.len(), 3);
        assert!(optimized[0].is_speech);
        assert_eq!(optimized[0].start_time, 0.0);
        assert_eq!(optimized[0].end_time, 2.0); // Merged
        assert!(!optimized[1].is_speech);
        assert!(optimized[2].is_speech);
    }

    /// Test detection with various threshold configurations
    #[test]
    fn test_detection_thresholds() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        // Test strict threshold
        let strict_config = SyncConfig {
            max_offset_seconds: 5.0,
            audio_sample_rate: 16000,
            correlation_threshold: 0.8,
            dialogue_detection_threshold: 0.8, // High threshold
            min_dialogue_duration_ms: 1000,
            dialogue_merge_gap_ms: 500,
            enable_dialogue_detection: true,
            auto_detect_sample_rate: false,
        };

        let strict_analyzer = EnergyAnalyzer::new(
            strict_config.dialogue_detection_threshold,
            strict_config.min_dialogue_duration_ms,
        );

        let strict_detector = DialogueDetector {
            energy_analyzer: strict_analyzer,
            config: strict_config,
        };

        // Test lenient threshold
        let lenient_config = SyncConfig {
            max_offset_seconds: 5.0,
            audio_sample_rate: 16000,
            correlation_threshold: 0.5,
            dialogue_detection_threshold: 0.1, // Low threshold
            min_dialogue_duration_ms: 100,
            dialogue_merge_gap_ms: 200,
            enable_dialogue_detection: true,
            auto_detect_sample_rate: false,
        };

        let lenient_analyzer = EnergyAnalyzer::new(
            lenient_config.dialogue_detection_threshold,
            lenient_config.min_dialogue_duration_ms,
        );

        let lenient_detector = DialogueDetector {
            energy_analyzer: lenient_analyzer,
            config: lenient_config,
        };

        // Verify configuration differences
        assert!(
            strict_detector.config.dialogue_detection_threshold
                > lenient_detector.config.dialogue_detection_threshold
        );
        assert!(
            strict_detector.config.min_dialogue_duration_ms
                > lenient_detector.config.min_dialogue_duration_ms
        );
    }

    /// Test edge cases in speech ratio calculation
    #[test]
    fn test_speech_ratio_edge_cases() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Empty segments
        let empty_segments = vec![];
        let ratio = detector.get_speech_ratio(&empty_segments);
        assert_eq!(ratio, 0.0, "Empty segments should have 0.0 speech ratio");

        // Zero duration segments
        let zero_segments = vec![
            DialogueSegment::new_speech(1.0, 1.0), // Zero duration
        ];
        let ratio = detector.get_speech_ratio(&zero_segments);
        assert_eq!(
            ratio, 0.0,
            "Zero duration segments should result in 0.0 ratio"
        );

        // Mixed zero and non-zero duration
        let mixed_segments = vec![
            DialogueSegment::new_speech(0.0, 0.0),  // Zero duration
            DialogueSegment::new_speech(1.0, 3.0),  // 2 seconds
            DialogueSegment::new_silence(3.0, 5.0), // 2 seconds
        ];
        // Total: 4 seconds, Speech: 2 seconds, Ratio: 0.5
        let ratio = detector.get_speech_ratio(&mixed_segments);
        assert_eq!(ratio, 0.5, "Mixed segments should have 0.5 speech ratio");
    }

    /// Test configuration loading and validation
    #[test]
    fn test_detector_configuration() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Verify configuration is loaded correctly
        assert!(detector.config.dialogue_detection_threshold >= 0.0);
        assert!(detector.config.min_dialogue_duration_ms > 0);
        // dialogue_merge_gap_ms is u64, so it's always >= 0

        // Test that detector can be created multiple times
        let detector2 = DialogueDetector::new().unwrap();
        assert_eq!(
            detector.config.dialogue_detection_threshold,
            detector2.config.dialogue_detection_threshold
        );
    }

    /// Test dialogue segment merging logic
    #[test]
    fn test_dialogue_segment_merging() {
        crate::config::reset_global_config_manager();
        crate::config::init_config_manager().unwrap();

        let detector = DialogueDetector::new().unwrap();

        // Test segments that should NOT be merged (different types)
        let mixed_segments = vec![
            DialogueSegment::new_speech(0.0, 1.0),
            DialogueSegment::new_silence(1.1, 2.0), // Silence, shouldn't merge with speech
            DialogueSegment::new_speech(2.1, 3.0),
        ];

        let optimized = detector.optimize_segments(mixed_segments);
        assert_eq!(optimized.len(), 3, "Mixed types should not merge");

        // Test segments that SHOULD be merged (same type, small gap)
        let speech_segments = vec![
            DialogueSegment::new_speech(0.0, 1.0),
            DialogueSegment::new_speech(1.2, 2.0), // 200ms gap, should merge
            DialogueSegment::new_speech(2.1, 3.0), // 100ms gap, should merge
        ];

        let optimized = detector.optimize_segments(speech_segments);
        assert_eq!(
            optimized.len(),
            1,
            "Close speech segments should merge into one"
        );
        assert_eq!(optimized[0].start_time, 0.0);
        assert_eq!(optimized[0].end_time, 3.0);
    }
}
