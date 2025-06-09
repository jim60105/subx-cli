//! Dialogue detection and analysis modules for synchronization.
//!
//! This module exposes submodules to detect and segment dialogue regions
//! from audio signals, which are used to improve subtitle synchronization.
pub mod analyzer;
pub mod detector;
pub mod segment;

pub use analyzer::EnergyAnalyzer;
pub use detector::DialogueDetector;
pub use segment::{DialogueSegment, SilenceSegment};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_config_manager;
    use tempfile::TempDir;

    #[test]
    fn test_config_loading() {
        init_config_manager().unwrap();
        let cfg = crate::config::load_config().unwrap();
        assert!(cfg.sync.dialogue_detection_threshold > 0.0);
        assert!(cfg.sync.min_dialogue_duration_ms > 0);
        assert!(cfg.sync.dialogue_merge_gap_ms > 0);
        assert!(cfg.sync.enable_dialogue_detection);
    }

    #[test]
    fn test_dialogue_segment_operations() {
        let mut s1 = DialogueSegment::new_speech(1.0, 3.0);
        let s2 = DialogueSegment::new_speech(2.5, 4.0);
        assert!(s1.overlaps_with(&s2));
        assert_eq!(s1.duration(), 2.0);
        s1.merge_with(&s2);
        assert_eq!(s1.end_time, 4.0);
    }

    #[test]
    fn test_energy_analyzer_basic() {
        let analyzer = EnergyAnalyzer::new(0.01, 500);
        let sample_audio = vec![0.0; 44100];
        let segs = analyzer.analyze(&sample_audio, 44100);
        assert!(segs.iter().all(|s| !s.is_speech));
    }

    #[tokio::test]
    #[ignore]
    async fn test_dialogue_detector_integration() {
        let temp = TempDir::new().unwrap();
        let audio_path = temp.path().join("test.wav");
        // TODO: 實際測試需提供真實音訊檔案
        let detector = DialogueDetector::new().unwrap();
        let segs = detector.detect_dialogue(&audio_path).await.unwrap();
        assert!(segs.is_empty());
    }
}
