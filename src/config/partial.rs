//! Partial configuration structures and merging logic.

use serde::{Deserialize, Serialize};

/// Partial configuration for all sections.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialConfig {
    pub ai: PartialAIConfig,
    pub formats: PartialFormatsConfig,
    pub sync: PartialSyncConfig,
    pub general: PartialGeneralConfig,
}

/// Partial AI configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialAIConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub max_sample_length: Option<usize>,
    pub temperature: Option<f32>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

/// Partial formats configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialFormatsConfig {
    pub default_output: Option<String>,
    pub preserve_styling: Option<bool>,
    pub default_encoding: Option<String>,
}

/// Partial sync configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialSyncConfig {
    pub max_offset_seconds: Option<f32>,
    pub audio_sample_rate: Option<u32>,
    pub correlation_threshold: Option<f32>,
    pub dialogue_detection_threshold: Option<f32>,
    pub min_dialogue_duration_ms: Option<u64>,
}

/// Partial general configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialGeneralConfig {
    pub backup_enabled: Option<bool>,
    pub max_concurrent_jobs: Option<usize>,
}

impl PartialConfig {
    /// Merge another partial configuration, overriding present fields.
    pub fn merge(
        &mut self,
        other: PartialConfig,
    ) -> Result<(), crate::config::manager::ConfigError> {
        if let Some(v) = other.ai.provider {
            self.ai.provider = Some(v);
        }
        if let Some(v) = other.ai.api_key {
            self.ai.api_key = Some(v);
        }
        if let Some(v) = other.ai.model {
            self.ai.model = Some(v);
        }
        if let Some(v) = other.ai.max_sample_length {
            self.ai.max_sample_length = Some(v);
        }
        if let Some(v) = other.ai.temperature {
            self.ai.temperature = Some(v);
        }
        if let Some(v) = other.ai.retry_attempts {
            self.ai.retry_attempts = Some(v);
        }
        if let Some(v) = other.ai.retry_delay_ms {
            self.ai.retry_delay_ms = Some(v);
        }
        if let Some(v) = other.formats.default_output {
            self.formats.default_output = Some(v);
        }
        if let Some(v) = other.formats.preserve_styling {
            self.formats.preserve_styling = Some(v);
        }
        if let Some(v) = other.formats.default_encoding {
            self.formats.default_encoding = Some(v);
        }
        if let Some(v) = other.sync.max_offset_seconds {
            self.sync.max_offset_seconds = Some(v);
        }
        if let Some(v) = other.sync.audio_sample_rate {
            self.sync.audio_sample_rate = Some(v);
        }
        if let Some(v) = other.sync.correlation_threshold {
            self.sync.correlation_threshold = Some(v);
        }
        if let Some(v) = other.sync.dialogue_detection_threshold {
            self.sync.dialogue_detection_threshold = Some(v);
        }
        if let Some(v) = other.sync.min_dialogue_duration_ms {
            self.sync.min_dialogue_duration_ms = Some(v);
        }
        if let Some(v) = other.general.backup_enabled {
            self.general.backup_enabled = Some(v);
        }
        if let Some(v) = other.general.max_concurrent_jobs {
            self.general.max_concurrent_jobs = Some(v);
        }
        Ok(())
    }
}
