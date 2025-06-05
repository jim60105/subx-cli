use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use crate::Result;
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

    const SAMPLE: &str = "1\n00:00:01,000 --> 00:00:03,500\nHello\nWorld\n";

    #[test]
    fn test_parse_and_serialize() {
        let fmt = SrtFormat;
        let subtitle = fmt.parse(SAMPLE).expect("解析失敗");
        assert_eq!(subtitle.entries.len(), 1);
        let serialized = fmt.serialize(&subtitle).expect("序列化失敗");
        assert!(serialized.contains("Hello\nWorld"));
    }

    #[test]
    fn test_detect_true_and_false() {
        let fmt = SrtFormat;
        assert!(fmt.detect(SAMPLE));
        assert!(!fmt.detect("Not a subtitle content"));
    }

    #[test]
    fn test_parse_multiple_entries_and_serialize_indices() {
        let multi =
            "1\n00:00:00,000 --> 00:00:01,000\nLine1\n\n2\n00:00:01,500 --> 00:00:02,000\nLine2\n";
        let fmt = SrtFormat;
        let subtitle = fmt.parse(multi).expect("解析多條目失敗");
        assert_eq!(subtitle.entries.len(), 2);
        assert_eq!(subtitle.entries[0].text, "Line1");
        assert_eq!(subtitle.entries[1].text, "Line2");
        let out = fmt.serialize(&subtitle).expect("序列化多條目失敗");
        // 檢查序列號及時間戳
        assert!(out.starts_with("1\n00:00:00,000 --> 00:00:01,000\nLine1"));
        assert!(out.contains("2\n00:00:01,500 --> 00:00:02,000\nLine2"));
    }
}
