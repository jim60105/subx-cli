//! Mock data generators, providing generation functionality for audio, subtitles and other test data.

use std::path::Path;
use std::time::Duration;
use subx_cli::{Result, error::SubXError};
use tokio::fs;

/// Dialogue segment information
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start: f64,
    pub end: f64,
    pub is_speech: bool,
}

/// Audio metadata
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub duration: f64,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub channels: u32,
    pub dialogue_segments: Vec<DialogueSegment>,
}

/// Audio file simulator
pub struct AudioMockGenerator {
    sample_rate: u32,
    duration: f64,
    channels: u32,
}

impl AudioMockGenerator {
    /// Create new audio simulator
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            duration: 5.0, // Default 5 seconds
            channels: 1,   // Default mono
        }
    }

    /// Set audio duration
    pub fn with_duration(mut self, duration: f64) -> Self {
        self.duration = duration;
        self
    }

    /// Set number of channels
    #[allow(dead_code)]
    pub fn with_channels(mut self, channels: u32) -> Self {
        self.channels = channels;
        self
    }

    /// Generate audio file containing dialogue
    pub async fn generate_dialogue_audio(&self, path: &Path) -> Result<AudioMetadata> {
        // Generate audio segments containing dialogue and silence
        let dialogue_segments = vec![
            DialogueSegment {
                start: 0.0,
                end: 1.0,
                is_speech: false,
            }, // Silence
            DialogueSegment {
                start: 1.0,
                end: 3.0,
                is_speech: true,
            }, // Dialogue
            DialogueSegment {
                start: 3.0,
                end: 3.5,
                is_speech: false,
            }, // Silence
            DialogueSegment {
                start: 3.5,
                end: 5.0,
                is_speech: true,
            }, // Dialogue
        ];

        let samples = self.generate_samples_from_segments(&dialogue_segments);
        self.write_wav_file(path, &samples).await?;

        Ok(AudioMetadata {
            duration: self.duration,
            sample_rate: self.sample_rate,
            channels: self.channels,
            dialogue_segments,
        })
    }

    /// Generate pure music audio file (no dialogue)
    #[allow(dead_code)]
    pub async fn generate_music_audio(&self, path: &Path) -> Result<AudioMetadata> {
        // Generate pure music audio (no dialogue)
        let samples = self.generate_sine_wave(440.0); // A4 note
        self.write_wav_file(path, &samples).await?;

        Ok(AudioMetadata {
            duration: self.duration,
            sample_rate: self.sample_rate,
            channels: self.channels,
            dialogue_segments: vec![], // No dialogue segments
        })
    }

    /// Generate silence audio file
    #[allow(dead_code)]
    pub async fn generate_silence_audio(&self, path: &Path) -> Result<AudioMetadata> {
        let samples = vec![0.0; (self.sample_rate as f64 * self.duration) as usize];
        self.write_wav_file(path, &samples).await?;

        Ok(AudioMetadata {
            duration: self.duration,
            sample_rate: self.sample_rate,
            channels: self.channels,
            dialogue_segments: vec![DialogueSegment {
                start: 0.0,
                end: self.duration,
                is_speech: false,
            }],
        })
    }

    /// Generate audio samples from dialogue segments
    fn generate_samples_from_segments(&self, segments: &[DialogueSegment]) -> Vec<f32> {
        let total_samples = (self.sample_rate as f64 * self.duration) as usize;
        let mut samples = vec![0.0; total_samples];

        for segment in segments {
            let start_sample = (segment.start * self.sample_rate as f64) as usize;
            let end_sample = (segment.end * self.sample_rate as f64) as usize;

            if segment.is_speech {
                // Generate speech samples (simple sine wave mix)
                #[allow(clippy::needless_range_loop)]
                for i in start_sample..end_sample.min(total_samples) {
                    let t = i as f32 / self.sample_rate as f32;
                    // Mix multiple frequencies to simulate voice
                    samples[i] = 0.3 * (440.0 * 2.0 * std::f32::consts::PI * t).sin()
                        + 0.2 * (880.0 * 2.0 * std::f32::consts::PI * t).sin()
                        + 0.1 * (1320.0 * 2.0 * std::f32::consts::PI * t).sin();
                }
            }
            // Silence segments remain 0.0
        }

        samples
    }

    /// Generate sine wave
    #[allow(dead_code)]
    fn generate_sine_wave(&self, frequency: f64) -> Vec<f32> {
        let total_samples = (self.sample_rate as f64 * self.duration) as usize;
        let mut samples = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f64 / self.sample_rate as f64;
            let sample = (frequency * 2.0 * std::f64::consts::PI * t).sin() as f32;
            samples.push(sample * 0.5); // Reduce volume
        }

        samples
    }

    /// Write WAV file
    async fn write_wav_file(&self, path: &Path, samples: &[f32]) -> Result<()> {
        // Calculate correct number of samples
        let total_samples = (self.sample_rate as f64 * self.duration) as usize;
        let actual_samples = if samples.len() > total_samples {
            &samples[..total_samples]
        } else {
            samples
        };

        // WAV header
        let mut wav_data = Vec::new();
        let data_size = actual_samples.len() * 2 * self.channels as usize; // 16-bit samples
        let file_size = 36 + data_size;

        // RIFF header
        wav_data.extend_from_slice(b"RIFF");
        wav_data.extend_from_slice(&(file_size as u32).to_le_bytes());
        wav_data.extend_from_slice(b"WAVE");

        // fmt chunk
        wav_data.extend_from_slice(b"fmt ");
        wav_data.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav_data.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
        wav_data.extend_from_slice(&(self.channels as u16).to_le_bytes());
        wav_data.extend_from_slice(&self.sample_rate.to_le_bytes());
        wav_data.extend_from_slice(&(self.sample_rate * self.channels * 2).to_le_bytes()); // byte rate
        wav_data.extend_from_slice(&((self.channels * 2) as u16).to_le_bytes()); // block align
        wav_data.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

        // data chunk
        wav_data.extend_from_slice(b"data");
        wav_data.extend_from_slice(&(data_size as u32).to_le_bytes());

        // Audio data (convert to 16-bit PCM)
        for sample in actual_samples {
            let sample_16 = (*sample * 32767.0).clamp(-32767.0, 32767.0) as i16;
            wav_data.extend_from_slice(&sample_16.to_le_bytes());
        }

        fs::write(path, wav_data).await.map_err(|e| {
            SubXError::FileOperationFailed(format!("Failed to write WAV file: {}", e))
        })?;

        Ok(())
    }
}

/// Subtitle format
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum SubtitleFormat {
    Srt,
    #[allow(dead_code)]
    Ass,
    #[allow(dead_code)]
    Vtt,
}

/// Subtitle entry
#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub start_time: Duration,
    pub end_time: Duration,
    pub text: String,
}

/// Subtitle file generator
pub struct SubtitleGenerator {
    format: SubtitleFormat,
    entries: Vec<SubtitleEntry>,
    #[allow(dead_code)]
    encoding: String,
}

impl SubtitleGenerator {
    /// Create new subtitle generator
    pub fn new(format: SubtitleFormat) -> Self {
        Self {
            format,
            entries: Vec::new(),
            encoding: "UTF-8".to_string(),
        }
    }

    /// Set encoding
    #[allow(dead_code)]
    pub fn with_encoding(mut self, encoding: &str) -> Self {
        self.encoding = encoding.to_string();
        self
    }

    /// Add subtitle entry
    pub fn add_entry(mut self, start: f64, end: f64, text: &str) -> Self {
        self.entries.push(SubtitleEntry {
            start_time: Duration::from_secs_f64(start),
            end_time: Duration::from_secs_f64(end),
            text: text.to_string(),
        });
        self
    }

    /// Generate typical movie subtitles
    #[allow(dead_code)]
    pub fn generate_typical_movie(mut self) -> Self {
        // Generate typical movie subtitle pattern
        let dialogues = vec![
            (5.0, 8.0, "The movie has started"),
            (
                10.0,
                15.0,
                "This is the first dialogue, a bit longer content",
            ),
            (18.0, 20.0, "Short dialogue"),
            (25.0, 30.0, "Response from another character"),
            (35.0, 40.0, "More dialogue content"),
            (45.0, 50.0, "Plot development..."),
        ];

        for (start, end, text) in dialogues {
            self = self.add_entry(start, end, text);
        }

        self
    }

    /// Generate short test subtitles
    #[allow(dead_code)]
    pub fn generate_short_test(mut self) -> Self {
        let dialogues = vec![(1.0, 3.0, "Test subtitle 1"), (4.0, 6.0, "Test subtitle 2")];

        for (start, end, text) in dialogues {
            self = self.add_entry(start, end, text);
        }

        self
    }

    /// Save to file
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = match self.format {
            SubtitleFormat::Srt => self.generate_srt_content(),
            SubtitleFormat::Ass => self.generate_ass_content(),
            SubtitleFormat::Vtt => self.generate_vtt_content(),
        };

        fs::write(path, content).await.map_err(|e| {
            SubXError::FileOperationFailed(format!("Failed to write subtitle file: {}", e))
        })?;

        Ok(())
    }

    /// Generate SRT format content
    fn generate_srt_content(&self) -> String {
        let mut content = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            content.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                format_srt_time(entry.start_time),
                format_srt_time(entry.end_time),
                entry.text
            ));
        }
        content
    }

    /// Generate ASS format content
    fn generate_ass_content(&self) -> String {
        let mut content = String::new();
        content.push_str("[Script Info]\nTitle: Test Subtitle\n\n");
        content.push_str("[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,16,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,2,2,10,10,10,1\n\n");
        content.push_str("[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");

        for entry in &self.entries {
            content.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
                format_ass_time(entry.start_time),
                format_ass_time(entry.end_time),
                entry.text
            ));
        }
        content
    }

    /// Generate VTT format content
    fn generate_vtt_content(&self) -> String {
        let mut content = String::new();
        content.push_str("WEBVTT\n\n");

        for entry in &self.entries {
            content.push_str(&format!(
                "{} --> {}\n{}\n\n",
                format_vtt_time(entry.start_time),
                format_vtt_time(entry.end_time),
                entry.text
            ));
        }
        content
    }
}

/// Format SRT time
fn format_srt_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let milliseconds = total_ms % 1000;

    format!(
        "{:02}:{:02}:{:02},{:03}",
        hours, minutes, seconds, milliseconds
    )
}

/// Format ASS time
fn format_ass_time(duration: Duration) -> String {
    let total_centiseconds = duration.as_millis() / 10;
    let hours = total_centiseconds / 360000;
    let minutes = (total_centiseconds % 360000) / 6000;
    let seconds = (total_centiseconds % 6000) / 100;
    let centiseconds = total_centiseconds % 100;

    format!(
        "{}:{:02}:{:02}.{:02}",
        hours, minutes, seconds, centiseconds
    )
}

/// Format VTT time
fn format_vtt_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let milliseconds = total_ms % 1000;

    if hours > 0 {
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            hours, minutes, seconds, milliseconds
        )
    } else {
        format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_audio_mock_generator() {
        let temp_dir = TempDir::new().unwrap();
        let audio_path = temp_dir.path().join("test_audio.wav");

        let generator = AudioMockGenerator::new(44100).with_duration(2.0);
        let metadata = generator
            .generate_dialogue_audio(&audio_path)
            .await
            .unwrap();

        assert!(audio_path.exists());
        assert_eq!(metadata.duration, 2.0);
        assert_eq!(metadata.sample_rate, 44100);
        assert!(!metadata.dialogue_segments.is_empty());
    }

    #[tokio::test]
    async fn test_subtitle_generator_srt() {
        let temp_dir = TempDir::new().unwrap();
        let subtitle_path = temp_dir.path().join("test.srt");

        let generator =
            SubtitleGenerator::new(SubtitleFormat::Srt).add_entry(1.0, 3.0, "Test subtitle");

        generator.save_to_file(&subtitle_path).await.unwrap();

        assert!(subtitle_path.exists());
        let content = fs::read_to_string(&subtitle_path).await.unwrap();
        assert!(content.contains("1\n"));
        assert!(content.contains("Test subtitle"));
    }

    #[test]
    fn test_format_srt_time() {
        let duration = Duration::from_millis(3661500); // 1:01:01.500
        let formatted = format_srt_time(duration);
        assert_eq!(formatted, "01:01:01,500");
    }

    #[test]
    fn test_format_vtt_time() {
        let duration = Duration::from_millis(3661500); // 1:01:01.500
        let formatted = format_vtt_time(duration);
        assert_eq!(formatted, "01:01:01.500");
    }
}
