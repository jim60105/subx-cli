//! 音訊採樣率檢測器
#![allow(dead_code, unused_imports)]

use crate::services::audio::{AudioData, AudioMetadata};
use crate::Result;
use std::fs::File;
use std::path::Path;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;

/// 採樣率檢測器
pub struct SampleRateDetector {
    supported_rates: Vec<u32>,
    auto_detect_enabled: bool,
}

impl SampleRateDetector {
    pub fn new() -> Self {
        Self {
            supported_rates: vec![
                8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 192000,
            ],
            auto_detect_enabled: true,
        }
    }

    /// 檢測音訊檔案的採樣率
    pub async fn detect_sample_rate<P: AsRef<Path>>(&self, audio_path: P) -> Result<u32> {
        // 讀取檔頭並解析採樣率
        let file = File::open(audio_path.as_ref())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let format_opts = FormatOptions::default();
        let metadata_opts = Default::default();
        let hint = Hint::new();
        let probed = get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or_else(|| {
                crate::error::SubXError::audio_processing(
                    "找不到音訊軌道 for sample rate detection",
                )
            })?;
        let rate = track.codec_params.sample_rate.unwrap();
        if self.is_supported_rate(rate) {
            Ok(rate)
        } else {
            Err(crate::error::SubXError::audio_processing(
                format!("不支援的採樣率: {}", rate),
            ))
        }
    }

    /// 檢測音訊資料的採樣率
    pub fn detect_from_data(&self, audio_data: &AudioData) -> Result<u32> {
        // 從 AudioData 直接使用其 sample_rate
        let rate = audio_data.sample_rate;
        if self.is_supported_rate(rate) {
            Ok(rate)
        } else {
            Err(crate::error::SubXError::audio_processing(
                format!("不支援的採樣率: {}", rate),
            ))
        }
    }

    /// 驗證採樣率是否受支援
    pub fn is_supported_rate(&self, sample_rate: u32) -> bool {
        self.supported_rates.contains(&sample_rate)
    }

    /// 取得建議的採樣率
    pub fn get_recommended_rate(&self, source_rate: u32, target_use: AudioUseCase) -> u32 {
        match target_use {
            AudioUseCase::SpeechRecognition => self.optimize_for_speech(source_rate),
            AudioUseCase::MusicAnalysis => self.optimize_for_music(source_rate),
            AudioUseCase::SyncMatching => self.optimize_for_sync(source_rate),
        }
    }

    fn optimize_for_speech(&self, source_rate: u32) -> u32 {
        // 語音處理最佳化：通常 16kHz 已足夠
        match source_rate {
            rate if rate <= 16000 => rate,
            _ => 16000,
        }
    }

    fn optimize_for_music(&self, source_rate: u32) -> u32 {
        // 音樂分析最佳化：保持較高品質
        match source_rate {
            rate if rate >= 44100 => rate,
            _ => 44100,
        }
    }

    fn optimize_for_sync(&self, source_rate: u32) -> u32 {
        // 同步匹配最佳化：平衡精度和效能
        match source_rate {
            rate if rate <= 22050 => 22050,
            rate if rate <= 44100 => 44100,
            _ => 48000,
        }
    }
}

/// 使用場景
#[derive(Debug, Clone, Copy)]
pub enum AudioUseCase {
    SpeechRecognition,
    MusicAnalysis,
    SyncMatching,
}
