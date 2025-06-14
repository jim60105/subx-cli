use super::AudioInfo;
use crate::{Result, error::SubXError};
use hound::{SampleFormat, WavReader};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct VadAudioProcessor {
    target_sample_rate: u32,
    target_channels: u16,
}

#[derive(Debug)]
pub struct ProcessedAudioData {
    pub samples: Vec<i16>,
    pub info: AudioInfo,
}

impl VadAudioProcessor {
    pub fn new(target_sample_rate: u32, target_channels: u16) -> Result<Self> {
        Ok(Self {
            target_sample_rate,
            target_channels,
        })
    }

    pub async fn load_and_prepare_audio(&self, audio_path: &Path) -> Result<ProcessedAudioData> {
        // 1. 載入音訊檔案
        let raw_audio_data = self.load_wav_file(audio_path)?;

        // 2. 轉換採樣率（如果需要）
        let resampled_data = if raw_audio_data.info.sample_rate != self.target_sample_rate {
            self.resample_audio(&raw_audio_data)?
        } else {
            raw_audio_data
        };

        // 3. 轉換為單聲道（如果需要）
        let mono_data = if resampled_data.info.channels > 1 {
            self.convert_to_mono(&resampled_data)?
        } else {
            resampled_data
        };

        Ok(mono_data)
    }

    fn load_wav_file(&self, path: &Path) -> Result<ProcessedAudioData> {
        let file = File::open(path)
            .map_err(|e| SubXError::io(format!("Failed to open audio file: {}", e)))?;

        let reader = WavReader::new(BufReader::new(file))
            .map_err(|e| SubXError::audio_processing(format!("Failed to read WAV file: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels;

        // 讀取所有樣本並轉換為 i16
        let samples: Result<Vec<i16>> = match spec.sample_format {
            SampleFormat::Int => {
                match spec.bits_per_sample {
                    16 => reader
                        .into_samples::<i16>()
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| {
                            SubXError::audio_processing(format!("Failed to read samples: {}", e))
                        }),
                    32 => {
                        // 轉換 i32 到 i16
                        let i32_samples: Vec<i32> = reader
                            .into_samples::<i32>()
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| {
                                SubXError::audio_processing(format!(
                                    "Failed to read i32 samples: {}",
                                    e
                                ))
                            })?;
                        Ok(i32_samples.iter().map(|&s| (s >> 16) as i16).collect())
                    }
                    _ => Err(SubXError::audio_processing(format!(
                        "Unsupported bit depth: {}",
                        spec.bits_per_sample
                    ))),
                }
            }
            SampleFormat::Float => {
                // 轉換 f32 到 i16
                let f32_samples: Vec<f32> = reader
                    .into_samples::<f32>()
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| {
                        SubXError::audio_processing(format!("Failed to read f32 samples: {}", e))
                    })?;
                Ok(f32_samples.iter().map(|&s| (s * 32767.0) as i16).collect())
            }
        }?;

        let duration_seconds = samples.len() as f64 / (sample_rate as f64 * channels as f64);

        Ok(ProcessedAudioData {
            samples,
            info: AudioInfo {
                sample_rate,
                channels,
                duration_seconds,
                total_samples: samples.len(),
            },
        })
    }

    fn resample_audio(&self, audio_data: &ProcessedAudioData) -> Result<ProcessedAudioData> {
        if audio_data.info.sample_rate == self.target_sample_rate {
            return Ok(audio_data.clone());
        }

        // 設定重取樣參數
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: rubato::WindowFunction::BlackmanHarris2,
        };

        // 建立重取樣器
        let mut resampler = SincFixedIn::<f64>::new(
            self.target_sample_rate as f64 / audio_data.info.sample_rate as f64,
            2.0, // max_resample_ratio_relative
            params,
            audio_data.samples.len(),
            audio_data.info.channels as usize,
        )
        .map_err(|e| SubXError::audio_processing(format!("Failed to create resampler: {}", e)))?;

        // 轉換樣本格式為 f64
        let input_channels = if audio_data.info.channels == 1 {
            vec![
                audio_data
                    .samples
                    .iter()
                    .map(|&s| s as f64 / 32768.0)
                    .collect(),
            ]
        } else {
            // 處理多聲道音訊
            let mut channels = vec![Vec::new(); audio_data.info.channels as usize];
            for (i, &sample) in audio_data.samples.iter().enumerate() {
                channels[i % audio_data.info.channels as usize].push(sample as f64 / 32768.0);
            }
            channels
        };

        // 執行重取樣
        let output_channels = resampler
            .process(&input_channels, None)
            .map_err(|e| SubXError::audio_processing(format!("Resampling failed: {}", e)))?;

        // 轉換回 i16 格式
        let mut resampled_samples = Vec::new();
        if audio_data.info.channels == 1 {
            resampled_samples = output_channels[0]
                .iter()
                .map(|&s| (s * 32767.0) as i16)
                .collect();
        } else {
            // 交錯多聲道樣本
            let max_len = output_channels.iter().map(|ch| ch.len()).max().unwrap_or(0);
            for i in 0..max_len {
                for ch in &output_channels {
                    if i < ch.len() {
                        resampled_samples.push((ch[i] * 32767.0) as i16);
                    }
                }
            }
        }

        let duration_seconds = resampled_samples.len() as f64
            / (self.target_sample_rate as f64 * audio_data.info.channels as f64);

        Ok(ProcessedAudioData {
            samples: resampled_samples,
            info: AudioInfo {
                sample_rate: self.target_sample_rate,
                channels: audio_data.info.channels,
                duration_seconds,
                total_samples: resampled_samples.len(),
            },
        })
    }

    fn convert_to_mono(&self, audio_data: &ProcessedAudioData) -> Result<ProcessedAudioData> {
        if audio_data.info.channels == 1 {
            return Ok(audio_data.clone());
        }

        let channels = audio_data.info.channels as usize;
        let mut mono_samples = Vec::new();

        // 轉換為單聲道（平均所有聲道）
        for chunk in audio_data.samples.chunks_exact(channels) {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            let average = (sum / channels as i32) as i16;
            mono_samples.push(average);
        }

        let duration_seconds = mono_samples.len() as f64 / audio_data.info.sample_rate as f64;

        Ok(ProcessedAudioData {
            samples: mono_samples,
            info: AudioInfo {
                sample_rate: audio_data.info.sample_rate,
                channels: 1,
                duration_seconds,
                total_samples: mono_samples.len(),
            },
        })
    }
}
