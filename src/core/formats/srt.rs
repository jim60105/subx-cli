use crate::Result;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use regex::Regex;
use std::time::Duration;

/// SubRip (.srt) 格式解析與序列化
pub struct SrtFormat;

impl SubtitleFormat for SrtFormat {
    fn parse(&self, content: &str) -> Result<Subtitle> {
        let time_regex =
            Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3}) --> (\d{2}):(\d{2}):(\d{2}),(\d{3})")
                .map_err(|e| {
                    SubXError::subtitle_format(
                        self.format_name(),
                        format!("時間格式編譯錯誤: {}", e),
                    )
                })?;

        let mut entries = Vec::new();
        let blocks: Vec<&str> = content.split("\n\n").collect();

        for block in blocks {
            if block.trim().is_empty() {
                continue;
            }
            let lines: Vec<&str> = block.lines().collect();
            if lines.len() < 3 {
                continue;
            }

            let index: usize = lines[0].trim().parse().map_err(|e| {
                SubXError::subtitle_format(self.format_name(), format!("無效的序列號: {}", e))
            })?;

            if let Some(caps) = time_regex.captures(lines[1]) {
                let start_time = parse_time(&caps, 1)?;
                let end_time = parse_time(&caps, 5)?;
                let text = lines[2..].join("\n");

                entries.push(SubtitleEntry {
                    index,
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
                frame_rate: None,
                original_format: SubtitleFormatType::Srt,
            },
            format: SubtitleFormatType::Srt,
        })
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        let mut output = String::new();

        for (i, entry) in subtitle.entries.iter().enumerate() {
            output.push_str(&format!("{}\n", i + 1));
            output.push_str(&format_time_range(entry.start_time, entry.end_time));
            output.push_str(&format!("{}\n\n", entry.text));
        }

        Ok(output)
    }

    fn detect(&self, content: &str) -> bool {
        let time_pattern =
            Regex::new(r"\d{2}:\d{2}:\d{2},\d{3} --> \d{2}:\d{2}:\d{2},\d{3}").unwrap();
        time_pattern.is_match(content)
    }

    fn format_name(&self) -> &'static str {
        "SRT"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["srt"]
    }
}

fn parse_time(caps: &regex::Captures, start_group: usize) -> Result<Duration> {
    let hours: u64 = caps[start_group]
        .parse()
        .map_err(|e| SubXError::subtitle_format("SRT", format!("時間值解析失敗: {}", e)))?;
    let minutes: u64 = caps[start_group + 1]
        .parse()
        .map_err(|e| SubXError::subtitle_format("SRT", format!("時間值解析失敗: {}", e)))?;
    let seconds: u64 = caps[start_group + 2]
        .parse()
        .map_err(|e| SubXError::subtitle_format("SRT", format!("時間值解析失敗: {}", e)))?;
    let milliseconds: u64 = caps[start_group + 3]
        .parse()
        .map_err(|e| SubXError::subtitle_format("SRT", format!("時間值解析失敗: {}", e)))?;

    Ok(Duration::from_millis(
        hours * 3600000 + minutes * 60000 + seconds * 1000 + milliseconds,
    ))
}

fn format_time_range(start: Duration, end: Duration) -> String {
    format!("{} --> {}\n", format_duration(start), format_duration(end))
}

fn format_duration(duration: Duration) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::{SubtitleFormat, SubtitleFormatType};
    use std::time::Duration;

    const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:03,000\nHello, World!\n\n2\n00:00:05,000 --> 00:00:08,000\nThis is a test subtitle.\n多行測試\n\n";

    #[test]
    fn test_srt_parsing_basic() {
        let format = SrtFormat;
        let subtitle = format.parse(SAMPLE_SRT).unwrap();

        assert_eq!(subtitle.entries.len(), 2);
        assert_eq!(subtitle.format, SubtitleFormatType::Srt);

        let first = &subtitle.entries[0];
        assert_eq!(first.index, 1);
        assert_eq!(first.start_time, Duration::from_millis(1000));
        assert_eq!(first.end_time, Duration::from_millis(3000));
        assert_eq!(first.text, "Hello, World!");

        let second = &subtitle.entries[1];
        assert_eq!(second.index, 2);
        assert_eq!(second.start_time, Duration::from_millis(5000));
        assert_eq!(second.end_time, Duration::from_millis(8000));
        assert_eq!(second.text, "This is a test subtitle.\n多行測試");
    }

    #[test]
    fn test_srt_serialization_roundtrip() {
        let format = SrtFormat;
        let subtitle = format.parse(SAMPLE_SRT).unwrap();
        let serialized = format.serialize(&subtitle).unwrap();
        let reparsed = format.parse(&serialized).unwrap();
        assert_eq!(subtitle.entries.len(), reparsed.entries.len());
        for (o, r) in subtitle.entries.iter().zip(reparsed.entries.iter()) {
            assert_eq!(o.start_time, r.start_time);
            assert_eq!(o.end_time, r.end_time);
            assert_eq!(o.text, r.text);
        }
    }

    #[test]
    fn test_srt_detection() {
        let format = SrtFormat;
        assert!(format.detect(SAMPLE_SRT));
        assert!(!format.detect("This is not SRT content"));
        assert!(!format.detect("WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello"));
    }

    #[test]
    fn test_srt_invalid_format() {
        let format = SrtFormat;
        let invalid_time = "1\n00:00:01 --> 00:00:03\nText\n\n";
        let subtitle = format.parse(invalid_time).unwrap();
        assert_eq!(subtitle.entries.len(), 0);
        let invalid_index = "invalid\n00:00:01,000 --> 00:00:03,000\nText\n\n";
        assert!(format.parse(invalid_index).is_err());
    }

    #[test]
    fn test_srt_empty_and_malformed_blocks() {
        let format = SrtFormat;
        let subtitle = format.parse("").unwrap();
        assert_eq!(subtitle.entries.len(), 0);
        let subtitle = format.parse("\n\n\n").unwrap();
        assert_eq!(subtitle.entries.len(), 0);
        let malformed = "1\n00:00:01,000 --> 00:00:03,000\n\n";
        let subtitle = format.parse(malformed).unwrap();
        assert_eq!(subtitle.entries.len(), 0);
    }

    #[test]
    fn test_time_parsing_edge_cases() {
        let format = SrtFormat;
        let edge = "1\n23:59:59,999 --> 23:59:59,999\nEnd of day\n\n";
        let subtitle = format.parse(edge).unwrap();
        assert_eq!(subtitle.entries.len(), 1);
        let entry = &subtitle.entries[0];
        let expected = Duration::from_millis(23 * 3600000 + 59 * 60000 + 59 * 1000 + 999);
        assert_eq!(entry.start_time, expected);
        assert_eq!(entry.end_time, expected);
    }

    #[test]
    fn test_file_extensions_and_name() {
        let format = SrtFormat;
        assert_eq!(format.file_extensions(), &["srt"]);
        assert_eq!(format.format_name(), "SRT");
    }
}
