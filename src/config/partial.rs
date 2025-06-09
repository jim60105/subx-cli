//! Partial configuration structures and merging logic.

use crate::config::OverflowStrategy;
use serde::{Deserialize, Serialize};

/// Partial configuration for all sections.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PartialConfig {
    pub ai: PartialAIConfig,
    pub formats: PartialFormatsConfig,
    pub sync: PartialSyncConfig,
    pub general: PartialGeneralConfig,
    pub parallel: PartialParallelConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_partial_ai_config_merge_and_to_complete_base_url() {
        // 初始部分配置，含預設 base_url
        let mut base = PartialConfig::default();
        // 覆蓋 base_url 欄位
        let mut override_cfg = PartialConfig::default();
        override_cfg.ai.base_url = Some("https://override.example.com/v1".to_string());

        base.merge(override_cfg).unwrap();
        let complete = base.to_complete_config().unwrap();
        assert_eq!(complete.ai.base_url, "https://override.example.com/v1");
    }

    #[test]
    fn test_partial_ai_config_to_complete_default_base_url() {
        let base = PartialConfig::default();
        let complete = base.to_complete_config().unwrap();
        assert_eq!(complete.ai.base_url, Config::default().ai.base_url);
    }
}

/// Partial AI configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialAIConfig {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
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
    /// 編碼檢測信心度閾值（0.0-1.0）
    pub encoding_detection_confidence: Option<f32>,
}

/// Partial sync configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialSyncConfig {
    pub max_offset_seconds: Option<f32>,
    pub audio_sample_rate: Option<u32>,
    pub correlation_threshold: Option<f32>,
    pub dialogue_detection_threshold: Option<f32>,
    pub min_dialogue_duration_ms: Option<u64>,
    /// 對話片段合併間隔（毫秒）
    pub dialogue_merge_gap_ms: Option<u64>,
    /// 是否啟用對話檢測
    pub enable_dialogue_detection: Option<bool>,
    /// 是否自動檢測原始採樣率
    pub auto_detect_sample_rate: Option<bool>,
}

/// Partial general configuration.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialGeneralConfig {
    pub backup_enabled: Option<bool>,
    pub max_concurrent_jobs: Option<usize>,
    pub task_timeout_seconds: Option<u64>,
    pub enable_progress_bar: Option<bool>,
    pub worker_idle_timeout_seconds: Option<u64>,
}

/// Partial parallel processing configuration
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialParallelConfig {
    pub task_queue_size: Option<usize>,
    pub enable_task_priorities: Option<bool>,
    pub auto_balance_workers: Option<bool>,
    /// Strategy to apply when the task queue reaches its maximum size.
    pub queue_overflow_strategy: Option<OverflowStrategy>,
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
        if let Some(v) = other.ai.base_url {
            self.ai.base_url = Some(v);
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
        if let Some(v) = other.sync.auto_detect_sample_rate {
            self.sync.auto_detect_sample_rate = Some(v);
        }
        if let Some(v) = other.general.backup_enabled {
            self.general.backup_enabled = Some(v);
        }
        if let Some(v) = other.general.max_concurrent_jobs {
            self.general.max_concurrent_jobs = Some(v);
        }
        if let Some(v) = other.general.task_timeout_seconds {
            self.general.task_timeout_seconds = Some(v);
        }
        if let Some(v) = other.general.enable_progress_bar {
            self.general.enable_progress_bar = Some(v);
        }
        if let Some(v) = other.general.worker_idle_timeout_seconds {
            self.general.worker_idle_timeout_seconds = Some(v);
        }
        if let Some(v) = other.parallel.task_queue_size {
            self.parallel.task_queue_size = Some(v);
        }
        if let Some(v) = other.parallel.enable_task_priorities {
            self.parallel.enable_task_priorities = Some(v);
        }
        if let Some(v) = other.parallel.auto_balance_workers {
            self.parallel.auto_balance_workers = Some(v);
        }
        if let Some(v) = other.parallel.queue_overflow_strategy {
            self.parallel.queue_overflow_strategy = Some(v);
        }
        Ok(())
    }
}

impl PartialConfig {
    /// 轉換為完整配置，使用預設值填充缺少的欄位
    pub fn to_complete_config(
        &self,
    ) -> Result<crate::config::Config, crate::config::manager::ConfigError> {
        use crate::config::{
            AIConfig, Config, FormatsConfig, GeneralConfig, ParallelConfig, SyncConfig,
        };
        let default = Config::default();

        let ai = AIConfig {
            provider: self.ai.provider.clone().unwrap_or(default.ai.provider),
            api_key: self.ai.api_key.clone().or(default.ai.api_key),
            model: self.ai.model.clone().unwrap_or(default.ai.model),
            base_url: self.ai.base_url.clone().unwrap_or(default.ai.base_url),
            max_sample_length: self
                .ai
                .max_sample_length
                .unwrap_or(default.ai.max_sample_length),
            temperature: self.ai.temperature.unwrap_or(default.ai.temperature),
            retry_attempts: self.ai.retry_attempts.unwrap_or(default.ai.retry_attempts),
            retry_delay_ms: self.ai.retry_delay_ms.unwrap_or(default.ai.retry_delay_ms),
        };

        let formats = FormatsConfig {
            default_output: self
                .formats
                .default_output
                .clone()
                .unwrap_or(default.formats.default_output),
            preserve_styling: self
                .formats
                .preserve_styling
                .unwrap_or(default.formats.preserve_styling),
            default_encoding: self
                .formats
                .default_encoding
                .clone()
                .unwrap_or(default.formats.default_encoding.clone()),
            encoding_detection_confidence: self
                .formats
                .encoding_detection_confidence
                .unwrap_or(default.formats.encoding_detection_confidence),
        };

        let sync = SyncConfig {
            max_offset_seconds: self
                .sync
                .max_offset_seconds
                .unwrap_or(default.sync.max_offset_seconds),
            audio_sample_rate: self
                .sync
                .audio_sample_rate
                .unwrap_or(default.sync.audio_sample_rate),
            correlation_threshold: self
                .sync
                .correlation_threshold
                .unwrap_or(default.sync.correlation_threshold),
            dialogue_detection_threshold: self
                .sync
                .dialogue_detection_threshold
                .unwrap_or(default.sync.dialogue_detection_threshold),
            min_dialogue_duration_ms: self
                .sync
                .min_dialogue_duration_ms
                .unwrap_or(default.sync.min_dialogue_duration_ms),
            dialogue_merge_gap_ms: self
                .sync
                .dialogue_merge_gap_ms
                .unwrap_or(default.sync.dialogue_merge_gap_ms),
            enable_dialogue_detection: self
                .sync
                .enable_dialogue_detection
                .unwrap_or(default.sync.enable_dialogue_detection),
            auto_detect_sample_rate: self
                .sync
                .auto_detect_sample_rate
                .unwrap_or(default.sync.auto_detect_sample_rate),
        };

        let general = GeneralConfig {
            backup_enabled: self
                .general
                .backup_enabled
                .unwrap_or(default.general.backup_enabled),
            max_concurrent_jobs: self
                .general
                .max_concurrent_jobs
                .unwrap_or(default.general.max_concurrent_jobs),
            task_timeout_seconds: self
                .general
                .task_timeout_seconds
                .unwrap_or(default.general.task_timeout_seconds),
            enable_progress_bar: self
                .general
                .enable_progress_bar
                .unwrap_or(default.general.enable_progress_bar),
            worker_idle_timeout_seconds: self
                .general
                .worker_idle_timeout_seconds
                .unwrap_or(default.general.worker_idle_timeout_seconds),
        };

        let parallel = ParallelConfig {
            task_queue_size: self
                .parallel
                .task_queue_size
                .unwrap_or(default.parallel.task_queue_size),
            enable_task_priorities: self
                .parallel
                .enable_task_priorities
                .unwrap_or(default.parallel.enable_task_priorities),
            auto_balance_workers: self
                .parallel
                .auto_balance_workers
                .unwrap_or(default.parallel.auto_balance_workers),
            queue_overflow_strategy: self
                .parallel
                .queue_overflow_strategy
                .unwrap_or(default.parallel.queue_overflow_strategy),
        };
        Ok(Config {
            ai,
            formats,
            sync,
            general,
            parallel,
            loaded_from: None,
        })
    }
}
