//! Audio analyzer based on the aus crate.

use crate::services::audio::{AudioData, AudioEnvelope};
use crate::{Result, error::SubXError};
use aus::{AudioFile, WindowType, analysis, operations, spectrum};
use std::path::Path;

/// Audio analyzer based on aus.
pub struct AusAudioAnalyzer {
    sample_rate: u32,
    window_size: usize,
    hop_size: usize,
}

impl AusAudioAnalyzer {
    /// Create a new analyzer and set the sample rate
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            window_size: 1024,
            hop_size: 512,
        }
    }

    /// Load audio file using aus
    pub async fn load_audio_file<P: AsRef<Path>>(&self, audio_path: P) -> Result<AudioFile> {
        let path = audio_path.as_ref();
        let path_str = path
            .to_str()
            .ok_or_else(|| SubXError::audio_processing("Failed to convert path to UTF-8 string"))?;
        let mut audio_file = aus::read(path_str)?;
        if audio_file.num_channels > 1 {
            aus::mixdown(&mut audio_file);
        }
        Ok(audio_file)
    }

    /// Load audio file and convert to AudioData format
    pub async fn load_audio_data<P: AsRef<Path>>(&self, audio_path: P) -> Result<AudioData> {
        let audio_file = self.load_audio_file(audio_path).await?;
        let samples: Vec<f32> = audio_file.samples[0].iter().map(|&x| x as f32).collect();
        Ok(AudioData {
            samples,
            sample_rate: audio_file.sample_rate,
            channels: audio_file.num_channels,
            duration: audio_file.duration as f32,
        })
    }

    /// Extract audio energy envelope
    pub async fn extract_envelope<P: AsRef<Path>>(&self, audio_path: P) -> Result<AudioEnvelope> {
        let audio_file = self.load_audio_file(audio_path).await?;
        let samples = &audio_file.samples[0];
        let mut energy_samples = Vec::new();
        for chunk in samples.chunks(self.hop_size) {
            let rms_energy = operations::rms(chunk);
            energy_samples.push(rms_energy as f32);
        }
        let duration = audio_file.duration as f32;
        Ok(AudioEnvelope {
            samples: energy_samples,
            sample_rate: self.sample_rate,
            duration,
        })
    }

    /// Detect dialogue segments (legacy interface compatible)
    pub fn detect_dialogue(
        &self,
        envelope: &AudioEnvelope,
        threshold: f32,
    ) -> Vec<crate::services::audio::DialogueSegment> {
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
                    segments.push(crate::services::audio::DialogueSegment {
                        start_time: start,
                        end_time: t,
                        intensity: e,
                    });
                }
            }
        }

        segments
    }

    /// Audio feature analysis using aus
    pub async fn analyze_audio_features(&self, audio_file: &AudioFile) -> Result<AudioFeatures> {
        let samples = &audio_file.samples[0];
        let stft_result = spectrum::rstft(
            samples,
            self.window_size,
            self.hop_size,
            WindowType::Hanning,
        );

        let mut features = Vec::new();
        for frame in stft_result.iter() {
            let (magnitude_spectrum, _) = spectrum::complex_to_polar_rfft(frame);
            let frequencies = spectrum::rfftfreq(self.window_size, audio_file.sample_rate);

            let spectral_centroid = analysis::spectral_centroid(&magnitude_spectrum, &frequencies);
            let spectral_entropy = analysis::spectral_entropy(&magnitude_spectrum);
            let zero_crossing_rate = analysis::zero_crossing_rate(samples, audio_file.sample_rate);

            features.push(FrameFeatures {
                spectral_centroid: spectral_centroid as f32,
                spectral_entropy: spectral_entropy as f32,
                zero_crossing_rate: zero_crossing_rate as f32,
            });
        }

        Ok(AudioFeatures { frames: features })
    }
}

/// Audio feature data structure containing extracted characteristics.
///
/// Contains frame-by-frame audio features extracted from audio analysis,
/// used for dialogue detection and subtitle synchronization.
#[derive(Debug, Clone)]
pub struct AudioFeatures {
    /// Vector of feature data for each audio frame
    pub frames: Vec<FrameFeatures>,
}

/// Feature data for a single audio frame.
///
/// Contains various audio characteristics computed for a short
/// time window of audio data.
#[derive(Debug, Clone)]
pub struct FrameFeatures {
    /// Spectral centroid indicating the "brightness" of the sound
    pub spectral_centroid: f32,
    /// Spectral entropy measuring randomness in the frequency domain
    pub spectral_entropy: f32,
    /// Zero crossing rate indicating the noisiness of the signal
    pub zero_crossing_rate: f32,
}
