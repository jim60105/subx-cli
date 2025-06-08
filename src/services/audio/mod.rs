//! SubX 音訊服務模組

use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

use crate::error::SubXError;
use crate::Result;

/// 音訊分析器
pub struct AudioAnalyzer {
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
}

pub mod resampler;

pub use resampler::{
    AudioResampler, OptimizationResult, ResampleConfig, ResampleQuality, SampleRateDetector,
    SampleRateOptimizer,
};

/// 音訊能量包絡
#[derive(Debug, Clone)]
pub struct AudioEnvelope {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: f32,
}

/// 對話段落
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f32,
    pub end_time: f32,
    pub intensity: f32,
}

/// 音訊原始資料元資料
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: f32,
}

/// 音訊原始樣本資料
#[derive(Debug, Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration: f32,
}

impl AudioAnalyzer {
    /// 建立分析器，設定採樣率
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            window_size: 1024,
            hop_size: 512,
        }
    }

    /// 提取音訊能量包絡
    pub async fn extract_envelope(&self, audio_path: &Path) -> Result<AudioEnvelope> {
        let file = std::fs::File::open(audio_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let format_opts = FormatOptions::default();
        let metadata_opts = Default::default();
        let hint = Hint::new();
        let probed = get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or_else(|| SubXError::audio_processing("找不到音訊軌道"))?;
        let track_id = track.id;
        let track_sr = track.codec_params.sample_rate.unwrap_or(self.sample_rate);

        let decoder_opts = DecoderOptions::default();
        let mut decoder = get_codecs().make(&track.codec_params, &decoder_opts)?;

        let mut samples = Vec::new();
        let mut total_duration = 0.0;

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() == track_id {
                let audio_buf = decoder.decode(&packet)?;
                let envelope_chunk = self.extract_energy_from_buffer(&audio_buf);
                samples.extend(envelope_chunk);
                total_duration += packet.dur as f32 / track_sr as f32;
            }
        }

        Ok(AudioEnvelope {
            samples,
            sample_rate: self.sample_rate,
            duration: total_duration,
        })
    }

    fn extract_energy_from_buffer(&self, audio_buf: &AudioBufferRef) -> Vec<f32> {
        // 轉換為 f32 線性樣本緩衝
        let spec = *audio_buf.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(audio_buf.clone());
        let samples = sample_buf.samples();
        let channels = spec.channels.count();
        let mut energy_samples = Vec::new();

        // 每 hop_size 幀計算一次 RMS 能量
        for chunk in samples.chunks(self.hop_size * channels) {
            let mut sum_squares = 0.0;
            let mut count = 0;
            for frame in chunk.chunks(channels) {
                let mono = frame.iter().sum::<f32>() / channels as f32;
                sum_squares += mono * mono;
                count += 1;
            }
            let rms = if count > 0 {
                (sum_squares / count as f32).sqrt()
            } else {
                0.0
            };
            energy_samples.push(rms);
        }

        energy_samples
    }
    /// 偵測對話段落
    pub fn detect_dialogue(
        &self,
        envelope: &AudioEnvelope,
        threshold: f32,
    ) -> Vec<DialogueSegment> {
        let mut segments = Vec::new();
        let mut in_dialogue = false;
        let mut start = 0.0;
        let time_per_sample = envelope.duration / envelope.samples.len() as f32;

        for (i, &e) in envelope.samples.iter().enumerate() {
            let t = i as f32 * time_per_sample;
            if e > threshold && !in_dialogue {
                in_dialogue = true;
                start = t;
            } else if e <= threshold && in_dialogue {
                in_dialogue = false;
                if t - start > 0.5 {
                    segments.push(DialogueSegment {
                        start_time: start,
                        end_time: t,
                        intensity: e,
                    });
                }
            }
        }

        segments
    }

    /// 載入音訊檔案並回傳原始樣本資料
    pub async fn load_audio_file<P: AsRef<std::path::Path>>(
        &self,
        audio_path: P,
    ) -> crate::Result<AudioData> {
        // 讀取並解碼音訊檔案，回傳原始樣本資料
        let path = audio_path.as_ref();
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let format_opts = FormatOptions::default();
        let metadata_opts = Default::default();
        let hint = Hint::new();
        let probed = get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or_else(|| SubXError::audio_processing("找不到音訊軌道"))?;
        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap();
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);

        let mut decoder = get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
        let mut samples = Vec::new();

        while let Ok(packet) = format.next_packet() {
            if packet.track_id() == track_id {
                let audio_buf = decoder.decode(&packet)?;
                let mut sample_buf =
                    SampleBuffer::<f32>::new(audio_buf.capacity() as u64, *audio_buf.spec());
                sample_buf.copy_interleaved_ref(audio_buf.clone());
                samples.extend(sample_buf.samples());
            }
        }

        let duration = samples.len() as f32 / (sample_rate as f32 * channels as f32);
        Ok(AudioData {
            samples,
            sample_rate,
            channels,
            duration,
        })
    }

    /// 使用最佳採樣率分析音訊
    pub async fn analyze_with_optimal_rate<P: AsRef<std::path::Path>>(
        &mut self,
        audio_path: P,
    ) -> crate::Result<AudioData> {
        // 1. 載入音訊檔案
        let audio_data = self.load_audio_file(audio_path).await?;
        // 2. 最佳化採樣率
        let optimizer = SampleRateOptimizer::new()?;
        let auto_opt = optimizer.auto_optimize(&audio_data).await?;
        // 3. 如需重採樣則執行
        if let Some(sugg) = auto_opt.optimization_result.optimization {
            let config = ResampleConfig::new(sugg.recommended_rate);
            let mut resampler = AudioResampler::new(config)?;
            resampler.resample(&audio_data, sugg.recommended_rate)
        } else {
            Ok(audio_data)
        }
    }
}
