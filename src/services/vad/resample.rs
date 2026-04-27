//! Audio resampling utilities using the rubato crate.
//!
//! Provides i16 ↔ f32 conversion and synchronous resampling of mono PCM
//! audio via [`rubato::Fft`] with the [`rubato::FixedSync::Input`]
//! configuration. The rubato 2.0 API uses `audioadapter`-based buffers,
//! so this module wraps the input slice with `SequentialSlice` and the
//! output buffer with `SequentialSliceOfVecs` before delegating to
//! `Resampler::process_all_into_buffer`.

use audioadapter_buffers::direct::{SequentialSlice, SequentialSliceOfVecs};
use log::{debug, trace};
use rubato::{Fft, FixedSync, Resampler};
use std::error::Error;
use std::time::Instant;

/// Number of input frames per processing chunk handed to the resampler.
///
/// Tuned to balance throughput and intermediate buffer footprint; the
/// rubato resampler handles arbitrary input lengths through
/// `process_all_into_buffer`, so this value only influences the FFT
/// sub-chunking — the caller-visible behavior is unaffected.
const CHUNK_SIZE: usize = 8192;

/// Resample i16 mono audio to the target sample rate (returns `Vec<i16>`).
pub fn resample_to_target_rate(
    input_samples: &[i16],
    input_sample_rate: u32,
    output_sample_rate: u32,
) -> Result<Vec<i16>, Box<dyn Error>> {
    let total_start = Instant::now();
    debug!(
        "[resample] input_samples: {} input_sample_rate: {} output_sample_rate: {}",
        input_samples.len(),
        input_sample_rate,
        output_sample_rate
    );

    if input_sample_rate == output_sample_rate {
        debug!("[resample] sample rate unchanged, fast path");
        return Ok(input_samples.to_vec());
    }

    if input_samples.is_empty() {
        debug!("[resample] empty input, returning empty output");
        return Ok(Vec::new());
    }

    let t_convert = Instant::now();
    let input_f32: Vec<f32> = input_samples.iter().map(|&s| s as f32 / 32768.0).collect();
    debug!(
        "[resample] i16->f32 conversion done in {:.3?}",
        t_convert.elapsed()
    );

    let input_len = input_f32.len();
    let channels = 1;

    // Construct a fresh resampler per call. The rubato FFT plan caches
    // its trigonometric tables internally, so re-creation cost is
    // dominated by a few small allocations rather than transform setup.
    let t_init = Instant::now();
    let mut resampler = Fft::<f32>::new(
        input_sample_rate as usize,
        output_sample_rate as usize,
        CHUNK_SIZE,
        1,
        channels,
        FixedSync::Input,
    )?;
    debug!("[resample] Fft ready in {:.3?}", t_init.elapsed());

    // Pre-size the output buffer using rubato's own length oracle so
    // `process_all_into_buffer` never has to truncate.
    let needed_out = resampler.process_all_needed_output_len(input_len);
    let mut output_f32: Vec<Vec<f32>> = vec![vec![0.0f32; needed_out]];

    let in_adapter = SequentialSlice::new(&input_f32, channels, input_len)
        .map_err(|e| format!("input adapter construction failed: {e}"))?;
    let mut out_adapter = SequentialSliceOfVecs::new_mut(&mut output_f32, channels, needed_out)
        .map_err(|e| format!("output adapter construction failed: {e}"))?;

    let t_resample = Instant::now();
    let (in_used, out_produced) =
        resampler.process_all_into_buffer(&in_adapter, &mut out_adapter, input_len, None)?;
    trace!(
        "[resample] process_all_into_buffer consumed {} frames, produced {} frames",
        in_used, out_produced
    );
    debug!("[resample] resampling done in {:.3?}", t_resample.elapsed());

    let t_i16 = Instant::now();
    let resample_ratio = output_sample_rate as f64 / input_sample_rate as f64;
    let expected_len = ((input_samples.len() as f64) * resample_ratio).round() as usize;
    let mut output_i16: Vec<i16> = output_f32[0]
        .iter()
        .take(out_produced)
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    if output_i16.len() > expected_len {
        output_i16.truncate(expected_len);
    }
    debug!(
        "[resample] f32->i16 conversion done in {:.3?}",
        t_i16.elapsed()
    );
    debug!(
        "[resample] total elapsed: {:.3?} (input {} -> output {} samples)",
        total_start.elapsed(),
        input_samples.len(),
        output_i16.len()
    );
    Ok(output_i16)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Identity case: equal sample rates SHALL bypass the resampler.
    #[test]
    fn identical_sample_rates_returns_input_unchanged() {
        let input: Vec<i16> = (0..16_000).map(|i| (i % 1024) as i16).collect();
        let out = resample_to_target_rate(&input, 16_000, 16_000).expect("resample succeeds");
        assert_eq!(out, input);
    }

    /// Empty input SHALL produce an empty output without panicking.
    #[test]
    fn empty_input_returns_empty() {
        let out = resample_to_target_rate(&[], 44_100, 16_000).expect("resample succeeds");
        assert!(out.is_empty());
    }

    /// Downsampling 1 second of audio from 44.1 kHz to 16 kHz SHALL
    /// produce ~16 000 frames (within a tight tolerance because rubato
    /// `process_all_into_buffer` already trims the resampler delay).
    #[test]
    fn downsample_44100_to_16000_length_matches_ratio() {
        let input: Vec<i16> = (0..44_100).map(|i| (i % 1024) as i16).collect();
        let out = resample_to_target_rate(&input, 44_100, 16_000).expect("resample succeeds");
        let ratio = 16_000.0 / 44_100.0;
        let expected = (input.len() as f64 * ratio).round() as usize;
        assert!(
            out.len().abs_diff(expected) <= 8,
            "expected ~{expected} frames, got {}",
            out.len()
        );
    }

    /// Upsampling SHALL also respect the ratio.
    #[test]
    fn upsample_16000_to_48000_length_matches_ratio() {
        let input: Vec<i16> = (0..16_000).map(|i| (i % 1024) as i16).collect();
        let out = resample_to_target_rate(&input, 16_000, 48_000).expect("resample succeeds");
        let ratio = 48_000.0 / 16_000.0;
        let expected = (input.len() as f64 * ratio).round() as usize;
        assert!(
            out.len().abs_diff(expected) <= 8,
            "expected ~{expected} frames, got {}",
            out.len()
        );
    }
}
