//! aus crate 適配器模組

use crate::{error::SubXError, Result};
use aus::AudioFile;
use std::path::Path;

/// 將 SubX AudioData 轉換為 aus AudioFile 的適配器
pub struct AusAdapter {
    sample_rate: u32,
}

impl AusAdapter {
    /// 建立新的 AusAdapter
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// 讀取音訊檔案為 aus AudioFile
    pub fn read_audio_file<P: AsRef<Path>>(&self, path: P) -> Result<AudioFile> {
        let path_ref = path.as_ref();
        let path_str = path_ref
            .to_str()
            .ok_or_else(|| SubXError::audio_processing("無法轉換路徑為 UTF-8 字串"))?;
        aus::read(path_str)
            .map_err(|e| SubXError::audio_processing(format!("aus 讀取音訊檔案失敗: {:?}", e)))
    }

    /// 將 AudioFile 轉換為 SubX 相容的 AudioData
    pub fn to_subx_audio_data(
        &self,
        _audio_file: &AudioFile,
    ) -> Result<crate::services::audio::AudioData> {
        // 實作轉換邏輯，後續階段完善
        todo!("將在後續階段實作 aus to AudioData 轉換")
    }
}
