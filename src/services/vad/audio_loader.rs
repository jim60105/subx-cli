//! 直接音訊載入器：使用 Symphonia 直接解碼多種音訊格式並取得 i16 樣本資料。
//!
//! 支援 MP4、MKV、OGG、WAV 等格式，回傳樣本資料與音訊資訊。
use crate::{error::SubXError, Result};
use symphonia::core::codecs::CodecRegistry;
use symphonia::core::probe::Probe;
use symphonia::default::{get_codecs, get_probe};
use std::path::Path;
use crate::services::vad::detector::AudioInfo;

/// 直接音訊載入器，使用 Symphonia 解碼取得原始樣本資料。
pub struct DirectAudioLoader {
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

impl DirectAudioLoader {
    /// 建立新的音訊載入器實例。
    pub fn new() -> Result<Self> {
        Ok(Self {
            probe: get_probe(),
            codecs: get_codecs(),
        })
    }

    /// 從音訊檔案路徑載入 i16 樣本與音訊資訊。
    pub fn load_audio_samples<P: AsRef<Path>>(
        &self,
        _path: P,
    ) -> Result<(Vec<i16>, AudioInfo)> {
        // TODO: 使用 Symphonia 直接解碼，取得 samples 與 AudioInfo
        Err(SubXError::audio_processing(
            "Direct audio loading 尚未實作".to_string(),
        ))
    }
}
