use crate::services::audio::AudioTranscoder;
use crate::{Result, error::SubXError};
use std::path::Path;
use std::time::Duration;
use tempfile::NamedTempFile;

/// 音訊片段擷取器，用於 Whisper API 分析
pub struct AudioSegmentExtractor {
    transcoder: AudioTranscoder,
}

impl AudioSegmentExtractor {
    /// 建立擷取器
    pub fn new() -> Result<Self> {
        Ok(Self {
            transcoder: AudioTranscoder::new()?,
        })
    }

    /// 提取以中心時間為基準的音訊片段
    pub async fn extract_segment(
        &self,
        audio_path: &Path,
        center_time: Duration,
        window_seconds: u32,
    ) -> Result<std::path::PathBuf> {
        let half = Duration::from_secs(window_seconds as u64) / 2;
        let start = if center_time > half {
            center_time - half
        } else {
            Duration::ZERO
        };
        let end = center_time + half;

        let tmp = NamedTempFile::with_suffix(".wav").map_err(|e| {
            SubXError::audio_extraction(format!("Failed to create temp file: {}", e))
        })?;
        let out = tmp.path().to_path_buf();

        self.transcoder
            .extract_segment(audio_path, &out, start, end)
            .await?;
        Ok(out)
    }

    /// 轉碼音訊至 Whisper 建議格式
    pub async fn prepare_for_whisper(&self, audio_path: &Path) -> Result<std::path::PathBuf> {
        let tmp = NamedTempFile::with_suffix(".wav").map_err(|e| {
            SubXError::audio_extraction(format!("Failed to create temp file: {}", e))
        })?;
        let out = tmp.path().to_path_buf();

        self.transcoder
            .transcode_to_format(audio_path, &out, 16000, 1)
            .await?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_extract_segment_creates_file() {
        let extractor = AudioSegmentExtractor::new().unwrap();
        // 使用不存在的檔案會錯誤處理
        let err = extractor
            .extract_segment(&std::path::Path::new("no.wav"), Duration::from_secs(0), 1)
            .await;
        assert!(err.is_err());
    }
}
