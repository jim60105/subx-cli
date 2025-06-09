//! 簡化的重採樣器

use crate::Result;
use crate::services::audio::AudioData;

/// 簡化的線性重採樣器
pub struct SimplifiedResampler {
    target_rate: u32,
}

impl SimplifiedResampler {
    /// 建立新的重採樣器
    pub fn new(target_rate: u32) -> Self {
        Self { target_rate }
    }

    /// 線性插值重採樣
    pub fn resample(&self, input: &AudioData) -> Result<AudioData> {
        if input.sample_rate == self.target_rate {
            return Ok(input.clone());
        }

        let ratio = self.target_rate as f64 / input.sample_rate as f64;
        let output_length = (input.samples.len() as f64 * ratio) as usize;
        let mut output = Vec::with_capacity(output_length);

        for i in 0..output_length {
            let src_index = i as f64 / ratio;
            let index = src_index as usize;

            if index + 1 < input.samples.len() {
                let fraction = src_index - index as f64;
                let sample = input.samples[index] * (1.0 - fraction as f32)
                    + input.samples[index + 1] * fraction as f32;
                output.push(sample);
            } else if index < input.samples.len() {
                output.push(input.samples[index]);
            }
        }

        Ok(AudioData {
            samples: output,
            sample_rate: self.target_rate,
            channels: input.channels,
            duration: input.duration,
        })
    }
}
