//! 音訊轉碼服務：基於 Symphonia 的多格式轉 WAV 機制。

use crate::{Result, error::SubXError};
use std::fs;
use std::path::{Path, PathBuf};
use symphonia::core::{codecs::CodecRegistry, probe::Probe};
use symphonia::default::{get_codecs, get_probe};

/// 音訊轉碼器：檢測檔案格式並將非 WAV 檔案轉為 WAV。
pub struct AudioTranscoder {
    temp_dir: PathBuf,
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

impl AudioTranscoder {
    /// 建立新的 AudioTranscoder 實例，並初始化暫存資料夾。
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("subx_audio_transcode");
        fs::create_dir_all(&temp_dir).map_err(|e| {
            SubXError::audio_processing(format!("Failed to create temp dir: {}", e))
        })?;
        let probe = get_probe();
        let codecs = get_codecs();
        Ok(Self {
            temp_dir,
            probe,
            codecs,
        })
    }

    /// 檢查指定路徑的音訊檔案是否需要轉碼（基於副檔名判斷）。
    pub fn needs_transcoding<P: AsRef<Path>>(&self, audio_path: P) -> Result<bool> {
        if let Some(ext) = audio_path.as_ref().extension().and_then(|s| s.to_str()) {
            let ext_lc = ext.to_lowercase();
            if ext_lc == "wav" { Ok(false) } else { Ok(true) }
        } else {
            Err(SubXError::audio_processing(
                "Missing file extension".to_string(),
            ))
        }
    }

    /// 將輸入音訊檔案轉碼為 WAV。尚未實作，僅回傳未實作錯誤。
    pub async fn transcode_to_wav<P: AsRef<Path>>(&self, _input_path: P) -> Result<PathBuf> {
        Err(SubXError::audio_processing(
            "transcode_to_wav not implemented".to_string(),
        ))
    }

    /// 清理暫存目錄及其內容。
    pub fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir).map_err(|e| {
                SubXError::audio_processing(format!("Failed to clean temp dir: {}", e))
            })?;
        }
        Ok(())
    }
}
