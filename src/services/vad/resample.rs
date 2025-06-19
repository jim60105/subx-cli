//!
//! Audio resampling utilities using the rubato crate.
//! Supports i16 <-> f32 conversion and multi-channel processing.

use log::{debug, trace};
use rubato::{FftFixedIn, Resampler};
use std::error::Error;
use std::time::Instant;

use once_cell::sync::Lazy;
use std::sync::Mutex;

type FftResamplerCache = Option<(u32, u32, FftFixedIn<f32>)>;

static FFT_RESAMPLER_CACHE: Lazy<Mutex<FftResamplerCache>> = Lazy::new(|| Mutex::new(None));

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
    let t_convert = Instant::now();
    // Convert to f32 and minimize allocation and copying
    let input: Vec<f32> = input_samples.iter().map(|&s| s as f32 / 32768.0).collect();
    debug!(
        "[resample] i16->f32 conversion done in {:.3?}",
        t_convert.elapsed()
    );
    let input_len = input.len();
    let input_channels = 1;
    let resample_ratio = output_sample_rate as f64 / input_sample_rate as f64;
    let chunk_size = 8192; // Increase chunk size to reduce the number of function calls
    let t_resampler_init = Instant::now();
    let mut resampler = {
        let mut cache = FFT_RESAMPLER_CACHE.lock().unwrap();
        if let Some((in_sr, out_sr, ref mut cached)) = *cache {
            if in_sr == input_sample_rate && out_sr == output_sample_rate {
                debug!("[resample] using cached FftFixedIn");
                cached.reset();
                let mut new_resampler = FftFixedIn::<f32>::new(
                    input_sample_rate as usize,
                    output_sample_rate as usize,
                    chunk_size,
                    1, // sub_chunks
                    input_channels,
                )?;
                std::mem::swap(&mut new_resampler, cached);
                new_resampler
            } else {
                debug!("[resample] creating new FftFixedIn (cache miss)");
                let new_resampler = FftFixedIn::<f32>::new(
                    input_sample_rate as usize,
                    output_sample_rate as usize,
                    chunk_size,
                    1, // sub_chunks
                    input_channels,
                )?;
                *cache = Some((input_sample_rate, output_sample_rate, new_resampler));
                let mut new_resampler = FftFixedIn::<f32>::new(
                    input_sample_rate as usize,
                    output_sample_rate as usize,
                    chunk_size,
                    1, // sub_chunks
                    input_channels,
                )?;
                std::mem::swap(&mut new_resampler, &mut cache.as_mut().unwrap().2);
                new_resampler
            }
        } else {
            debug!("[resample] creating new FftFixedIn (cache empty)");
            let new_resampler = FftFixedIn::<f32>::new(
                input_sample_rate as usize,
                output_sample_rate as usize,
                chunk_size,
                1, // sub_chunks
                input_channels,
            )?;
            *cache = Some((input_sample_rate, output_sample_rate, new_resampler));
            let mut new_resampler = FftFixedIn::<f32>::new(
                input_sample_rate as usize,
                output_sample_rate as usize,
                chunk_size,
                1, // sub_chunks
                input_channels,
            )?;
            std::mem::swap(&mut new_resampler, &mut cache.as_mut().unwrap().2);
            new_resampler
        }
    };
    debug!(
        "[resample] FftFixedIn ready in {:.3?}",
        t_resampler_init.elapsed()
    );
    let mut output: Vec<f32> =
        Vec::with_capacity((input_len as f64 * resample_ratio) as usize + 128);
    let mut pos = 0;
    let t_resample = Instant::now();
    let mut chunk_count = 0;
    // Correction: each chunk must be input frame of chunk_size, pad with 0 if not enough at the end
    while pos < input_len {
        let frames_needed = resampler.input_frames_next();
        let end = (pos + frames_needed).min(input_len);
        let mut chunk: Vec<f32> = Vec::with_capacity(frames_needed);
        chunk.extend_from_slice(&input[pos..end]);
        if end - pos < frames_needed {
            // Pad with 0 until frames_needed is reached
            chunk.resize(frames_needed, 0.0);
        }
        let chunk_ref = [&chunk[..]];
        trace!(
            "[resample] chunk {}: pos={} end={} frames_needed={}",
            chunk_count, pos, end, frames_needed
        );
        let out_chunk = resampler.process(&chunk_ref, None)?;
        output.extend_from_slice(&out_chunk[0]);
        pos += frames_needed;
        chunk_count += 1;
    }
    debug!(
        "[resample] main resample loop done in {:.3?} ({} chunks)",
        t_resample.elapsed(),
        chunk_count
    );
    // flush
    let t_flush = Instant::now();
    let mut flush_count = 0;
    loop {
        let out_chunk = resampler.process_partial::<Vec<f32>>(None, None)?;
        if out_chunk[0].is_empty() {
            break;
        }
        output.extend_from_slice(&out_chunk[0]);
        flush_count += 1;
    }
    debug!(
        "[resample] flush done in {:.3?} ({} flushes)",
        t_flush.elapsed(),
        flush_count
    );
    // f32 -> i16
    let t_i16 = Instant::now();
    // Correction: only keep samples with correct length
    let expected_len = ((input_samples.len() as f64) * resample_ratio).round() as usize;
    let mut output_i16: Vec<i16> = output
        .iter()
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
