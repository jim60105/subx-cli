//! 音訊重採樣轉換器
#![allow(dead_code, unused_imports)]

use crate::config::{load_config, SyncConfig};
use crate::error::SubXError;
use crate::services::audio::AudioData;
use crate::Result;

/// 重採樣配置結構
#[derive(Debug, Clone)]
pub struct ResampleConfig {
    pub target_sample_rate: u32,
    pub quality: ResampleQuality,
    pub preserve_duration: bool,
    pub anti_aliasing: bool,
    pub normalize_volume: bool,
}

/// 重採樣品質等級
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleQuality {
    Low,    // 快速但品質較低
    Medium, // 平衡品質和速度
    High,   // 高品質但較慢
    Best,   // 最佳品質但最慢
}

impl ResampleQuality {
    pub fn from_string(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" | "fast" => Ok(Self::Low),
            "medium" | "normal" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "best" | "highest" => Ok(Self::Best),
            _ => Err(SubXError::config(format!("無效的重採樣品質設定: {}", s))),
        }
    }

    pub fn to_ratio_precision(&self) -> f64 {
        match self {
            Self::Low => 0.85,
            Self::Medium => 0.92,
            Self::High => 0.97,
            Self::Best => 0.99,
        }
    }
}

impl ResampleConfig {
    pub fn new(target_rate: u32) -> Self {
        Self {
            target_sample_rate: target_rate,
            quality: ResampleQuality::High,
            preserve_duration: true,
            anti_aliasing: true,
            normalize_volume: false,
        }
    }

    pub fn from_config() -> Result<Self> {
        let config = load_config()?;
        Ok(Self {
            target_sample_rate: config.sync.audio_sample_rate,
            quality: ResampleQuality::from_string(&config.sync.resample_quality())?,
            preserve_duration: true,
            anti_aliasing: true,
            normalize_volume: false,
        })
    }

    pub fn for_speech() -> Self {
        Self {
            target_sample_rate: 16000,
            quality: ResampleQuality::Medium,
            preserve_duration: true,
            anti_aliasing: true,
            normalize_volume: true,
        }
    }

    pub fn for_sync() -> Self {
        Self {
            target_sample_rate: 22050,
            quality: ResampleQuality::High,
            preserve_duration: true,
            anti_aliasing: true,
            normalize_volume: false,
        }
    }
}

/// 音訊重採樣器
pub struct AudioResampler {
    config: ResampleConfig,
    interpolator: Box<dyn Interpolator>,
    buffer: std::collections::VecDeque<f32>,
}

impl AudioResampler {
    pub fn new(config: ResampleConfig) -> Result<Self> {
        let interpolator = Self::create_interpolator(&config)?;
        Ok(Self {
            config,
            interpolator,
            buffer: std::collections::VecDeque::new(),
        })
    }

    /// 重採樣音訊資料
    pub fn resample(&mut self, input: &AudioData, target_rate: u32) -> Result<AudioData> {
        if input.sample_rate == target_rate {
            return Ok(input.clone());
        }

        let ratio = target_rate as f64 / input.sample_rate as f64;
        let output_length = (input.samples.len() as f64 * ratio) as usize;
        let resampled_samples =
            self.interpolator
                .interpolate(&input.samples, ratio, output_length)?;

        Ok(AudioData {
            samples: resampled_samples,
            sample_rate: target_rate,
            channels: input.channels,
            duration: input.duration,
        })
    }

    /// 批次重採樣多個音訊資料
    pub async fn resample_batch(
        &mut self,
        files: Vec<AudioData>,
        target_rate: u32,
    ) -> Result<Vec<AudioData>> {
        let mut results = Vec::with_capacity(files.len());
        for audio_data in files {
            results.push(self.resample(&audio_data, target_rate)?);
        }
        Ok(results)
    }

    fn create_interpolator(config: &ResampleConfig) -> Result<Box<dyn Interpolator>> {
        match config.quality {
            ResampleQuality::Low => Ok(Box::new(LinearInterpolator::new())),
            ResampleQuality::Medium => Ok(Box::new(CubicInterpolator::new())),
            ResampleQuality::High => Ok(Box::new(SincInterpolator::new(8))),
            ResampleQuality::Best => Ok(Box::new(SincInterpolator::new(16))),
        }
    }
}

/// 插值器特質定義
trait Interpolator: Send + Sync {
    fn interpolate(&self, input: &[f32], ratio: f64, output_length: usize) -> Result<Vec<f32>>;
}

/// 線性插值器（快速但品質較低）
struct LinearInterpolator;

impl LinearInterpolator {
    fn new() -> Self {
        Self
    }
}

impl Interpolator for LinearInterpolator {
    fn interpolate(&self, input: &[f32], ratio: f64, output_length: usize) -> Result<Vec<f32>> {
        let mut output = Vec::with_capacity(output_length);
        for i in 0..output_length {
            let src_index = i as f64 / ratio;
            let index = src_index as usize;
            let fraction = src_index - index as f64;
            if index + 1 < input.len() {
                let sample =
                    input[index] * (1.0 - fraction as f32) + input[index + 1] * fraction as f32;
                output.push(sample);
            } else if index < input.len() {
                output.push(input[index]);
            } else {
                output.push(0.0);
            }
        }
        Ok(output)
    }
}

/// 三次插值器（中等品質和速度）
struct CubicInterpolator;

impl CubicInterpolator {
    fn new() -> Self {
        Self
    }

    fn cubic_interpolate(&self, y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
        let a0 = y3 - y2 - y0 + y1;
        let a1 = y0 - y1 - a0;
        let a2 = y2 - y0;
        let a3 = y1;
        a0 * mu * mu * mu + a1 * mu * mu + a2 * mu + a3
    }
}

impl Interpolator for CubicInterpolator {
    fn interpolate(&self, input: &[f32], ratio: f64, output_length: usize) -> Result<Vec<f32>> {
        let mut output = Vec::with_capacity(output_length);
        for i in 0..output_length {
            let src_index = i as f64 / ratio;
            let index = src_index as usize;
            let fraction = (src_index - index as f64) as f32;
            if index >= 1 && index + 2 < input.len() {
                let sample = self.cubic_interpolate(
                    input[index - 1],
                    input[index],
                    input[index + 1],
                    input[index + 2],
                    fraction,
                );
                output.push(sample);
            } else if index < input.len() {
                if index + 1 < input.len() {
                    let sample = input[index] * (1.0 - fraction) + input[index + 1] * fraction;
                    output.push(sample);
                } else {
                    output.push(input[index]);
                }
            } else {
                output.push(0.0);
            }
        }
        Ok(output)
    }
}

/// Sinc 插值器（最高品質）
struct SincInterpolator {
    kernel_size: usize,
}

impl SincInterpolator {
    fn new(kernel_size: usize) -> Self {
        Self { kernel_size }
    }
    fn sinc(&self, x: f64) -> f64 {
        if x.abs() < 1e-9 {
            1.0
        } else {
            let p = std::f64::consts::PI * x;
            p.sin() / p
        }
    }
    fn windowed_sinc(&self, x: f64) -> f64 {
        if x.abs() >= self.kernel_size as f64 {
            0.0
        } else {
            let window = 0.42 - 0.5 * (std::f64::consts::PI * x / self.kernel_size as f64).cos()
                + 0.08 * (2.0 * std::f64::consts::PI * x / self.kernel_size as f64).cos();
            self.sinc(x) * window
        }
    }
}

impl Interpolator for SincInterpolator {
    fn interpolate(&self, input: &[f32], ratio: f64, output_length: usize) -> Result<Vec<f32>> {
        let mut output = Vec::with_capacity(output_length);
        for i in 0..output_length {
            let src_index = i as f64 / ratio;
            let center = src_index as isize;
            let mut sample = 0.0f64;
            for j in -(self.kernel_size as isize)..=(self.kernel_size as isize) {
                let idx = center + j;
                if idx >= 0 && (idx as usize) < input.len() {
                    let w = self.windowed_sinc(src_index - idx as f64);
                    sample += input[idx as usize] as f64 * w;
                }
            }
            output.push(sample as f32);
        }
        Ok(output)
    }
}
