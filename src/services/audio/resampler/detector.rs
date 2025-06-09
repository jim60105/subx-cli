//! 基於 aus 的採樣率檢測器

use crate::{Result, error::SubXError};
use aus::AudioFile;
use std::path::Path;

/// 音訊使用案例
#[derive(Debug, Clone, Copy)]
pub enum AudioUseCase {
    /// 語音識別
    SpeechRecognition,
    /// 音樂分析
    MusicAnalysis,
    /// 同步匹配
    SyncMatching,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config;
    use std::path::Path;

    #[tokio::test]
    async fn test_auto_detect_disabled_uses_config_rate() {
        let detector = AusSampleRateDetector::new();
        let mut cfg = load_config().unwrap().sync;
        cfg.auto_detect_sample_rate = false;
        let rate = detector
            .auto_detect_if_enabled(Path::new("nonexistent.wav"), &cfg)
            .await
            .unwrap();
        assert_eq!(rate, cfg.audio_sample_rate);
    }

    #[tokio::test]
    async fn test_auto_detect_failure_fallback() {
        let detector = AusSampleRateDetector::new();
        let cfg = load_config().unwrap().sync;
        // cfg.auto_detect_sample_rate 默認為 true
        let rate = detector
            .auto_detect_if_enabled(Path::new("nonexistent.wav"), &cfg)
            .await
            .unwrap();
        assert_eq!(rate, cfg.audio_sample_rate);
    }
}

/// 基於 aus 的採樣率檢測器
pub struct AusSampleRateDetector;

impl AusSampleRateDetector {
    /// 建立新的檢測器
    pub fn new() -> Self {
        Self
    }

    /// 使用 aus 檢測音訊檔案的採樣率
    pub async fn detect_sample_rate<P: AsRef<Path>>(&self, audio_path: P) -> Result<u32> {
        let path = audio_path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| SubXError::audio_processing("無法轉換路徑為 UTF-8 字串"))?;
        let audio_file = aus::read(path_str)?;
        Ok(audio_file.sample_rate as u32)
    }

    /// 從 AudioFile 獲取採樣率
    pub fn detect_from_audio_file(&self, audio_file: &AudioFile) -> u32 {
        audio_file.sample_rate
    }

    /// 驗證採樣率是否受支援
    pub fn is_supported_rate(&self, sample_rate: u32) -> bool {
        matches!(sample_rate, 8000..=192000)
    }

    /// 取得建議的採樣率
    pub fn get_recommended_rate(&self, _source_rate: u32, target_use: AudioUseCase) -> u32 {
        let _ = _source_rate;
        match target_use {
            AudioUseCase::SpeechRecognition => 16000,
            AudioUseCase::MusicAnalysis => 44100,
            AudioUseCase::SyncMatching => 22050,
        }
    }

    /// 若啟用 auto_detect_sample_rate，嘗試自動檢測，否則使用配置預設
    pub async fn auto_detect_if_enabled<P: AsRef<Path>>(
        &self,
        audio_path: P,
        config: &crate::config::SyncConfig,
    ) -> crate::Result<u32> {
        if config.auto_detect_sample_rate {
            match self.detect_sample_rate(audio_path).await {
                Ok(rate) => {
                    log::info!("自動檢測到採樣率: {}Hz", rate);
                    Ok(rate)
                }
                Err(err) => {
                    log::warn!(
                        "採樣率自動檢測失敗 ({}), 使用預設 {}Hz",
                        err,
                        config.audio_sample_rate
                    );
                    Ok(config.audio_sample_rate)
                }
            }
        } else {
            log::debug!(
                "auto_detect_sample_rate=false, 使用配置採樣率 {}Hz",
                config.audio_sample_rate
            );
            Ok(config.audio_sample_rate)
        }
    }
}
