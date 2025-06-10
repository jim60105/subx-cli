//! Dialogue detector based on the aus crate.

use crate::Result;
use crate::services::audio::DialogueSegment;
use aus::{AudioFile, WindowType, analysis, spectrum};
use std::collections::VecDeque;

/// Dialogue detector based on aus analysis.
pub struct AusDialogueDetector {
    energy_threshold: f32,
    spectral_threshold: f32,
    min_duration_ms: u32,
    window_size: usize,
    hop_size: usize,
}

impl AusDialogueDetector {
    /// Create a dialogue detector and set the energy threshold.
    pub fn new(threshold: f32) -> Self {
        Self {
            energy_threshold: threshold,
            spectral_threshold: 1500.0,
            min_duration_ms: 500,
            window_size: 1024,
            hop_size: 512,
        }
    }

    /// Multi-feature dialogue detection.
    pub fn detect_dialogue(&self, audio_file: &AudioFile) -> Result<Vec<DialogueSegment>> {
        let samples = &audio_file.samples[0];
        let sample_rate = audio_file.sample_rate;
        let stft_result = spectrum::rstft(
            samples,
            self.window_size,
            self.hop_size,
            WindowType::Hanning,
        );

        let mut segments = Vec::new();
        let mut dialogue_buffer = VecDeque::new();
        let mut current_start: Option<f32> = None;
        let frame_duration = self.hop_size as f32 / sample_rate as f32;

        for (frame_idx, frame) in stft_result.iter().enumerate() {
            let time_stamp = frame_idx as f32 * frame_duration;
            let (magnitude_spectrum, _) = spectrum::complex_to_polar_rfft(frame);
            let frequencies = spectrum::rfftfreq(self.window_size, sample_rate);
            let frame_energy = analysis::energy(&magnitude_spectrum);
            let spectral_centroid = analysis::spectral_centroid(&magnitude_spectrum, &frequencies);
            let spectral_entropy = analysis::spectral_entropy(&magnitude_spectrum);

            let is_speech = self.is_speech_frame(
                frame_energy as f32,
                spectral_centroid as f32,
                spectral_entropy as f32,
            );

            dialogue_buffer.push_back(is_speech);
            if dialogue_buffer.len() > 10 {
                dialogue_buffer.pop_front();
            }

            let speech_ratio = dialogue_buffer.iter().filter(|&&x| x).count() as f32
                / dialogue_buffer.len() as f32;

            if speech_ratio > 0.6 && current_start.is_none() {
                current_start = Some(time_stamp);
            }

            if speech_ratio < 0.3 && current_start.is_some() {
                let start = current_start.unwrap();
                let duration_ms = (time_stamp - start) * 1000.0;

                if duration_ms >= self.min_duration_ms as f32 {
                    segments.push(DialogueSegment {
                        start_time: start,
                        end_time: time_stamp,
                        intensity: speech_ratio,
                    });
                }
                current_start = None;
            }
        }

        if let Some(start) = current_start {
            let duration = samples.len() as f32 / sample_rate as f32;
            segments.push(DialogueSegment {
                start_time: start,
                end_time: duration,
                intensity: 0.8,
            });
        }

        Ok(segments)
    }

    /// Multi-feature speech detection.
    fn is_speech_frame(&self, energy: f32, spectral_centroid: f32, spectral_entropy: f32) -> bool {
        let energy_check = energy > self.energy_threshold;
        let spectral_check =
            spectral_centroid > 300.0 && spectral_centroid < self.spectral_threshold;
        let entropy_check = spectral_entropy > 0.5;

        [energy_check, spectral_check, entropy_check]
            .iter()
            .filter(|&&x| x)
            .count()
            >= 2
    }
}
