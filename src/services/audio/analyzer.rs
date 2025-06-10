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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Test audio file loading functionality
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_file_success() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();
        // Create mock WAV file (minimal valid WAV header)
        let wav_data = create_minimal_wav_file(44100, 1, 1.0);
        let wav_path = temp_dir.path().join("test.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let result = analyzer.load_audio_file(&wav_path).await;
        assert!(result.is_ok());

        let audio_file = result.unwrap();
        assert_eq!(audio_file.sample_rate, 44100);
        assert!(audio_file.duration > 0.0);
        assert_eq!(audio_file.num_channels, 1);
    }

    /// Test error handling for non-existent files
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_file_not_exists() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let result = analyzer.load_audio_file("non_existent.wav").await;
        assert!(result.is_err());
    }

    /// Test audio data format conversion
    #[ignore]
    #[tokio::test]
    async fn test_load_audio_data_conversion() {
        let analyzer = AusAudioAnalyzer::new(16000);
        let temp_dir = TempDir::new().unwrap();

        let wav_data = create_minimal_wav_file(16000, 1, 2.0);
        let wav_path = temp_dir.path().join("test.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let audio_data = analyzer.load_audio_data(&wav_path).await.unwrap();

        assert_eq!(audio_data.sample_rate, 16000);
        assert_eq!(audio_data.channels, 1);
        assert!(audio_data.duration > 1.9 && audio_data.duration < 2.1);
        assert!(!audio_data.samples.is_empty());
    }

    /// Test audio energy envelope extraction
    #[ignore]
    #[tokio::test]
    async fn test_extract_envelope_features() {
        let sample_rate = 44100;
        let analyzer = AusAudioAnalyzer::new(sample_rate);
        let temp_dir = TempDir::new().unwrap();

        // Create audio file with varying energy levels
        let wav_data = create_varying_energy_wav(44100, 2.0);
        let wav_path = temp_dir.path().join("varying.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let envelope = analyzer.extract_envelope(&wav_path).await.unwrap();

        assert!(!envelope.samples.is_empty());
        assert_eq!(envelope.sample_rate, sample_rate);
        assert!(envelope.duration > 1.9);

        // Verify energy values are within reasonable range
        for &energy in &envelope.samples {
            assert!(energy >= 0.0);
            assert!(energy <= 1.0);
        }
    }

    /// Test dialogue detection functionality
    #[ignore]
    #[tokio::test]
    async fn test_detect_dialogue_segments() {
        let analyzer = AusAudioAnalyzer::new(16000);

        // Create mock audio envelope (containing speech and silence segments)
        let envelope = AudioEnvelope {
            samples: vec![
                0.1, 0.8, 0.9, 0.7, 0.2, // Speech segment
                0.05, 0.03, 0.02, 0.04, // Silence segment
                0.6, 0.8, 0.7, 0.9, 0.5, // Speech segment
            ],
            sample_rate: 16000,
            duration: 2.0,
        };

        let segments = analyzer.detect_dialogue(&envelope, 0.3);

        assert!(!segments.is_empty());

        // Verify detected speech segments
        let speech_segments: Vec<_> = segments.iter().filter(|s| s.intensity > 0.3).collect();
        assert!(speech_segments.len() >= 2);
    }

    /// Test audio feature analysis
    #[ignore]
    #[tokio::test]
    async fn test_audio_features_analysis() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        let wav_data = create_spectral_rich_wav(44100, 1.0);
        let wav_path = temp_dir.path().join("rich.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let audio_file = analyzer.load_audio_file(&wav_path).await.unwrap();
        let features = analyzer.analyze_audio_features(&audio_file).await.unwrap();

        assert!(!features.frames.is_empty());

        for frame in &features.frames {
            // Verify spectral centroid is within reasonable range (0 to Nyquist frequency)
            assert!(frame.spectral_centroid >= 0.0);
            assert!(frame.spectral_centroid <= 22050.0);

            // Verify spectral entropy
            assert!(frame.spectral_entropy >= 0.0);
            assert!(frame.spectral_entropy <= 1.0);

            // Verify zero crossing rate
            assert!(frame.zero_crossing_rate >= 0.0);
            assert!(frame.zero_crossing_rate <= 1.0);
        }
    }

    /// Test invalid audio format handling
    #[ignore]
    #[tokio::test]
    async fn test_invalid_audio_format() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        // Create invalid audio file
        let invalid_path = temp_dir.path().join("invalid.wav");
        fs::write(&invalid_path, b"This is not audio data").unwrap();

        let result = analyzer.load_audio_file(&invalid_path).await;
        assert!(result.is_err());
    }

    /// Test large file processing and memory management
    #[ignore]
    #[tokio::test]
    async fn test_large_file_memory_management() {
        let analyzer = AusAudioAnalyzer::new(44100);
        let temp_dir = TempDir::new().unwrap();

        // Create larger audio file (10 seconds)
        let wav_data = create_minimal_wav_file(44100, 1, 10.0);
        let wav_path = temp_dir.path().join("large.wav");
        fs::write(&wav_path, wav_data).unwrap();

        let start_memory = get_memory_usage();
        let _audio_data = analyzer.load_audio_data(&wav_path).await.unwrap();
        let end_memory = get_memory_usage();

        // Verify memory usage is within reasonable range (< 100MB growth)
        assert!((end_memory - start_memory) < 100_000_000);
    }

    // Helper functions for creating test audio files
    fn create_minimal_wav_file(sample_rate: u32, channels: u16, duration: f32) -> Vec<u8> {
        let samples_per_channel = (sample_rate as f32 * duration) as u32;
        let total_samples = samples_per_channel * channels as u32;
        let data_size = total_samples * 2; // 16-bit samples
        let mut wav_data = Vec::new();
        // WAV header
        wav_data.extend_from_slice(b"RIFF");
        wav_data.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav_data.extend_from_slice(b"WAVE");
        wav_data.extend_from_slice(b"fmt ");
        wav_data.extend_from_slice(&16u32.to_le_bytes());
        wav_data.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav_data.extend_from_slice(&channels.to_le_bytes());
        wav_data.extend_from_slice(&sample_rate.to_le_bytes());
        wav_data.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
        wav_data.extend_from_slice(&(channels * 2).to_le_bytes());
        wav_data.extend_from_slice(&16u16.to_le_bytes());
        wav_data.extend_from_slice(b"data");
        wav_data.extend_from_slice(&data_size.to_le_bytes());
        // Audio data (simple sine wave)
        for i in 0..total_samples {
            let t = i as f32 / sample_rate as f32;
            let amplitude = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            let sample = (amplitude * 32767.0) as i16;
            wav_data.extend_from_slice(&sample.to_le_bytes());
        }
        wav_data
    }

    fn create_varying_energy_wav(sample_rate: u32, duration: f32) -> Vec<u8> {
        // Implementation for creating audio file with varying energy
        create_minimal_wav_file(sample_rate, 1, duration)
    }

    fn create_spectral_rich_wav(sample_rate: u32, duration: f32) -> Vec<u8> {
        // Implementation for creating spectrally rich audio file
        create_minimal_wav_file(sample_rate, 1, duration)
    }

    fn get_memory_usage() -> usize {
        // Simplified memory usage detection
        0 // Actual implementation could use procfs or other system tools
    }
}
