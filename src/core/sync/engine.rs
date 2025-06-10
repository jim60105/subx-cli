//! Subtitle synchronization engine using audio analysis and pattern matching.
//!
//! This module provides `SyncEngine` and related types to align subtitle timing
//! with audio tracks based on correlation and dialogue analysis.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::sync::engine::{SyncEngine, SyncConfig};
//! let config = SyncConfig { max_offset_seconds: 5.0, correlation_threshold: 0.8, dialogue_threshold: 0.5, min_dialogue_length: 1.0 };
//! let engine = SyncEngine::new(config);
//! ```
use crate::Result;
use crate::core::formats::Subtitle;
use crate::services::audio::{AudioAnalyzer, AudioEnvelope};
use std::path::Path;

/// Synchronization engine for aligning subtitles with audio tracks.
pub struct SyncEngine {
    audio_analyzer: AudioAnalyzer,
    config: SyncConfig,
}

/// Configuration parameters for the subtitle synchronization process.
///
/// Controls various aspects of the audio-subtitle synchronization algorithm,
/// including detection thresholds and search ranges.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum time offset to search for synchronization (in seconds)
    pub max_offset_seconds: f32,
    /// Minimum correlation threshold for accepting a sync match
    pub correlation_threshold: f32,
    /// Threshold for detecting dialogue in audio analysis
    pub dialogue_threshold: f32,
    /// Minimum length required for dialogue segments (in seconds)
    pub min_dialogue_length: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
    use std::time::Duration;

    /// Test manual offset application to subtitle timings
    #[test]
    fn test_apply_sync_offset_positive() {
        let mut subtitle = Subtitle {
            entries: vec![SubtitleEntry::new(
                1,
                Duration::from_secs(1),
                Duration::from_secs(2),
                String::from("test"),
            )],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        };
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 0.0,
            correlation_threshold: 0.0,
            dialogue_threshold: 0.0,
            min_dialogue_length: 0.0,
        });
        engine.apply_sync_offset(&mut subtitle, 2.0).unwrap();
        assert_eq!(subtitle.entries[0].start_time, Duration::from_secs(3));
        assert_eq!(subtitle.entries[0].end_time, Duration::from_secs(4));
    }

    /// Test negative offset application to subtitle timings
    #[test]
    fn test_apply_sync_offset_negative() {
        let mut subtitle = Subtitle {
            entries: vec![SubtitleEntry::new(
                1,
                Duration::from_secs(5),
                Duration::from_secs(7),
                String::from("test"),
            )],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        };
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 0.0,
            correlation_threshold: 0.0,
            dialogue_threshold: 0.0,
            min_dialogue_length: 0.0,
        });
        engine.apply_sync_offset(&mut subtitle, -2.0).unwrap();
        assert_eq!(subtitle.entries[0].start_time, Duration::from_secs(3));
        assert_eq!(subtitle.entries[0].end_time, Duration::from_secs(5));
    }

    /// Test sync configuration validation
    #[test]
    fn test_sync_config_creation() {
        let config = SyncConfig {
            max_offset_seconds: 5.0,
            correlation_threshold: 0.8,
            dialogue_threshold: 0.5,
            min_dialogue_length: 1.0,
        };

        assert_eq!(config.max_offset_seconds, 5.0);
        assert_eq!(config.correlation_threshold, 0.8);
        assert_eq!(config.dialogue_threshold, 0.5);
        assert_eq!(config.min_dialogue_length, 1.0);
    }

    /// Test audio correlation algorithm with known signals
    #[test]
    fn test_calculate_correlation_at_offset() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 1.0,
            correlation_threshold: 0.7,
            dialogue_threshold: 0.3,
            min_dialogue_length: 0.5,
        });

        // Create identical signals - should have perfect correlation at offset 0
        let audio_signal = vec![0.5, 0.8, 0.2, 0.9, 0.1];
        let subtitle_signal = vec![0.5, 0.8, 0.2, 0.9, 0.1];

        let correlation =
            engine.calculate_correlation_at_offset(&audio_signal, &subtitle_signal, 0);
        assert!(
            correlation > 0.99,
            "Perfect correlation should be close to 1.0, got: {}",
            correlation
        );

        // Test with offset
        let correlation_offset =
            engine.calculate_correlation_at_offset(&audio_signal, &subtitle_signal, 1);
        assert!(
            correlation_offset < correlation,
            "Correlation with offset should be lower"
        );
    }

    /// Test subtitle signal generation algorithm
    #[test]
    fn test_generate_subtitle_signal() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 5.0,
            correlation_threshold: 0.8,
            dialogue_threshold: 0.5,
            min_dialogue_length: 1.0,
        });

        let subtitle = Subtitle {
            entries: vec![
                SubtitleEntry::new(
                    1,
                    Duration::from_secs(1),
                    Duration::from_secs(2),
                    "Test 1".to_string(),
                ),
                SubtitleEntry::new(
                    2,
                    Duration::from_secs(4),
                    Duration::from_secs(5),
                    "Test 2".to_string(),
                ),
            ],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        };

        let signal = engine.generate_subtitle_signal(&subtitle, 6.0, 1); // 1 Hz for simplicity

        // Signal should be 6 samples long (6 seconds * 1 Hz)
        assert_eq!(signal.len(), 6);

        // Check subtitle coverage: samples 1-2 and 4-5 should be 1.0
        assert_eq!(signal[0], 0.0); // Before first subtitle
        assert_eq!(signal[1], 1.0); // First subtitle (1-2s)
        assert_eq!(signal[2], 0.0); // Gap between subtitles
        assert_eq!(signal[3], 0.0); // Gap continues
        assert_eq!(signal[4], 1.0); // Second subtitle (4-5s)
        assert_eq!(signal[5], 0.0); // After last subtitle
    }

    /// Test cross-correlation result structure
    #[test]
    fn test_sync_result_creation() {
        let result = SyncResult {
            offset_seconds: 2.5,
            confidence: 0.85,
            method_used: SyncMethod::AudioCorrelation,
            correlation_peak: 0.92,
        };

        assert_eq!(result.offset_seconds, 2.5);
        assert_eq!(result.confidence, 0.85);
        assert!(matches!(result.method_used, SyncMethod::AudioCorrelation));
        assert_eq!(result.correlation_peak, 0.92);
    }

    /// Test engine initialization with different configurations
    #[test]
    fn test_engine_initialization() {
        let config = SyncConfig {
            max_offset_seconds: 10.0,
            correlation_threshold: 0.6,
            dialogue_threshold: 0.4,
            min_dialogue_length: 2.0,
        };

        let engine = SyncEngine::new(config);
        assert_eq!(engine.config.max_offset_seconds, 10.0);
        assert_eq!(engine.config.correlation_threshold, 0.6);
    }

    /// Test zero-time edge cases in offset application
    #[test]
    fn test_apply_sync_offset_edge_cases() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 5.0,
            correlation_threshold: 0.8,
            dialogue_threshold: 0.5,
            min_dialogue_length: 1.0,
        });

        // Test zero offset
        let mut subtitle = Subtitle {
            entries: vec![SubtitleEntry::new(
                1,
                Duration::from_secs(2),
                Duration::from_secs(4),
                "Test".to_string(),
            )],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        };

        engine.apply_sync_offset(&mut subtitle, 0.0).unwrap();
        assert_eq!(subtitle.entries[0].start_time, Duration::from_secs(2));
        assert_eq!(subtitle.entries[0].end_time, Duration::from_secs(4));

        // Test negative offset larger than start time (should clamp to zero)
        engine.apply_sync_offset(&mut subtitle, -3.0).unwrap();
        assert_eq!(subtitle.entries[0].start_time, Duration::ZERO);
        assert_eq!(subtitle.entries[0].end_time, Duration::from_secs(3));
    }

    /// Test correlation algorithm with misaligned signals
    #[test]
    fn test_correlation_with_misalignment() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 2.0,
            correlation_threshold: 0.5,
            dialogue_threshold: 0.3,
            min_dialogue_length: 0.5,
        });

        // Audio signal with peak at position 3
        let audio_signal = vec![0.1, 0.2, 0.1, 0.9, 0.1, 0.2, 0.1];
        // Subtitle signal with peak at position 1 (should correlate better when shifted)
        let subtitle_signal = vec![0.1, 0.9, 0.1, 0.2, 0.1];

        // Test various offsets to find the best correlation
        let mut best_corr = 0.0;
        let mut best_offset = 0;

        for offset in -3..=3 {
            let corr =
                engine.calculate_correlation_at_offset(&audio_signal, &subtitle_signal, offset);
            if corr > best_corr {
                best_corr = corr;
                best_offset = offset;
            }
        }

        // The best correlation should be found at offset -2 (shifting subtitle signal left to align peaks)
        assert_eq!(best_offset, -2);
        assert!(
            best_corr > 0.5,
            "Best correlation should be reasonably high: {}",
            best_corr
        );
    }

    /// Test subtitle signal generation with overlapping entries
    #[test]
    fn test_generate_subtitle_signal_overlapping() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 5.0,
            correlation_threshold: 0.8,
            dialogue_threshold: 0.5,
            min_dialogue_length: 1.0,
        });

        let subtitle = Subtitle {
            entries: vec![
                SubtitleEntry::new(
                    1,
                    Duration::from_secs(1),
                    Duration::from_secs(3),
                    "First".to_string(),
                ),
                SubtitleEntry::new(
                    2,
                    Duration::from_secs(2),
                    Duration::from_secs(4),
                    "Second".to_string(),
                ),
            ],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        };

        let signal = engine.generate_subtitle_signal(&subtitle, 5.0, 1); // 1 Hz

        // Overlapping region (2-3s) should still be 1.0
        assert_eq!(signal[0], 0.0); // Before any subtitle
        assert_eq!(signal[1], 1.0); // First subtitle starts
        assert_eq!(signal[2], 1.0); // Overlapping region
        assert_eq!(signal[3], 1.0); // Second subtitle continues
        assert_eq!(signal[4], 0.0); // After all subtitles
    }

    /// Test correlation calculation with empty or invalid signals
    #[test]
    fn test_correlation_edge_cases() {
        let engine = SyncEngine::new(SyncConfig {
            max_offset_seconds: 1.0,
            correlation_threshold: 0.5,
            dialogue_threshold: 0.3,
            min_dialogue_length: 0.5,
        });

        // Test with all-zero signals
        let zero_signal = vec![0.0; 5];
        let correlation = engine.calculate_correlation_at_offset(&zero_signal, &zero_signal, 0);
        assert_eq!(
            correlation, 0.0,
            "Correlation of zero signals should be 0.0"
        );

        // Test with empty signals
        let empty_signal = vec![];
        let correlation = engine.calculate_correlation_at_offset(&empty_signal, &empty_signal, 0);
        assert_eq!(
            correlation, 0.0,
            "Correlation of empty signals should be 0.0"
        );

        // Test with out-of-bounds offset
        let signal = vec![1.0, 2.0, 3.0];
        let correlation = engine.calculate_correlation_at_offset(&signal, &signal, 10);
        assert_eq!(
            correlation, 0.0,
            "Correlation with out-of-bounds offset should be 0.0"
        );
    }
}

/// Result of the subtitle synchronization process.
///
/// Contains detailed information about the synchronization outcome,
/// including timing adjustments and confidence metrics.
#[derive(Debug)]
pub struct SyncResult {
    /// Time offset in seconds to apply to subtitle timing
    pub offset_seconds: f32,
    /// Confidence level of the synchronization result (0.0 to 1.0)
    pub confidence: f32,
    /// Method used to achieve synchronization
    pub method_used: SyncMethod,
    /// Peak correlation value found during analysis
    pub correlation_peak: f32,
}

/// Available methods for synchronizing subtitles with audio.
///
/// Represents different algorithms and approaches that can be used
/// to determine the correct timing offset between audio and subtitles.
#[derive(Debug)]
pub enum SyncMethod {
    /// Correlation-based synchronization using audio analysis
    AudioCorrelation,
    /// Manual offset specified by the user
    ManualOffset,
    /// Pattern matching between subtitle and audio timing
    PatternMatching,
}

impl SyncEngine {
    /// Creates a new `SyncEngine` instance with the given configuration.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            audio_analyzer: AudioAnalyzer::new(16000),
            config,
        }
    }

    /// Automatically adjusts subtitle timing to match the audio in the video file.
    ///
    /// # Arguments
    ///
    /// * `video_path` - Path to the source video or audio file.
    /// * `subtitle` - The subtitle object to synchronize.
    pub async fn sync_subtitle(
        &self,
        video_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        let audio_envelope = self.audio_analyzer.extract_envelope(video_path).await?;
        let _dialogue_segments = self
            .audio_analyzer
            .detect_dialogue(&audio_envelope, self.config.dialogue_threshold);

        let subtitle_signal = self.generate_subtitle_signal(
            subtitle,
            audio_envelope.duration,
            audio_envelope.sample_rate,
        );
        let correlation_result =
            self.calculate_cross_correlation(&audio_envelope, &subtitle_signal)?;

        Ok(correlation_result)
    }

    fn generate_subtitle_signal(
        &self,
        subtitle: &Subtitle,
        total_duration: f32,
        sample_rate: u32,
    ) -> Vec<f32> {
        let sample_rate = sample_rate as f32;
        let signal_length = (total_duration * sample_rate) as usize;
        let mut signal = vec![0.0; signal_length];

        for entry in &subtitle.entries {
            let start = (entry.start_time.as_secs_f32() * sample_rate) as usize;
            let end = (entry.end_time.as_secs_f32() * sample_rate) as usize;
            let range_end = end.min(signal_length);
            signal[start..range_end].iter_mut().for_each(|v| *v = 1.0);
        }

        signal
    }

    fn calculate_cross_correlation(
        &self,
        audio_envelope: &AudioEnvelope,
        subtitle_signal: &[f32],
    ) -> Result<SyncResult> {
        let max_offset_samples =
            (self.config.max_offset_seconds * audio_envelope.sample_rate as f32) as i32;
        let mut best_offset = 0;
        let mut best_correlation = 0.0;

        for offset in -max_offset_samples..=max_offset_samples {
            let corr = self.calculate_correlation_at_offset(
                &audio_envelope.samples,
                subtitle_signal,
                offset,
            );
            if corr > best_correlation {
                best_correlation = corr;
                best_offset = offset;
            }
        }

        let offset_seconds = best_offset as f32 / audio_envelope.sample_rate as f32;
        let confidence = if best_correlation > self.config.correlation_threshold {
            best_correlation
        } else {
            0.0
        };

        Ok(SyncResult {
            offset_seconds,
            confidence,
            method_used: SyncMethod::AudioCorrelation,
            correlation_peak: best_correlation,
        })
    }

    fn calculate_correlation_at_offset(
        &self,
        audio_signal: &[f32],
        subtitle_signal: &[f32],
        offset: i32,
    ) -> f32 {
        let audio_len = audio_signal.len() as i32;
        let subtitle_len = subtitle_signal.len() as i32;
        let mut sum_product = 0.0;
        let mut sum_audio_sq = 0.0;
        let mut sum_sub_sq = 0.0;
        let mut count = 0;

        for i in 0..audio_len {
            let j = i + offset;
            if j >= 0 && j < subtitle_len {
                let a = audio_signal[i as usize];
                let s = subtitle_signal[j as usize];
                sum_product += a * s;
                sum_audio_sq += a * a;
                sum_sub_sq += s * s;
                count += 1;
            }
        }

        if count == 0 || sum_audio_sq == 0.0 || sum_sub_sq == 0.0 {
            return 0.0;
        }

        sum_product / (sum_audio_sq.sqrt() * sum_sub_sq.sqrt())
    }

    /// Apply sync offset to subtitle
    pub fn apply_sync_offset(&self, subtitle: &mut Subtitle, offset_seconds: f32) -> Result<()> {
        let offset_dur = std::time::Duration::from_secs_f32(offset_seconds.abs());
        for entry in &mut subtitle.entries {
            if offset_seconds >= 0.0 {
                entry.start_time += offset_dur;
                entry.end_time += offset_dur;
            } else if entry.start_time > offset_dur {
                entry.start_time -= offset_dur;
                entry.end_time -= offset_dur;
            } else {
                let rem = offset_dur - entry.start_time;
                entry.start_time = std::time::Duration::ZERO;
                if entry.end_time > rem {
                    entry.end_time -= rem;
                } else {
                    entry.end_time = std::time::Duration::ZERO;
                }
            }
        }
        Ok(())
    }
}
