//! Audio transcoding service: Multi-format to WAV conversion based on Symphonia.

use crate::{Result, error::SubXError};
use hound::{SampleFormat, WavSpec, WavWriter};
use log::warn;
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::{
    audio::{Layout, SampleBuffer},
    codecs::CODEC_TYPE_NULL,
    io::MediaSourceStream,
};
use symphonia::core::{codecs::CodecRegistry, probe::Probe};
use symphonia::default::{get_codecs, get_probe};
use tempfile::TempDir;
/// Audio transcoder: Detects file format and converts non-WAV files to WAV.
pub struct AudioTranscoder {
    /// Temporary directory for storing transcoding results
    temp_dir: TempDir,
    probe: &'static Probe,
    codecs: &'static CodecRegistry,
}

/// Audio transcoding statistics
pub struct TranscodeStats {
    /// Total number of packets processed
    pub total_packets: u64,
    /// Number of successfully decoded packets
    pub decoded_packets: u64,
    /// Number of packets skipped due to DecodeError
    pub skipped_decode_errors: u64,
    /// Number of packets skipped due to IoError
    pub skipped_io_errors: u64,
    /// Number of times reset was required
    pub reset_required_count: u64,
}

impl TranscodeStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_packets: 0,
            decoded_packets: 0,
            skipped_decode_errors: 0,
            skipped_io_errors: 0,
            reset_required_count: 0,
        }
    }

    /// Calculate decoding success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_packets == 0 {
            0.0
        } else {
            self.decoded_packets as f64 / self.total_packets as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    /// Create a minimal WAV file for testing transcoding.
    fn create_minimal_wav_file(dir: &TempDir) -> PathBuf {
        let path = dir.path().join("test.wav");
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).unwrap();
        writer.write_sample(0i16).unwrap();
        writer.finalize().unwrap();
        path
    }

    #[test]
    fn test_needs_transcoding() {
        let transcoder = AudioTranscoder::new().expect("Failed to create transcoder");
        assert!(transcoder.needs_transcoding("test.mp4").unwrap());
        assert!(transcoder.needs_transcoding("test.MKV").unwrap());
        assert!(transcoder.needs_transcoding("test.ogg").unwrap());
        assert!(!transcoder.needs_transcoding("test.wav").unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn test_transcode_wav_to_wav() {
        let transcoder = AudioTranscoder::new().expect("Failed to create transcoder");
        let temp_dir = TempDir::new().unwrap();
        let wav_path = create_minimal_wav_file(&temp_dir);
        let out_path = transcoder
            .transcode_to_wav(&wav_path)
            .await
            .expect("Transcode failed");
        assert_eq!(out_path.extension().and_then(|e| e.to_str()), Some("wav"));
        let meta = std::fs::metadata(&out_path).expect("Failed to stat output file");
        assert!(meta.len() > 0, "Output WAV file should not be empty");
    }

    #[test]
    fn test_transcode_stats_success_rate() {
        let mut stats = TranscodeStats::new();
        assert_eq!(stats.success_rate(), 0.0);
        stats.total_packets = 10;
        stats.decoded_packets = 7;
        let rate = stats.success_rate();
        assert!(
            (rate - 0.7).abs() < f64::EPSILON,
            "Expected 0.7, got {}",
            rate
        );
    }
}

impl AudioTranscoder {
    /// Create a new AudioTranscoder instance and initialize temporary directory.
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new().map_err(|e| {
            SubXError::audio_processing(format!("Failed to create temp dir: {}", e))
        })?;
        let probe = get_probe();
        let codecs = get_codecs();
        Ok(Self {
            temp_dir,
            probe,
            codecs,
        })
    }

    /// Check if the audio file at the specified path needs transcoding (based on file extension).
    pub fn needs_transcoding<P: AsRef<Path>>(&self, audio_path: P) -> Result<bool> {
        if let Some(ext) = audio_path.as_ref().extension().and_then(|s| s.to_str()) {
            let ext_lc = ext.to_lowercase();
            if ext_lc == "wav" { Ok(false) } else { Ok(true) }
        } else {
            Err(SubXError::audio_processing(
                "Missing file extension".to_string(),
            ))
        }
    }

    /// Actively clean up temporary directory
    pub fn cleanup(self) -> Result<()> {
        self.temp_dir
            .close()
            .map_err(|e| SubXError::audio_processing(format!("Failed to clean temp dir: {}", e)))?;
        Ok(())
    }
}

impl AudioTranscoder {
    /// Audio transcoding method with configuration, allowing specification of minimum success rate
    pub async fn transcode_to_wav_with_config<P: AsRef<Path>>(
        &self,
        input_path: P,
        min_success_rate: Option<f64>,
    ) -> Result<(PathBuf, TranscodeStats)> {
        use symphonia::core::errors::Error as SymphoniaError;

        let input = input_path.as_ref();
        let min_success_rate = min_success_rate.unwrap_or(0.5);
        let mut stats = TranscodeStats::new();

        let file = File::open(input).map_err(|e| {
            SubXError::audio_processing(format!(
                "Failed to open input file {}: {}",
                input.display(),
                e
            ))
        })?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let probed = self
            .probe
            .format(
                &Default::default(),
                mss,
                &Default::default(),
                &Default::default(),
            )
            .map_err(|e| SubXError::audio_processing(format!("Format probe error: {}", e)))?;
        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SubXError::audio_processing("No audio track found".to_string()))?;

        let mut decoder = self
            .codecs
            .make(&track.codec_params, &Default::default())
            .map_err(|e| SubXError::audio_processing(format!("Decoder error: {}", e)))?;

        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let layout = track.codec_params.channel_layout.unwrap_or(Layout::Stereo);
        let channels = layout.into_channels().count() as u16;
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let wav_path = self
            .temp_dir
            .path()
            .join(input.file_stem().unwrap_or_default())
            .with_extension("wav");
        let mut writer = WavWriter::create(&wav_path, spec)
            .map_err(|e| SubXError::audio_processing(format!("WAV writer error: {}", e)))?;

        loop {
            stats.total_packets += 1;
            match format.next_packet() {
                Ok(packet) => match decoder.decode(&packet) {
                    Ok(audio_buf) => {
                        stats.decoded_packets += 1;

                        let mut sample_buf = SampleBuffer::<i16>::new(
                            audio_buf.capacity() as u64,
                            *audio_buf.spec(),
                        );
                        sample_buf.copy_interleaved_ref(audio_buf);
                        for sample in sample_buf.samples() {
                            writer.write_sample(*sample).map_err(|e| {
                                SubXError::audio_processing(format!("Write sample error: {}", e))
                            })?;
                        }
                    }
                    Err(SymphoniaError::DecodeError(decode_err)) => {
                        warn!(
                            "Decode error (recoverable), skipping packet: {}",
                            decode_err
                        );
                        stats.skipped_decode_errors += 1;
                        continue;
                    }
                    Err(SymphoniaError::IoError(io_err)) => {
                        warn!("I/O error (recoverable), skipping packet: {}", io_err);
                        stats.skipped_io_errors += 1;
                        continue;
                    }
                    Err(SymphoniaError::ResetRequired) => {
                        warn!("Decoder reset required, audio specs may change");
                        stats.reset_required_count += 1;
                        continue;
                    }
                    Err(other) => {
                        return Err(SubXError::audio_processing(format!(
                            "Unrecoverable decode error: {}",
                            other
                        )));
                    }
                },
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    return Err(SubXError::audio_processing(format!(
                        "Packet read error: {}",
                        e
                    )));
                }
            }
        }

        writer
            .finalize()
            .map_err(|e| SubXError::audio_processing(format!("Finalize WAV error: {}", e)))?;

        if stats.success_rate() < min_success_rate {
            warn!(
                "Final decode success rate ({:.1}%) is below minimum threshold ({:.1}%)",
                stats.success_rate() * 100.0,
                min_success_rate * 100.0
            );
        }

        if stats.total_packets > 10 && stats.success_rate() < min_success_rate {
            return Err(SubXError::audio_processing(format!(
                "Decode success rate ({:.1}%) below minimum threshold ({:.1}%), output quality unacceptable",
                stats.success_rate() * 100.0,
                min_success_rate * 100.0
            )));
        }

        Ok((wav_path, stats))
    }

    /// Transcode input audio file to WAV and save to temporary directory (backward compatibility).
    pub async fn transcode_to_wav<P: AsRef<Path>>(&self, input_path: P) -> Result<PathBuf> {
        let (path, stats) = self.transcode_to_wav_with_config(input_path, None).await?;
        if stats.success_rate() < 0.8 {
            warn!(
                "Low decode success rate ({:.1}%), output quality may be affected",
                stats.success_rate() * 100.0
            );
        }
        Ok(path)
    }
}
