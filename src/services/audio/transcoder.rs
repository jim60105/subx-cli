//! 音訊轉碼服務：基於 Symphonia 的多格式轉 WAV 機制。

use crate::{Result, error::SubXError};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::{
    audio::{Layout, SampleBuffer},
    codecs::CODEC_TYPE_NULL,
    errors::Error as SymphoniaError,
    io::MediaSourceStream,
};
use symphonia::core::{codecs::CodecRegistry, probe::Probe};
use symphonia::default::{get_codecs, get_probe};
use tempfile::TempDir;
/// 音訊轉碼器：檢測檔案格式並將非 WAV 檔案轉為 WAV。
pub struct AudioTranscoder {
    /// 臨時目錄，用於存放轉碼結果
    temp_dir: TempDir,
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    /// Create a minimal WAV file for testing transcoding.
    fn create_minimal_wav_file(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("test.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        writer.write_sample(0i16).unwrap();
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn test_needs_transcoding() {
        let transcoder = AudioTranscoder::new().expect("Failed to create transcoder");
        assert!(transcoder.needs_transcoding("test.mp4").unwrap());
        assert!(transcoder.needs_transcoding("test.MKV").unwrap());
        assert!(transcoder.needs_transcoding("test.ogg").unwrap());
        assert!(!transcoder.needs_transcoding("test.wav").unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn test_transcode_wav_to_wav() {
        let transcoder = AudioTranscoder::new().expect("Failed to create transcoder");
        let temp_dir = TempDir::new().unwrap();
        let wav_path = create_minimal_wav_file(&temp_dir);
        let out_path = transcoder
            .transcode_to_wav(&wav_path)
            .await
            .expect("Transcode failed");
        assert_eq!(out_path.extension().and_then(|e| e.to_str()), Some("wav"));
        let meta = std::fs::metadata(&out_path).expect("Failed to stat output file");
        assert!(meta.len() > 0, "Output WAV file should not be empty");
    }
}

impl AudioTranscoder {
    /// 建立新的 AudioTranscoder 實例，並初始化暫存資料夾。
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().map_err(|e| {
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

    /// 將輸入音訊檔案轉碼為 WAV，並儲存於臨時目錄中。
    pub async fn transcode_to_wav<P: AsRef<Path>>(&self, input_path: P) -> Result<PathBuf> {
        let input = input_path.as_ref();
        // 開啟原始音訊檔案
        let file = File::open(input).map_err(|e| {
            SubXError::audio_processing(format!(
                "Failed to open input file {}: {}",
                input.display(),
                e
            ))
        })?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        // 偵測格式並建立 FormatReader
        let probed = self
            .probe
            .format(
                &Default::default(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .map_err(|e| SubXError::audio_processing(format!("Format probe error: {}", e)))?;
        let mut format = probed.format;
        // 選擇第一個有效音軌
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SubXError::audio_processing("No audio track found".to_string()))?;
        // 建立解碼器
        let mut decoder = self
            .codecs
            .make(&track.codec_params, &Default::default())
            .map_err(|e| SubXError::audio_processing(format!("Decoder error: {}", e)))?;
        // 設定 WAV 寫入規格
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        // 根據 channel_layout 決定聲道數量，若未知則預設為立體聲
        let layout = track.codec_params.channel_layout.unwrap_or(Layout::Stereo);
        let channels = layout.into_channels().count() as u16;
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let wav_path = self
            .temp_dir
            .path()
            .join(input.file_stem().unwrap_or_default())
            .with_extension("wav");
        let mut writer = WavWriter::create(&wav_path, spec)
            .map_err(|e| SubXError::audio_processing(format!("WAV writer error: {}", e)))?;
        // 解碼並寫入 WAV
        loop {
            match format.next_packet() {
                Ok(packet) => {
                    let audio_buf = decoder
                        .decode(&packet)
                        .map_err(|e| SubXError::audio_processing(format!("Decode error: {}", e)))?;
                    let mut sample_buf =
                        SampleBuffer::<i16>::new(audio_buf.capacity() as u64, *audio_buf.spec());
                    sample_buf.copy_interleaved_ref(audio_buf);
                    for sample in sample_buf.samples() {
                        writer.write_sample(*sample).map_err(|e| {
                            SubXError::audio_processing(format!("Write sample error: {}", e))
                        })?;
                    }
                }
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    return Err(SubXError::audio_processing(format!(
                        "Packet read error: {}",
                        e
                    )));
                }
            }
        }
        writer
            .finalize()
            .map_err(|e| SubXError::audio_processing(format!("Finalize WAV error: {}", e)))?;
        Ok(wav_path)
    }

    /// 主動清理臨時目錄
    pub fn cleanup(self) -> Result<()> {
        self.temp_dir
            .close()
            .map_err(|e| SubXError::audio_processing(format!("Failed to clean temp dir: {}", e)))?;
        Ok(())
    }
}
