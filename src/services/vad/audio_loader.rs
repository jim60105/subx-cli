//! 直接音訊載入器：使用 Symphonia 直接解碼多種音訊格式並取得 i16 樣本資料。
//!
//! 支援 MP4、MKV、OGG、WAV 等格式，回傳樣本資料與音訊資訊。
use crate::services::vad::detector::AudioInfo;
use crate::{Result, error::SubXError};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CodecRegistry;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::probe::Probe;
use symphonia::default::{get_codecs, get_probe};

/// 直接音訊載入器，使用 Symphonia 解碼取得原始樣本資料。
pub struct DirectAudioLoader {
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_mp4_loading() {
        // 使用 assets/SubX - The Subtitle Revolution.mp4 測試直接音訊載入
        let loader = DirectAudioLoader::new().expect("初始化 DirectAudioLoader 失敗");
        let (samples, info) = loader
            .load_audio_samples("assets/SubX - The Subtitle Revolution.mp4")
            .expect("load_audio_samples 失敗");
        assert!(!samples.is_empty(), "樣本資料不應為空");
        assert!(info.sample_rate > 0, "sample_rate 應大於 0");
        assert!(info.total_samples > 0, "total_samples 應大於 0");
    }
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
    pub fn load_audio_samples<P: AsRef<Path>>(&self, path: P) -> Result<(Vec<i16>, AudioInfo)> {
        let path_ref = path.as_ref();
        // Open the media source.
        let file = File::open(path_ref).map_err(|e| {
            SubXError::audio_processing(format!("Failed to open audio file: {}", e))
        })?;

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help format probing based on file extension.
        let mut hint = Hint::new();
        if let Some(ext) = path_ref.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // Probe the media format.
        let probed = self
            .probe
            .format(&hint, mss, &FormatOptions::default(), &Default::default())
            .map_err(|e| SubXError::audio_processing(format!("Failed to probe format: {}", e)))?;
        let mut format = probed.format;

        // 選擇第一個包含 sample_rate 的音訊軌道作為音訊來源。
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or_else(|| SubXError::audio_processing("No audio track found".to_string()))?;
        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| SubXError::audio_processing("Sample rate unknown".to_string()))?;
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(1);

        // Create decoder for the track.
        let dec_opts = DecoderOptions::default();
        let mut decoder = self
            .codecs
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| SubXError::audio_processing(format!("Failed to create decoder: {}", e)))?;

        // Decode packets and collect samples.
        let mut samples = Vec::new();
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = decoder
                .decode(&packet)
                .map_err(|e| SubXError::audio_processing(format!("Decode error: {}", e)))?;
            // Create a sample buffer for this packet using its signal spec and capacity.
            let spec = *decoded.spec();
            let mut sample_buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sample_buf.samples());
        }

        // 計算音訊總樣本與長度
        let total_samples = samples.len();
        let duration_seconds = total_samples as f64 / (sample_rate as f64 * channels as f64);

        Ok((
            samples,
            AudioInfo {
                sample_rate,
                channels,
                duration_seconds,
                total_samples,
            },
        ))
    }
}
