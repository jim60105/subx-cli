//! Direct audio loader: Uses Symphonia to directly decode various audio formats and obtain i16 sample data.
//!
//! Supports MP4, MKV, OGG, WAV and other formats, returning sample data and audio information.
use crate::services::vad::detector::AudioInfo;
use crate::{Result, error::SubXError};
use log::{debug, trace, warn};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::CodecRegistry;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use symphonia::core::probe::Probe;
use symphonia::default::{get_codecs, get_probe};

/// Direct audio loader using Symphonia to decode and obtain raw sample data.
pub struct DirectAudioLoader {
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_mp4_loading() {
        // Test direct audio loading using assets/SubX - The Subtitle Revolution.mp4
        let loader = DirectAudioLoader::new().expect("Failed to initialize DirectAudioLoader");
        let (samples, info) = loader
            .load_audio_samples("assets/SubX - The Subtitle Revolution.mp4")
            .expect("load_audio_samples failed");
        assert!(!samples.is_empty(), "Sample data should not be empty");
        assert!(info.sample_rate > 0, "sample_rate should be greater than 0");
        assert!(
            info.total_samples > 0,
            "total_samples should be greater than 0"
        );
    }
}

impl DirectAudioLoader {
    /// Creates a new audio loader instance.
    pub fn new() -> Result<Self> {
        Ok(Self {
            probe: get_probe(),
            codecs: get_codecs(),
        })
    }

    /// Loads i16 samples and audio information from an audio file path.
    pub fn load_audio_samples<P: AsRef<Path>>(&self, path: P) -> Result<(Vec<i16>, AudioInfo)> {
        let path_ref = path.as_ref();
        debug!(
            "[DirectAudioLoader] Start loading audio file: {:?}",
            path_ref
        );
        // Open the media source.
        let file = File::open(path_ref).map_err(|e| {
            warn!(
                "[DirectAudioLoader] Failed to open audio file: {:?}, error: {}",
                path_ref, e
            );
            SubXError::audio_processing(format!("Failed to open audio file: {}", e))
        })?;
        debug!(
            "[DirectAudioLoader] Successfully opened audio file: {:?}",
            path_ref
        );

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        debug!("[DirectAudioLoader] MediaSourceStream created");

        // Create a hint to help format probing based on file extension.
        let mut hint = Hint::new();
        if let Some(ext) = path_ref.extension().and_then(|e| e.to_str()) {
            debug!(
                "[DirectAudioLoader] Detected extension: {} (used for format probing)",
                ext
            );
            hint.with_extension(ext);
        } else {
            debug!("[DirectAudioLoader] No extension detected, using default format probing");
        }

        // Probe the media format.
        let probed = self
            .probe
            .format(&hint, mss, &FormatOptions::default(), &Default::default())
            .map_err(|e| {
                warn!("[DirectAudioLoader] Format probing failed: {}", e);
                SubXError::audio_processing(format!("Failed to probe format: {}", e))
            })?;
        debug!("[DirectAudioLoader] Format probing succeeded");
        let mut format = probed.format;

        // List all tracks and their channel info before selecting
        for (idx, t) in format.tracks().iter().enumerate() {
            let sr = t
                .codec_params
                .sample_rate
                .map(|v| v.to_string())
                .unwrap_or("None".to_string());
            let ch = t
                .codec_params
                .channels
                .map(|c| c.count().to_string())
                .unwrap_or("None".to_string());
            debug!(
                "[DirectAudioLoader] Track[{}]: id={}, sample_rate={}, channels={}",
                idx, t.id, sr, ch
            );
        }

        // Select the first audio track that contains sample_rate as the audio source.
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.sample_rate.is_some())
            .ok_or_else(|| {
                warn!("[DirectAudioLoader] No audio track with sample_rate found");
                SubXError::audio_processing("No audio track found".to_string())
            })?;
        // Clone necessary info first to avoid borrow conflicts
        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.ok_or_else(|| {
            warn!("[DirectAudioLoader] Audio track sample_rate is unknown");
            SubXError::audio_processing("Sample rate unknown".to_string())
        })?;
        let channels = track.codec_params.channels.map(|c| c.count() as u16);
        let time_base = track.codec_params.time_base;
        debug!(
            "[DirectAudioLoader] Selected track: id={}, sample_rate={}, channels={:?}",
            track_id, sample_rate, channels
        );

        // Create decoder for the track.
        let dec_opts = DecoderOptions::default();
        let mut decoder = self
            .codecs
            .make(&track.codec_params, &dec_opts)
            .map_err(|e| {
                warn!("[DirectAudioLoader] Failed to create decoder: {}", e);
                SubXError::audio_processing(format!("Failed to create decoder: {}", e))
            })?;
        debug!("[DirectAudioLoader] Decoder created successfully");

        // Decode packets and collect samples.
        let mut samples = Vec::new();
        let mut packet_count = 0;
        let mut last_pts: u64 = 0;
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != track_id {
                continue;
            }
            packet_count += 1;
            trace!(
                "[DirectAudioLoader] Decoding packet {} (track_id={})",
                packet_count, track_id
            );
            let decoded = decoder.decode(&packet).map_err(|e| {
                warn!("[DirectAudioLoader] Failed to decode packet: {}", e);
                SubXError::audio_processing(format!("Decode error: {}", e))
            })?;
            // Create a sample buffer for this packet using its signal spec and capacity.
            let spec = *decoded.spec();
            let mut sample_buf = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            let sample_len = sample_buf.samples().len();
            trace!(
                "[DirectAudioLoader] Packet decoded successfully, got {} samples",
                sample_len
            );
            samples.extend_from_slice(sample_buf.samples());
            // Directly record the timestamp of the last packet
            last_pts = packet.ts;
        }
        debug!(
            "[DirectAudioLoader] Packet decoding finished, total {} packets, {} samples accumulated",
            packet_count,
            samples.len()
        );

        // Calculate total samples and audio duration
        let total_samples = samples.len();
        // Use Timebase to calculate duration_seconds
        let duration_seconds = if let Some(tb) = time_base {
            if last_pts > 0 {
                let (num, den) = (tb.numer, tb.denom);
                last_pts as f64 * num as f64 / den as f64
            } else {
                total_samples as f64 / (sample_rate as f64 * channels.unwrap_or(1) as f64)
            }
        } else {
            total_samples as f64 / (sample_rate as f64 * channels.unwrap_or(1) as f64)
        };
        // If channels is None, try to infer channel count from duration_seconds
        let channels = channels.unwrap_or_else(|| {
            let ch = if duration_seconds > 0.0 {
                (total_samples as f64 / (sample_rate as f64 * duration_seconds)).round() as u16
            } else {
                1
            };
            debug!("[DirectAudioLoader] Inferred channel count: {}", ch);
            ch
        });
        debug!(
            "[DirectAudioLoader] Audio info: sample_rate={}, channels={}, duration_seconds={:.3}, total_samples={}",
            sample_rate, channels, duration_seconds, total_samples
        );

        Ok((
            samples,
            AudioInfo {
                sample_rate,
                channels,
                duration_seconds,
                total_samples,
            },
        ))
    }
}
