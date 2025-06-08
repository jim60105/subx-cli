use super::segment::DialogueSegment;
use std::collections::VecDeque;

/// 音訊能量分析器，用於語音活動檢測
pub struct EnergyAnalyzer {
    window_size: usize,
    hop_size: usize,
    threshold: f32,
    min_duration_ms: u64,
}

impl EnergyAnalyzer {
    /// 建立分析器，設定能量閾值與最短語音持續時間
    pub fn new(threshold: f32, min_duration_ms: u64) -> Self {
        Self {
            window_size: 1024,
            hop_size: 512,
            threshold,
            min_duration_ms,
        }
    }

    /// 分析音訊樣本，回傳對話片段列表
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
