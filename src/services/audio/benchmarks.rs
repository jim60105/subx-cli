//! 音訊處理效能基準測試

use crate::services::audio::{AudioAnalyzer, AusAudioAnalyzer};
use crate::Result;
use std::time::Instant;

/// 效能基準測試工具
pub struct PerformanceBenchmark {
    legacy_analyzer: AudioAnalyzer,
    aus_analyzer: AusAudioAnalyzer,
}

impl PerformanceBenchmark {
    /// 建立基準測試工具
    pub fn new() -> Self {
        Self {
            legacy_analyzer: AudioAnalyzer::new(16000),
            aus_analyzer: AusAudioAnalyzer::new(16000),
        }
    }

    /// 比較兩種實作的能量包絡提取效能
    pub async fn benchmark_envelope_extraction(
        &mut self,
        audio_path: &std::path::Path,
    ) -> Result<BenchmarkResult> {
        let start = Instant::now();
        let legacy_envelope = self.legacy_analyzer.extract_envelope(audio_path).await?;
        let legacy_duration = start.elapsed();

        let start = Instant::now();
        let aus_envelope = self.aus_analyzer.extract_envelope_v2(audio_path).await?;
        let aus_duration = start.elapsed();

        Ok(BenchmarkResult {
            legacy_duration,
            aus_duration,
            speedup_ratio: legacy_duration.as_secs_f64() / aus_duration.as_secs_f64(),
            results_similar: self.compare_envelopes(&legacy_envelope, &aus_envelope),
        })
    }

    /// 簡單比較能量包絡樣本相似度
    fn compare_envelopes(
        &self,
        legacy: &crate::services::audio::AudioEnvelope,
        aus: &crate::services::audio::AudioEnvelope,
    ) -> bool {
        if (legacy.samples.len() as i32 - aus.samples.len() as i32).abs() > 5 {
            return false;
        }

        let min_len = legacy.samples.len().min(aus.samples.len());
        let mut diff_sum = 0.0;
        for i in 0..min_len {
            diff_sum += (legacy.samples[i] - aus.samples[i]).abs();
        }
        let avg_diff = diff_sum / min_len as f32;
        avg_diff < 0.1
    }
}

/// 基準測試結果
#[derive(Debug)]
pub struct BenchmarkResult {
    pub legacy_duration: std::time::Duration,
    pub aus_duration: std::time::Duration,
    pub speedup_ratio: f64,
    pub results_similar: bool,
}
