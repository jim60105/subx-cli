//! 基於 aus 的採樣率檢測器 (v2)

use crate::services::audio::resampler::detector::AudioUseCase;
use crate::{error::SubXError, Result};
use aus::AudioFile;
use std::path::Path;

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
}
