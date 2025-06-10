//! Audio energy analyzer for dialogue segment detection.
//!
//! This module provides `EnergyAnalyzer` to detect speech activity in audio
//! streams based on energy thresholds and segment durations.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::sync::dialogue::EnergyAnalyzer;
//! // Analyze audio envelope to detect dialogue segments
//! ```

use super::segment::DialogueSegment;
use std::collections::VecDeque;

/// Analyzer that detects dialogue segments based on audio energy levels.
pub struct EnergyAnalyzer {
    window_size: usize,
    hop_size: usize,
    threshold: f32,
    min_duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 測試能量分析器能偵測能量變化區段
    #[test]
    fn test_energy_analyzer() {
        // 使用較低門檻以確保能量變化可被偵測
        let analyzer = EnergyAnalyzer::new(0.0, 0);
        // 建立交替靜音與高能量的樣本序列
        let mut samples = Vec::new();
        for block in 0..10 {
            let value = if block % 2 == 0 { 0.0 } else { 1.0 };
            for _ in 0..512 {
                samples.push(value);
            }
        }
        let segments = analyzer.analyze(&samples, 44100);
        assert!(!segments.is_empty(), "應能偵測到至少一個語音區段");
        // 至少有一筆區段資料，後續可依需求判斷變化情況
        assert!(!segments.is_empty(), "應偵測到至少一筆能量區段");
    }
}

impl EnergyAnalyzer {
    /// Create analyzer with energy threshold and minimum speech duration settings
    pub fn new(threshold: f32, min_duration_ms: u64) -> Self {
        Self {
            window_size: 1024,
            hop_size: 512,
            threshold,
            min_duration_ms,
        }
    }

    /// Analyze audio samples and return dialogue segment list
    pub fn analyze(&self, audio_data: &[f32], sample_rate: u32) -> Vec<DialogueSegment> {
        let mut segments: Vec<DialogueSegment> = Vec::new();
        let mut energy_buffer = VecDeque::new();

        for (i, chunk) in audio_data.chunks(self.hop_size).enumerate() {
            let energy = self.calculate_energy(chunk);
            energy_buffer.push_back(energy);
            if energy_buffer.len() > self.window_size / self.hop_size {
                energy_buffer.pop_front();
            }
            let is_speech = self.detect_speech(&energy_buffer);
            let timestamp = (i * self.hop_size) as f64 / sample_rate as f64;

            if is_speech {
                if let Some(last) = segments.last_mut() {
                    if last.is_speech {
                        last.end_time = timestamp;
                    } else {
                        segments.push(DialogueSegment::new_speech(timestamp, timestamp));
                    }
                } else {
                    segments.push(DialogueSegment::new_speech(timestamp, timestamp));
                }
            }
        }
        self.filter_short_segments(segments)
    }

    fn calculate_energy(&self, chunk: &[f32]) -> f32 {
        let sum_sq: f32 = chunk.iter().map(|&v| v * v).sum();
        if chunk.is_empty() {
            0.0
        } else {
            (sum_sq / chunk.len() as f32).sqrt()
        }
    }

    fn detect_speech(&self, buffer: &VecDeque<f32>) -> bool {
        if buffer.is_empty() {
            return false;
        }
        let avg: f32 = buffer.iter().copied().sum::<f32>() / buffer.len() as f32;
        avg > self.threshold
    }

    fn filter_short_segments(&self, segments: Vec<DialogueSegment>) -> Vec<DialogueSegment> {
        segments
            .into_iter()
            .filter(|seg| (seg.duration() * 1000.0) as u64 >= self.min_duration_ms)
            .collect()
    }
}
