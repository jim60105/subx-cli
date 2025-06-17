use super::AudioInfo;
use crate::services::vad::audio_loader::DirectAudioLoader;
use crate::{Result, error::SubXError};
use hound::{SampleFormat, WavReader};
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Audio processor for VAD operations.
///
/// Handles loading, resampling, and format conversion of audio files
/// for voice activity detection processing.
pub struct VadAudioProcessor {
    target_sample_rate: u32,
    target_channels: u16,
}

/// Processed audio data ready for VAD analysis.
///
/// Contains the audio samples and metadata after processing
/// and format conversion.
#[derive(Debug)]
pub struct ProcessedAudioData {
    /// Audio samples as 16-bit integers
    pub samples: Vec<i16>,
    /// Audio metadata and properties
    pub info: AudioInfo,
}

impl VadAudioProcessor {
    /// Create a new VAD audio processor.
    ///
    /// # Arguments
    ///
    /// * `target_sample_rate` - Desired sample rate for processing
    /// * `target_channels` - Desired number of audio channels
    ///
    /// # Returns
    ///
    /// A new `VadAudioProcessor` instance
    pub fn new(target_sample_rate: u32, target_channels: u16) -> Result<Self> {
        Ok(Self {
            target_sample_rate,
            target_channels,
        })
    }

    /// Load and prepare audio file for VAD processing.
    ///
    /// Performs all necessary audio processing steps including loading,
    /// resampling, and format conversion to prepare the audio for
    /// voice activity detection.
    ///
    /// # Arguments
    ///
    /// * `audio_path` - Path to the audio file to process
    ///
    /// # Returns
    ///
    /// Processed audio data ready for VAD analysis
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Audio file cannot be loaded
    /// - Audio format is unsupported
    /// - Resampling fails
    /// - Format conversion fails
    ///   直接載入並準備音訊檔案，用於 VAD 處理，支援多種格式。
    pub async fn load_and_prepare_audio_direct(
        &self,
        audio_path: &Path,
    ) -> Result<ProcessedAudioData> {
        // 1. 使用 DirectAudioLoader 載入樣本與音訊資訊
        let loader = DirectAudioLoader::new()?;
        let (samples, info) = loader.load_audio_samples(audio_path)?;
        let audio_data = ProcessedAudioData { samples, info };

        // 2. 重採樣（如需）
        let resampled = if audio_data.info.sample_rate != self.target_sample_rate {
            self.resample_audio(&audio_data)?
        } else {
            audio_data
        };

        // 3. 轉換為單聲道（如需）
        let mono = if resampled.info.channels > 1 {
            self.convert_to_mono(&resampled)?
        } else {
            resampled
        };

        Ok(mono)
    }

    fn load_wav_file(&self, path: &Path) -> Result<ProcessedAudioData> {
        let file = File::open(path).map_err(|e| {
            SubXError::audio_processing(format!("Failed to open audio file: {}", e))
        })?;

        let reader = WavReader::new(BufReader::new(file))
            .map_err(|e| SubXError::audio_processing(format!("Failed to read WAV file: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels;

        // Read all samples and convert to i16
        let samples: Vec<i16> = match spec.sample_format {
            SampleFormat::Int => match spec.bits_per_sample {
                16 => {
                    let samples: std::result::Result<Vec<i16>, hound::Error> =
                        reader.into_samples::<i16>().collect();
                    samples.map_err(|e| {
                        SubXError::audio_processing(format!("Failed to read samples: {}", e))
                    })?
                }
                32 => {
                    let samples: std::result::Result<Vec<i32>, hound::Error> =
                        reader.into_samples::<i32>().collect();
                    let i32_samples = samples.map_err(|e| {
                        SubXError::audio_processing(format!("Failed to read i32 samples: {}", e))
                    })?;
                    i32_samples.iter().map(|&s| (s >> 16) as i16).collect()
                }
                _ => {
                    return Err(SubXError::audio_processing(format!(
                        "Unsupported bit depth: {}",
                        spec.bits_per_sample
                    )));
                }
            },
            SampleFormat::Float => {
                let samples: std::result::Result<Vec<f32>, hound::Error> =
                    reader.into_samples::<f32>().collect();
                let f32_samples = samples.map_err(|e| {
                    SubXError::audio_processing(format!("Failed to read f32 samples: {}", e))
                })?;
                f32_samples.iter().map(|&s| (s * 32767.0) as i16).collect()
            }
        };

        let samples_len = samples.len();
        let duration_seconds = samples_len as f64 / (sample_rate as f64 * channels as f64);

        Ok(ProcessedAudioData {
            samples,
            info: AudioInfo {
                sample_rate,
                channels,
                duration_seconds,
                total_samples: samples_len,
            },
        })
    }

    fn resample_audio(&self, audio_data: &ProcessedAudioData) -> Result<ProcessedAudioData> {
        if audio_data.info.sample_rate == self.target_sample_rate {
            // Cloning via struct initializer to own data
            return Ok(ProcessedAudioData {
                samples: audio_data.samples.clone(),
                info: audio_data.info.clone(),
            });
        }

        // Configure resampling parameters
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 128,
            window: rubato::WindowFunction::BlackmanHarris2,
        };

        // Create resampler
        let mut resampler = SincFixedIn::<f64>::new(
            self.target_sample_rate as f64 / audio_data.info.sample_rate as f64,
            2.0, // max_resample_ratio_relative
            params,
            audio_data.samples.len(),
            audio_data.info.channels as usize,
        )
        .map_err(|e| SubXError::audio_processing(format!("Failed to create resampler: {}", e)))?;

        // Convert sample format to f64
        let input_channels = if audio_data.info.channels == 1 {
            vec![
                audio_data
                    .samples
                    .iter()
                    .map(|&s| s as f64 / 32768.0)
                    .collect(),
            ]
        } else {
            // Process multi-channel audio
            let mut channels = vec![Vec::new(); audio_data.info.channels as usize];
            for (i, &sample) in audio_data.samples.iter().enumerate() {
                channels[i % audio_data.info.channels as usize].push(sample as f64 / 32768.0);
            }
            channels
        };

        // Perform resampling
        let output_channels = resampler
            .process(&input_channels, None)
            .map_err(|e| SubXError::audio_processing(format!("Resampling failed: {}", e)))?;

        // Convert back to i16 format
        let mut resampled_samples = Vec::new();
        if audio_data.info.channels == 1 {
            resampled_samples = output_channels[0]
                .iter()
                .map(|&s| (s * 32767.0) as i16)
                .collect();
        } else {
            // Interleave multi-channel samples
            let max_len = output_channels.iter().map(|ch| ch.len()).max().unwrap_or(0);
            for i in 0..max_len {
                for ch in &output_channels {
                    if i < ch.len() {
                        resampled_samples.push((ch[i] * 32767.0) as i16);
                    }
                }
            }
        }

        let samples_len = resampled_samples.len();
        let duration_seconds =
            samples_len as f64 / (self.target_sample_rate as f64 * audio_data.info.channels as f64);

        Ok(ProcessedAudioData {
            samples: resampled_samples,
            info: AudioInfo {
                sample_rate: self.target_sample_rate,
                channels: audio_data.info.channels,
                duration_seconds,
                total_samples: samples_len,
            },
        })
    }

    fn convert_to_mono(&self, audio_data: &ProcessedAudioData) -> Result<ProcessedAudioData> {
        if audio_data.info.channels == 1 {
            return Ok(ProcessedAudioData {
                samples: audio_data.samples.clone(),
                info: audio_data.info.clone(),
            });
        }

        let channels = audio_data.info.channels as usize;
        let mut mono_samples = Vec::new();

        // Convert to mono (average all channels)
        for chunk in audio_data.samples.chunks_exact(channels) {
            let sum: i32 = chunk.iter().map(|&s| s as i32).sum();
            let average = (sum / channels as i32) as i16;
            mono_samples.push(average);
        }

        let samples_len = mono_samples.len();
        let duration_seconds = samples_len as f64 / audio_data.info.sample_rate as f64;

        Ok(ProcessedAudioData {
            samples: mono_samples,
            info: AudioInfo {
                sample_rate: audio_data.info.sample_rate,
                channels: 1,
                duration_seconds,
                total_samples: samples_len,
            },
        })
    }
}
