use crate::Result;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use regex::Regex;
use std::time::Duration;

const DEFAULT_SUB_FPS: f32 = 25.0;

/// Subtitle format implementation for MicroDVD/SubViewer SUB.
///
/// The `SubFormat` struct implements parsing, serialization, and
/// detection for SUB files using frame-based timing.
pub struct SubFormat;

impl SubtitleFormat for SubFormat {
    fn parse(&self, content: &str) -> Result<Subtitle> {
        let fps = DEFAULT_SUB_FPS;
        let re = Regex::new(r"^\{(\d+)\}\{(\d+)\}(.*)").map_err(|e: regex::Error| {
            SubXError::subtitle_format(self.format_name(), e.to_string())
        })?;
        let mut entries = Vec::new();
        for line in content.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if let Some(cap) = re.captures(l) {
                let start_frame: u64 = cap[1].parse().map_err(|e: std::num::ParseIntError| {
                    SubXError::subtitle_format(self.format_name(), e.to_string())
                })?;
                let end_frame: u64 = cap[2].parse().map_err(|e: std::num::ParseIntError| {
                    SubXError::subtitle_format(self.format_name(), e.to_string())
                })?;
                let text = cap[3].replace("|", "\n");
                let start_time = Duration::from_millis(
                    (start_frame as f64 * 1000.0 / fps as f64).round() as u64,
                );
                let end_time =
                    Duration::from_millis((end_frame as f64 * 1000.0 / fps as f64).round() as u64);
                entries.push(SubtitleEntry {
                    index: entries.len() + 1,
                    start_time,
                    end_time,
                    text,
                    styling: None,
                });
            }
        }
        Ok(Subtitle {
            entries,
            metadata: SubtitleMetadata {
                title: None,
                language: None,
                encoding: "utf-8".to_string(),
                frame_rate: Some(fps),
                original_format: SubtitleFormatType::Sub,
            },
            format: SubtitleFormatType::Sub,
        })
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        let fps = subtitle.metadata.frame_rate.unwrap_or(DEFAULT_SUB_FPS);
        let mut output = String::new();
        for entry in &subtitle.entries {
            let start_frame = (entry.start_time.as_secs_f64() * fps as f64).round() as u64;
            let end_frame = (entry.end_time.as_secs_f64() * fps as f64).round() as u64;
            let text = entry.text.replace("\n", "|");
            output.push_str(&format!("{{{}}}{{{}}}{}\n", start_frame, end_frame, text));
        }
        Ok(output)
    }

    fn detect(&self, content: &str) -> bool {
        if let Ok(re) = Regex::new(r"^\{\d+\}\{\d+\}") {
            return re.is_match(content.trim_start());
        }
        false
    }

    fn format_name(&self) -> &'static str {
        "SUB"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["sub"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "{10}{20}Hello|World\n";

    #[test]
    fn test_parse_and_serialize() {
        let fmt = SubFormat;
        let subtitle = fmt.parse(SAMPLE).expect("parse failed");
        assert_eq!(subtitle.entries.len(), 1);
        let out = fmt.serialize(&subtitle).expect("serialize failed");
        assert!(out.contains("{10}{20}Hello|World"));
    }

    #[test]
    fn test_detect_true_and_false() {
        let fmt = SubFormat;
        assert!(fmt.detect(SAMPLE));
        assert!(!fmt.detect("random text"));
    }

    #[test]
    fn test_parse_multiple_and_frame_rate() {
        let custom = "{0}{25}First|Line\n{25}{50}Second|Line\n";
        let fmt = SubFormat;
        let subtitle = fmt.parse(custom).expect("parse multiple failed");
        assert_eq!(subtitle.entries.len(), 2);
        assert_eq!(subtitle.metadata.frame_rate, Some(25.0));
        assert_eq!(subtitle.entries[0].text, "First\nLine");
        assert_eq!(subtitle.entries[1].text, "Second\nLine");
    }

    #[test]
    fn test_serialize_with_nondefault_fps() {
        let mut subtitle = Subtitle {
            entries: Vec::new(),
            metadata: SubtitleMetadata {
                title: None,
                language: None,
                encoding: "utf-8".to_string(),
                frame_rate: Some(50.0),
                original_format: SubtitleFormatType::Sub,
            },
            format: SubtitleFormatType::Sub,
        };
        subtitle.entries.push(SubtitleEntry {
            index: 1,
            start_time: Duration::from_secs_f64(1.0),
            end_time: Duration::from_secs_f64(2.0),
            text: "X".into(),
            styling: None,
        });
        let fmt = SubFormat;
        let out = fmt.serialize(&subtitle).expect("serialize fps failed");
        // 1s * 50fps = 50 frames
//! MicroDVD/SubViewer SUB subtitle format implementation.
//!
//! This module provides parsing, serialization, and detection
//! for the MicroDVD/SubViewer SUB subtitle format, based on frame counts.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::formats::{SubtitleFormat, SubFormat};
//! let sub = SubFormat;
//! let content = "{0}{25}Hello\n";
//! let subtitle = sub.parse(content).unwrap();
//! ```
        assert!(out.contains("{50}{100}X"));
    }
}
