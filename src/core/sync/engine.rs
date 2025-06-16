//! Refactored sync engine with VAD (Voice Activity Detection) support.
//!
//! This module provides unified subtitle synchronization functionality using
//! local VAD (Voice Activity Detection) for voice detection and sync offset calculation.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::SyncConfig;
use crate::core::formats::Subtitle;
use crate::services::vad::VadSyncDetector;
use crate::{Result, error::SubXError};

/// Unified sync engine based on VAD voice detection.
///
/// This engine provides automatic subtitle synchronization using Voice Activity
/// Detection (VAD) to analyze audio tracks and calculate optimal sync offsets.
pub struct SyncEngine {
    config: SyncConfig,
    vad_detector: Option<VadSyncDetector>,
}

impl SyncEngine {
    /// Create a new sync engine instance.
    ///
    /// # Arguments
    ///
    /// * `config` - Sync configuration containing VAD settings and thresholds
    ///
    /// # Returns
    ///
    /// A new sync engine instance with initialized VAD detector if enabled.
    pub fn new(config: SyncConfig) -> Result<Self> {
        let vad_detector = if config.vad.enabled {
            match VadSyncDetector::new(config.vad.clone()) {
                Ok(det) => Some(det),
                Err(e) => {
                    log::warn!("VAD initialization failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if vad_detector.is_none() {
            return Err(SubXError::config(
                "VAD detector is required but not available",
            ));
        }

        Ok(Self {
            config,
            vad_detector,
        })
    }

    /// Detect sync offset using automatic or specified method.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio file for analysis
    /// * `subtitle` - Subtitle data to synchronize
    /// * `method` - Optional sync method, defaults to automatic detection
    ///
    /// # Returns
    ///
    /// Sync result containing offset, confidence, and processing metadata.
    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        method: Option<SyncMethod>,
    ) -> Result<SyncResult> {
        let start = Instant::now();
        let m = method.unwrap_or_else(|| self.determine_default_method());
        let mut res = match m {
            SyncMethod::Auto | SyncMethod::LocalVad => {
                self.vad_detect_sync_offset(audio_path, subtitle).await?
            }
            SyncMethod::Manual => {
                return Err(SubXError::config("Manual method requires explicit offset"));
            }
        };
        res.processing_duration = start.elapsed();
        Ok(res)
    }

    async fn auto_detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        // Auto mode uses VAD
        if self.vad_detector.is_some() {
            return self.vad_detect_sync_offset(audio_path, subtitle).await;
        }
        Err(SubXError::audio_processing(
            "No detector available in auto mode",
        ))
    }

    /// Apply manual offset to subtitle timing.
    ///
    /// # Arguments
    ///
    /// * `subtitle` - Mutable subtitle data to modify
    /// * `offset_seconds` - Offset in seconds (positive delays, negative advances)
    ///
    /// # Returns
    ///
    /// Sync result with the applied offset and full confidence.
    ///
    /// # Errors
    ///
    /// Returns an error if the offset exceeds the configured maximum.
    pub fn apply_manual_offset(
        &self,
        subtitle: &mut Subtitle,
        offset_seconds: f32,
    ) -> Result<SyncResult> {
        // Validate offset against max_offset_seconds configuration
        if offset_seconds.abs() > self.config.max_offset_seconds {
            return Err(SubXError::config(format!(
                "Offset {:.2}s exceeds maximum allowed value {:.2}s. Please check the sync.max_offset_seconds configuration or use a smaller offset.",
                offset_seconds, self.config.max_offset_seconds
            )));
        }

        let start = Instant::now();
        for entry in &mut subtitle.entries {
            entry.start_time = entry
                .start_time
                .checked_add(Duration::from_secs_f32(offset_seconds.abs()))
                .or_else(|| {
                    if offset_seconds < 0.0 {
                        entry
                            .start_time
                            .checked_sub(Duration::from_secs_f32(-offset_seconds))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    SubXError::audio_processing("Invalid offset results in negative time")
                })?;
            entry.end_time = entry
                .end_time
                .checked_add(Duration::from_secs_f32(offset_seconds.abs()))
                .or_else(|| {
                    if offset_seconds < 0.0 {
                        entry
                            .end_time
                            .checked_sub(Duration::from_secs_f32(-offset_seconds))
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    SubXError::audio_processing("Invalid offset results in negative time")
                })?;
        }
        Ok(SyncResult {
            offset_seconds,
            confidence: 1.0,
            method_used: SyncMethod::Manual,
            correlation_peak: 1.0,
            additional_info: Some(json!({
                "applied_offset": offset_seconds,
                "entries_modified": subtitle.entries.len(),
            })),
            processing_duration: start.elapsed(),
            warnings: Vec::new(),
        })
    }

    fn determine_default_method(&self) -> SyncMethod {
        match self.config.default_method.as_str() {
            "vad" => SyncMethod::LocalVad,
            _ => SyncMethod::Auto,
        }
    }

    async fn vad_detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        let det = self
            .vad_detector
            .as_ref()
            .ok_or_else(|| SubXError::audio_processing("VAD detector not available"))?;

        let mut result = det.detect_sync_offset(audio_path, subtitle, 0).await?; // analysis_window_seconds no longer used

        // Validate detected offset against max_offset_seconds configuration
        if result.offset_seconds.abs() > self.config.max_offset_seconds {
            // Provide warning but don't completely fail, allow user to decide
            result.warnings.push(format!(
                "Detected offset {:.2}s exceeds configured maximum value {:.2}s. Consider checking audio quality or adjusting sync.max_offset_seconds configuration.",
                result.offset_seconds, self.config.max_offset_seconds
            ));

            // Optionally: clamp to maximum value (preserving sign)
            let sign = if result.offset_seconds >= 0.0 {
                1.0
            } else {
                -1.0
            };
            let original_offset = result.offset_seconds;
            result.offset_seconds = sign * self.config.max_offset_seconds;

            result.additional_info = Some(json!({
                "original_offset": original_offset,
                "clamped_offset": result.offset_seconds,
                "reason": "Exceeded max_offset_seconds configuration"
            }));
        }

        Ok(result)
    }
}

/// Sync method enumeration.
///
/// Defines the available methods for subtitle synchronization,
/// from automatic detection to manual offset specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncMethod {
    /// Automatic method selection (currently VAD only).
    Auto,
    /// Local VAD (Voice Activity Detection) processing.
    LocalVad,
    /// Manual offset specification.
    Manual,
}

/// Synchronization result structure.
///
/// Contains the complete results of subtitle synchronization analysis,
/// including calculated offset, confidence metrics, and processing metadata.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Calculated time offset in seconds
    pub offset_seconds: f32,
    /// Confidence level of the detection (0.0-1.0)
    pub confidence: f32,
    /// Synchronization method that was used
    pub method_used: SyncMethod,
    /// Peak correlation value from analysis
    pub correlation_peak: f32,
    /// Additional method-specific information
    pub additional_info: Option<serde_json::Value>,
    /// Time taken to complete the analysis
    pub processing_duration: Duration,
    /// Any warnings generated during processing
    pub warnings: Vec<String>,
}

/// Method selection strategy for synchronization analysis.
///
/// Defines preferences and fallback behavior for automatic method selection
/// when multiple synchronization approaches are available.
#[derive(Debug, Clone)]
pub struct MethodSelectionStrategy {
    /// Preferred methods in order of preference
    pub preferred_methods: Vec<SyncMethod>,
    /// Minimum confidence threshold for accepting results
    pub min_confidence_threshold: f32,
    /// Whether to allow fallback to alternative methods
    pub allow_fallback: bool,
    /// Maximum time to spend on analysis attempts
    pub max_attempt_duration: u32,
}

// Unit test module: Supplement sync engine core behavior verification
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TestConfigBuilder, TestConfigService, service::ConfigService};
    use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
    use std::time::Duration;

    #[tokio::test]
    async fn test_sync_engine_creation() {
        let config = TestConfigBuilder::new()
            .with_vad_enabled(true)
            .build_config();
        let config_service = TestConfigService::new(config);
        let result = SyncEngine::new(config_service.get_config().unwrap().sync);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manual_offset_application() {
        let config = TestConfigBuilder::new().build_config();
        let config_service = TestConfigService::new(config);
        let engine = SyncEngine::new(config_service.get_config().unwrap().sync).unwrap();

        let mut subtitle = create_test_subtitle();
        let original_start = subtitle.entries[0].start_time;

        let result = engine.apply_manual_offset(&mut subtitle, 2.5).unwrap();
        assert_eq!(result.offset_seconds, 2.5);
        assert_eq!(result.method_used, SyncMethod::Manual);
        assert_eq!(result.confidence, 1.0);

        let expected_start = original_start + Duration::from_secs_f32(2.5);
        assert_eq!(subtitle.entries[0].start_time, expected_start);
    }

    #[tokio::test]
    async fn test_determine_default_method() {
        let test_cases = vec![("vad", SyncMethod::LocalVad), ("unknown", SyncMethod::Auto)];

        for (config_value, expected_method) in test_cases {
            let config = TestConfigBuilder::new()
                .with_sync_method(config_value)
                .build_config();
            let engine = SyncEngine::new(config.sync).unwrap();
            assert_eq!(engine.determine_default_method(), expected_method);
        }
    }

    #[tokio::test]
    async fn test_method_selection_strategy_struct() {
        let strategy = MethodSelectionStrategy {
            preferred_methods: vec![SyncMethod::LocalVad],
            min_confidence_threshold: 0.7,
            allow_fallback: true,
            max_attempt_duration: 60,
        };
        assert_eq!(strategy.preferred_methods.len(), 1);
        assert!(strategy.allow_fallback);
    }

    fn create_test_subtitle() -> Subtitle {
        Subtitle {
            entries: vec![SubtitleEntry::new(
                1,
                Duration::from_secs(10),
                Duration::from_secs(12),
                "Test subtitle".to_string(),
            )],
            metadata: SubtitleMetadata::default(),
            format: SubtitleFormatType::Srt,
        }
    }
}

/// Backward compatibility - Deprecated legacy SyncConfig structure.
#[deprecated(note = "Use new SyncConfig with Whisper and VAD support")]
pub struct OldSyncConfig {
    /// Maximum search offset in seconds for synchronization.
    pub max_offset_seconds: f32,
    /// Minimum correlation threshold for accepting sync results.
    pub correlation_threshold: f32,
    /// Dialogue detection threshold for identifying speech segments.
    pub dialogue_threshold: f32,
    /// Minimum dialogue segment length in seconds.
    pub min_dialogue_length: f32,
}
