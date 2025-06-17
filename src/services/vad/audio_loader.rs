//! 直接音訊載入器：使用 Symphonia 直接解碼多種音訊格式並取得 i16 樣本資料。
//!
//! 支援 MP4、MKV、OGG、WAV 等格式，回傳樣本資料與音訊資訊。
use crate::services::vad::detector::AudioInfo;
use crate::{Result, error::SubXError};
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::CodecRegistry;
use symphonia::core::probe::Probe;
use symphonia::default::{get_codecs, get_probe};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

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

        // Get the default track.
        let track = format
            .default_track()
            .ok_or_else(|| SubXError::audio_processing("No default audio track found".to_string()))?;

        // Create decoder for the track.
        let dec_opts = DecoderOptions::default();
        let mut decoder = self
            .codecs
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| SubXError::audio_processing(format!("Failed to create decoder: {}", e)))?;

        // Prepare the sample buffer.
        let sample_rate = track.codec_params.sample_rate.ok_or_else(|| {
            SubXError::audio_processing("Sample rate unknown".to_string())
        })?;
        let mut sample_buf = SampleBuffer::<i16>::new(0, sample_rate);

        // Decode packets and collect samples.
        let mut samples = Vec::new();
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track.id {
                continue;
            }
            let decoded = decoder
                .decode(&packet)
                .map_err(|e| SubXError::audio_processing(format!("Decode error: {}", e)))?;
            sample_buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(sample_buf.samples());
        }

        // Gather audio info.
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(1);
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
