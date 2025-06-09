use crate::Result;
use crate::core::formats::Subtitle;
use crate::services::audio::{AudioAnalyzer, AudioEnvelope};
use std::path::Path;

/// 同步引擎
pub struct SyncEngine {
    audio_analyzer: AudioAnalyzer,
    config: SyncConfig,
}

/// 同步配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    pub max_offset_seconds: f32,
    pub correlation_threshold: f32,
    pub dialogue_threshold: f32,
    pub min_dialogue_length: f32,
}

/// 同步結果
#[derive(Debug)]
pub struct SyncResult {
    pub offset_seconds: f32,
    pub confidence: f32,
    pub method_used: SyncMethod,
    pub correlation_peak: f32,
}

/// 同步方法
#[derive(Debug)]
pub enum SyncMethod {
    AudioCorrelation,
    ManualOffset,
    PatternMatching,
}

impl SyncEngine {
    /// 建立同步引擎
    pub fn new(config: SyncConfig) -> Self {
        Self {
            audio_analyzer: AudioAnalyzer::new(16000),
            config,
        }
    }

    /// 自動同步字幕
    pub async fn sync_subtitle(
        &self,
        video_path: &Path,
        subtitle: &Subtitle,
    ) -> Result<SyncResult> {
        let audio_envelope = self.audio_analyzer.extract_envelope(video_path).await?;
        let _dialogue_segments = self
            .audio_analyzer
            .detect_dialogue(&audio_envelope, self.config.dialogue_threshold);

        let subtitle_signal = self.generate_subtitle_signal(
            subtitle,
            audio_envelope.duration,
            audio_envelope.sample_rate,
        );
        let correlation_result =
            self.calculate_cross_correlation(&audio_envelope, &subtitle_signal)?;

        Ok(correlation_result)
    }

    fn generate_subtitle_signal(
        &self,
        subtitle: &Subtitle,
        total_duration: f32,
        sample_rate: u32,
    ) -> Vec<f32> {
        let sample_rate = sample_rate as f32;
        let signal_length = (total_duration * sample_rate) as usize;
        let mut signal = vec![0.0; signal_length];

        for entry in &subtitle.entries {
            let start = (entry.start_time.as_secs_f32() * sample_rate) as usize;
            let end = (entry.end_time.as_secs_f32() * sample_rate) as usize;
            let range_end = end.min(signal_length);
            signal[start..range_end].iter_mut().for_each(|v| *v = 1.0);
        }

        signal
    }

    fn calculate_cross_correlation(
        &self,
        audio_envelope: &AudioEnvelope,
        subtitle_signal: &[f32],
    ) -> Result<SyncResult> {
        let max_offset_samples =
            (self.config.max_offset_seconds * audio_envelope.sample_rate as f32) as i32;
        let mut best_offset = 0;
        let mut best_correlation = 0.0;

        for offset in -max_offset_samples..=max_offset_samples {
            let corr = self.calculate_correlation_at_offset(
                &audio_envelope.samples,
                subtitle_signal,
                offset,
            );
            if corr > best_correlation {
                best_correlation = corr;
                best_offset = offset;
            }
        }

        let offset_seconds = best_offset as f32 / audio_envelope.sample_rate as f32;
        let confidence = if best_correlation > self.config.correlation_threshold {
            best_correlation
        } else {
            0.0
        };

        Ok(SyncResult {
            offset_seconds,
            confidence,
            method_used: SyncMethod::AudioCorrelation,
            correlation_peak: best_correlation,
        })
    }

    fn calculate_correlation_at_offset(
        &self,
        audio_signal: &[f32],
        subtitle_signal: &[f32],
        offset: i32,
    ) -> f32 {
        let audio_len = audio_signal.len() as i32;
        let subtitle_len = subtitle_signal.len() as i32;
        let mut sum_product = 0.0;
        let mut sum_audio_sq = 0.0;
        let mut sum_sub_sq = 0.0;
        let mut count = 0;

        for i in 0..audio_len {
            let j = i + offset;
            if j >= 0 && j < subtitle_len {
                let a = audio_signal[i as usize];
                let s = subtitle_signal[j as usize];
                sum_product += a * s;
                sum_audio_sq += a * a;
                sum_sub_sq += s * s;
                count += 1;
            }
        }

        if count == 0 || sum_audio_sq == 0.0 || sum_sub_sq == 0.0 {
            return 0.0;
        }

        sum_product / (sum_audio_sq.sqrt() * sum_sub_sq.sqrt())
    }

    /// 套用同步偏移到字幕
    pub fn apply_sync_offset(&self, subtitle: &mut Subtitle, offset_seconds: f32) -> Result<()> {
        let offset_dur = std::time::Duration::from_secs_f32(offset_seconds.abs());
        for entry in &mut subtitle.entries {
            if offset_seconds >= 0.0 {
                entry.start_time += offset_dur;
                entry.end_time += offset_dur;
            } else if entry.start_time > offset_dur {
                entry.start_time -= offset_dur;
                entry.end_time -= offset_dur;
            } else {
                let rem = offset_dur - entry.start_time;
                entry.start_time = std::time::Duration::ZERO;
                if entry.end_time > rem {
                    entry.end_time -= rem;
                } else {
                    entry.end_time = std::time::Duration::ZERO;
                }
            }
        }
        Ok(())
    }
}
