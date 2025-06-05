use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use crate::Result;
use regex::Regex;
use std::time::Duration;

const DEFAULT_SUB_FPS: f32 = 25.0;

/// MicroDVD/SubViewer SUB 格式解析（暫未實作）
pub struct SubFormat;

impl SubtitleFormat for SubFormat {
    fn parse(&self, content: &str) -> Result<Subtitle> {
        let fps = DEFAULT_SUB_FPS;
        let re = Regex::new(r"^\{(\d+)\}\{(\d+)\}(.*)")
            .map_err(|e: regex::Error| SubXError::subtitle_format(self.format_name(), e.to_string()))?;
        let mut entries = Vec::new();
        for line in content.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if let Some(cap) = re.captures(l) {
                let start_frame: u64 = cap[1]
                    .parse()
                    .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format(self.format_name(), e.to_string()))?;
                let end_frame: u64 = cap[2]
                    .parse()
                    .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format(self.format_name(), e.to_string()))?;
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
}
