//! 音訊重採樣品質評估器
#![allow(dead_code, unused_imports)]

use crate::services::audio::AudioData;
use crate::Result;

/// 品質評估器
pub struct QualityAssessor {
    metrics: Vec<QualityMetric>,
}

impl QualityAssessor {
    pub fn new() -> Self {
        Self {
            metrics: vec![
                QualityMetric::SignalToNoiseRatio,
                QualityMetric::FrequencyResponse,
                QualityMetric::DynamicRange,
            ],
        }
    }

    /// 評估重採樣品質
    pub fn assess_quality(
        &self,
        original: &AudioData,
        resampled: &AudioData,
    ) -> Result<QualityReport> {
        let mut report = QualityReport::new();
        for metric in &self.metrics {
            let score = self.calculate_metric_score(metric, original, resampled)?;
            report.add_metric_score(metric.clone(), score);
        }
        report.calculate_overall_score();
        Ok(report)
    }

    fn calculate_metric_score(
        &self,
        metric: &QualityMetric,
        original: &AudioData,
        resampled: &AudioData,
    ) -> Result<f32> {
        match metric {
            QualityMetric::SignalToNoiseRatio => self.calculate_snr(original, resampled),
            QualityMetric::FrequencyResponse => {
                self.calculate_frequency_response(original, resampled)
            }
            QualityMetric::DynamicRange => self.calculate_dynamic_range(original, resampled),
        }
    }

    fn calculate_snr(&self, original: &AudioData, resampled: &AudioData) -> Result<f32> {
        // 計算信噪比 (SNR)，歸一化至 0.0-1.0 範圍
        let len = original.samples.len().min(resampled.samples.len());
        if len == 0 {
            return Ok(0.0);
        }
        let mut signal_power = 0.0f32;
        let mut noise_power = 0.0f32;
        for i in 0..len {
            let orig = original.samples[i];
            signal_power += orig * orig;
            let noise = orig - resampled.samples[i];
            noise_power += noise * noise;
        }
        if noise_power == 0.0 {
            return Ok(1.0);
        }
        let snr_db = 10.0 * (signal_power / noise_power).log10();
        // 假設最大 50dB
        let normalized = (snr_db / 50.0).max(0.0).min(1.0);
        Ok(normalized)
    }

    fn calculate_frequency_response(
        &self,
        _original: &AudioData,
        _resampled: &AudioData,
    ) -> Result<f32> {
        // 頻率響應計算暫不實作，預設最佳相似度
        Ok(1.0)
    }

    fn calculate_dynamic_range(&self, original: &AudioData, resampled: &AudioData) -> Result<f32> {
        // 動態範圍保持度 = 重採樣後動態範圍 / 原始動態範圍，範圍 0.0-1.0
        let orig_max = original.samples.iter().cloned().fold(f32::MIN, f32::max);
        let orig_min = original.samples.iter().cloned().fold(f32::MAX, f32::min);
        let res_max = resampled.samples.iter().cloned().fold(f32::MIN, f32::max);
        let res_min = resampled.samples.iter().cloned().fold(f32::MAX, f32::min);
        let orig_range = orig_max - orig_min;
        if orig_range <= 0.0 {
            return Ok(0.0);
        }
        let res_range = res_max - res_min;
        let ratio = (res_range / orig_range).max(0.0).min(1.0);
        Ok(ratio)
    }
}

/// 品質評估指標
#[derive(Debug, Clone)]
pub enum QualityMetric {
    SignalToNoiseRatio,
    FrequencyResponse,
    DynamicRange,
}

/// 品質評估報告
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub overall_score: f32,
    pub metric_scores: std::collections::HashMap<String, f32>,
    pub recommendations: Vec<String>,
}

impl QualityReport {
    pub fn new() -> Self {
        Self {
            overall_score: 0.0,
            metric_scores: std::collections::HashMap::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn add_metric_score(&mut self, metric: QualityMetric, score: f32) {
        let name = format!("{:?}", metric);
        self.metric_scores.insert(name, score);
    }

    pub fn calculate_overall_score(&mut self) {
        if self.metric_scores.is_empty() {
            self.overall_score = 0.0;
            return;
        }
        let total: f32 = self.metric_scores.values().sum();
        self.overall_score = total / self.metric_scores.len() as f32;
        self.generate_recommendations();
    }

    fn generate_recommendations(&mut self) {
        self.recommendations.clear();
        if self.overall_score < 0.7 {
            self.recommendations
                .push("建議提高重採樣品質設定".to_string());
        }
        if let Some(&snr) = self.metric_scores.get("SignalToNoiseRatio") {
            if snr < 0.6 {
                self.recommendations
                    .push("檢測到較高的噪音，建議使用較高品質的重採樣演算法".to_string());
            }
        }
        if let Some(&freq) = self.metric_scores.get("FrequencyResponse") {
            if freq < 0.8 {
                self.recommendations
                    .push("頻率響應失真較大，建議使用 Sinc 插值器".to_string());
            }
        }
    }
}
