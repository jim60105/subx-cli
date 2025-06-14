//! 重構後的同步引擎，支援多種檢測方法
//!
//! 此模組提供統一的字幕同步功能，整合 OpenAI Whisper API 和本地 VAD
//! 等多種語音檢測方法，並提供自動方法選擇和回退機制。

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::config::{ConfigService, SyncConfig};
use crate::core::formats::Subtitle;
use crate::services::vad::VadSyncDetector;
use crate::services::whisper::{AudioSegmentExtractor, WhisperSyncDetector};
use crate::{Result, error::SubXError};

/// 統一的同步引擎，支援多種同步方法及自動模式
pub struct SyncEngine {
    config: SyncConfig,
    whisper_detector: Option<WhisperSyncDetector>,
    vad_detector: Option<VadSyncDetector>,
    audio_extractor: AudioSegmentExtractor,
}

impl SyncEngine {
    /// 建立新的同步引擎實例
    pub async fn new(config: SyncConfig, config_service: &dyn ConfigService) -> Result<Self> {
        let whisper_detector = if config.whisper.enabled {
            match Self::create_whisper_detector(&config, config_service).await {
                Ok(det) => Some(det),
                Err(e) => {
                    log::warn!("Whisper 初始化失敗: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let vad_detector = if config.vad.enabled {
            match VadSyncDetector::new(config.vad.clone()) {
                Ok(det) => Some(det),
                Err(e) => {
                    log::warn!("VAD 初始化失敗: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if whisper_detector.is_none() && vad_detector.is_none() {
            return Err(SubXError::config("No synchronization methods available"));
        }

        Ok(Self {
            config,
            whisper_detector,
            vad_detector,
            audio_extractor: AudioSegmentExtractor::new()?,
        })
    }

    async fn create_whisper_detector(
        config: &SyncConfig,
        config_service: &dyn ConfigService,
    ) -> Result<WhisperSyncDetector> {
        let full = config_service.get_config()?;
        let key = full
            .ai
            .api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| SubXError::config("OpenAI API key not found"))?;
        WhisperSyncDetector::new(key, full.ai.base_url, config.whisper.clone())
    }

    /// 自動或指定方法檢測同步偏移
    pub async fn detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
        method: Option<SyncMethod>,
    ) -> Result<SyncResult> {
        let start = Instant::now();
        let m = method.unwrap_or_else(|| self.determine_default_method());
        let mut res = match m {
            SyncMethod::Auto => self.auto_detect_sync_offset(audio_path, subtitle).await?,
            SyncMethod::WhisperApi => {
                self.whisper_detect_sync_offset(audio_path, subtitle)
                    .await?
            }
            SyncMethod::LocalVad => self.vad_detect_sync_offset(audio_path, subtitle).await?,
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
        // 簡易自動模式：依預設順序嘗試 Whisper API 再 VAD
        if self.whisper_detector.is_some() {
            return self.whisper_detect_sync_offset(audio_path, subtitle).await;
        }
        if self.vad_detector.is_some() {
            return self.vad_detect_sync_offset(audio_path, subtitle).await;
        }
        Err(SubXError::audio_processing(
            "No detector available in auto mode",
        ))
    }

    /// 應用手動偏移量
    pub fn apply_manual_offset(
        &self,
        subtitle: &mut Subtitle,
        offset_seconds: f32,
    ) -> Result<SyncResult> {
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
            "whisper" => SyncMethod::WhisperApi,
            "vad" => SyncMethod::LocalVad,
            _ => SyncMethod::Auto,
        }
    }

    async fn whisper_detect_sync_offset(
        &self,
        audio_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        let det = self
            .whisper_detector
            .as_ref()
            .ok_or_else(|| SubXError::audio_processing("Whisper detector not available"))?;
        let r = det
            .detect_sync_offset(audio_path, subtitle, self.config.analysis_window_seconds)
            .await?;
        if r.confidence < self.config.whisper.min_confidence_threshold
            && self.config.whisper.fallback_to_vad
        {
            if let Some(vad) = &self.vad_detector {
                let mut v = vad
                    .detect_sync_offset(audio_path, subtitle, self.config.analysis_window_seconds)
                    .await?;
                v.warnings
                    .push("Whisper confidence below threshold, fallback to VAD".to_string());
                return Ok(v);
            }
        }
        Ok(r)
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
        det.detect_sync_offset(audio_path, subtitle, self.config.analysis_window_seconds)
            .await
    }
}

/// 同步方法枚舉
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncMethod {
    Auto,
    WhisperApi,
    LocalVad,
    Manual,
}

/// 同步結果結構
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub offset_seconds: f32,
    pub confidence: f32,
    pub method_used: SyncMethod,
    pub correlation_peak: f32,
    pub additional_info: Option<serde_json::Value>,
    pub processing_duration: Duration,
    pub warnings: Vec<String>,
}

/// 方法選擇策略，預留後續擴充
#[derive(Debug, Clone)]
pub struct MethodSelectionStrategy {
    pub preferred_methods: Vec<SyncMethod>,
    pub min_confidence_threshold: f32,
    pub allow_fallback: bool,
    pub max_attempt_duration: u32,
}

// 單元測試模組：補充同步引擎核心行為驗證
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TestConfigBuilder, TestConfigService};
    use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
    use std::time::Duration;

    #[tokio::test]
    async fn test_sync_engine_creation() {
        let config = TestConfigBuilder::new()
            .with_whisper_enabled(false)
            .with_vad_enabled(true)
            .build_config();
        let config_service = TestConfigService::new(config);
        let result =
            SyncEngine::new(config_service.get_config().unwrap().sync, &config_service).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_manual_offset_application() {
        let config = TestConfigBuilder::new().build_config();
        let config_service = TestConfigService::new(config);
        let engine = SyncEngine::new(config_service.get_config().unwrap().sync, &config_service)
            .await
            .unwrap();

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
        let test_cases = vec![
            ("whisper", SyncMethod::WhisperApi),
            ("vad", SyncMethod::LocalVad),
            ("unknown", SyncMethod::Auto),
        ];

        for (config_value, expected_method) in test_cases {
            let config = TestConfigBuilder::new()
                .with_sync_method(config_value)
                .build_config();
            let config_service = TestConfigService::new(config);
            let engine =
                SyncEngine::new(config_service.get_config().unwrap().sync, &config_service)
                    .await
                    .unwrap();
            assert_eq!(engine.determine_default_method(), expected_method);
        }
    }

    #[tokio::test]
    async fn test_method_selection_strategy_struct() {
        let strategy = MethodSelectionStrategy {
            preferred_methods: vec![SyncMethod::WhisperApi, SyncMethod::LocalVad],
            min_confidence_threshold: 0.7,
            allow_fallback: true,
            max_attempt_duration: 60,
        };
        assert_eq!(strategy.preferred_methods.len(), 2);
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

/// 向後兼容性 - 保留舊的 SyncConfig 結構但標記為 deprecated
#[deprecated(note = "Use new SyncConfig with Whisper and VAD support")]
pub struct OldSyncConfig {
    pub max_offset_seconds: f32,
    pub correlation_threshold: f32,
    pub dialogue_threshold: f32,
    pub min_dialogue_length: f32,
}
