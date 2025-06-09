use crate::Result;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use regex::Regex;
use std::time::Duration;

/// WebVTT (.vtt) 格式解析（暫未實作）
pub struct VttFormat;

impl SubtitleFormat for VttFormat {
    fn parse(&self, content: &str) -> Result<Subtitle> {
        let time_re = Regex::new(
            r"(?m)^(\d{2}):(\d{2}):(\d{2})\.(\d{3}) --> (\d{2}):(\d{2}):(\d{2})\.(\d{3})",
        )
        .map_err(|e: regex::Error| SubXError::subtitle_format(self.format_name(), e.to_string()))?;
        let mut entries = Vec::new();
        for block in content.split("\n\n") {
            let block = block.trim();
            if block.is_empty()
                || block.starts_with("WEBVTT")
                || block.starts_with("NOTE")
                || block.starts_with("STYLE")
            {
                continue;
            }
            let lines: Vec<&str> = block.lines().collect();
            let mut idx = 0;
            if !time_re.is_match(lines[0]) {
                idx = 1;
                if idx >= lines.len() {
                    continue;
                }
            }
            if let Some(caps) = time_re.captures(lines[idx]) {
                let start = parse_vtt_time(&caps, 1)?;
                let end = parse_vtt_time(&caps, 5)?;
                let text = lines[(idx + 1)..].join("\n");
                entries.push(SubtitleEntry {
                    index: entries.len() + 1,
                    start_time: start,
                    end_time: end,
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
                frame_rate: None,
                original_format: SubtitleFormatType::Vtt,
            },
            format: SubtitleFormatType::Vtt,
        })
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        let mut output = String::new();
        output.push_str("WEBVTT\n\n");
        for entry in &subtitle.entries {
            output.push_str(&format!("{}\n", entry.index));
            output.push_str(&format_vtt_time_range(entry.start_time, entry.end_time));
            output.push_str(&format!("{}\n\n", entry.text));
        }
        Ok(output)
    }

    fn detect(&self, content: &str) -> bool {
        content.trim_start().starts_with("WEBVTT")
    }

    fn format_name(&self) -> &'static str {
        "VTT"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["vtt"]
    }
}

fn parse_vtt_time(caps: &regex::Captures, start: usize) -> Result<Duration> {
    let hours: u64 = caps[start]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let minutes: u64 = caps[start + 1]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let seconds: u64 = caps[start + 2]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let millis: u64 = caps[start + 3]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    Ok(Duration::from_millis(
        hours * 3600 * 1000 + minutes * 60 * 1000 + seconds * 1000 + millis,
    ))
}

fn format_vtt_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let millis = total_ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

fn format_vtt_time_range(start: Duration, end: Duration) -> String {
    format!("{} --> {}\n", format_vtt_time(start), format_vtt_time(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.500\nHello\nWorld\n";

    #[test]
    fn test_parse_and_serialize() {
        let fmt = VttFormat;
        let subtitle = fmt.parse(SAMPLE).expect("parse failed");
        assert_eq!(subtitle.entries.len(), 1);
        let out = fmt.serialize(&subtitle).expect("serialize failed");
        assert!(out.contains("00:00:01.000 --> 00:00:03.500"));
    }

    #[test]
    fn test_detect_and_skip_headers() {
        let fmt = VttFormat;
        // 有 WEBVTT 標頭
        assert!(fmt.detect("WEBVTT\nContent"));
        // 無標頭
        assert!(!fmt.detect("00:00:00.000 --> 00:00:01.000"));
    }

    #[test]
    fn test_parse_with_note_and_style() {
        let content = "WEBVTT\n\nNOTE this is note\nSTYLE body {color:red}\n\n1\n00:00:02.000 --> 00:00:03.000\nTest\n";
        let fmt = VttFormat;
        let subtitle = fmt.parse(content).expect("parse with NOTE/STYLE failed");
        assert_eq!(subtitle.entries.len(), 1);
        assert_eq!(subtitle.entries[0].text, "Test");
    }

    #[test]
    fn test_serialize_multiple_entries() {
        let mut subtitle = Subtitle {
            entries: Vec::new(),
            metadata: SubtitleMetadata {
                title: None,
                language: None,
                encoding: "utf-8".to_string(),
                frame_rate: None,
                original_format: SubtitleFormatType::Vtt,
            },
            format: SubtitleFormatType::Vtt,
        };
        subtitle.entries.push(SubtitleEntry {
            index: 1,
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "A".into(),
            styling: None,
        });
        subtitle.entries.push(SubtitleEntry {
            index: 2,
            start_time: Duration::from_secs(3),
            end_time: Duration::from_secs(4),
            text: "B".into(),
            styling: None,
        });
        let fmt = VttFormat;
        let out = fmt.serialize(&subtitle).expect("serialize multiple failed");
        assert!(out.contains("WEBVTT"));
        assert!(out.contains("1\n"));
        assert!(out.contains("2\n"));
    }
}
