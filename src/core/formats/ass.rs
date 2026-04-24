//! Advanced SubStation Alpha (ASS/SSA) subtitle format implementation.
//!
//! This module provides parsing, serialization, and detection capabilities
//! for the ASS/SSA subtitle format, including style and color definitions.
//!
//! # Examples
//!
//! ```rust,no_run
//! use subx_cli::core::formats::{SubtitleFormat, ass::AssFormat};
//! let ass = AssFormat;
//! let content = "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,Hello";
//! let subtitle = ass.parse(content).unwrap();
//! ```

use crate::Result;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use std::time::Duration;

/// ASS style definition for subtitle entries.
#[derive(Debug, Clone)]
pub struct AssStyle {
    /// Name identifier for this style
    pub name: String,
    /// Font family name to use for rendering
    pub font_name: String,
    /// Font size in points
    pub font_size: u32,
    /// Primary text color
    pub primary_color: Color,
    /// Secondary text color for styling effects
    pub secondary_color: Color,
    /// Outline border color
    pub outline_color: Color,
    /// Shadow color for text depth effect
    pub shadow_color: Color,
    /// Whether text should be rendered in bold
    pub bold: bool,
    /// Whether text should be rendered in italic
    pub italic: bool,
    /// Whether text should be underlined
    pub underline: bool,
    /// Text alignment value (1-9 for numpad positions)
    pub alignment: i32,
}

/// ASS color structure for style entries.
#[derive(Debug, Clone)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
}

impl Color {
    /// Creates a white color (RGB: 255, 255, 255).
    pub fn white() -> Self {
        Color {
            r: 255,
            g: 255,
            b: 255,
        }
    }

    /// Creates a black color (RGB: 0, 0, 0).
    pub fn black() -> Self {
        Color { r: 0, g: 0, b: 0 }
    }

    /// Creates a red color (RGB: 255, 0, 0).
    pub fn red() -> Self {
        Color { r: 255, g: 0, b: 0 }
    }
}

/// Subtitle format implementation for ASS/SSA.
///
/// The `AssFormat` struct implements parsing, serialization, and detection
/// for the ASS/SSA subtitle format.
pub struct AssFormat;

impl SubtitleFormat for AssFormat {
    fn parse(&self, content: &str) -> Result<Subtitle> {
        let mut entries = Vec::new();
        let mut in_events = false;
        let mut fields: Vec<&str> = Vec::new();
        for line in content.lines() {
            let l = line.trim_start();
            if l.eq_ignore_ascii_case("[events]") {
                in_events = true;
                continue;
            }
            if !in_events {
                continue;
            }
            if l.to_lowercase().starts_with("format:") {
                fields = l["Format:".len()..].split(',').map(|s| s.trim()).collect();
                continue;
            }
            if l.to_lowercase().starts_with("dialogue:") {
                let data = l["Dialogue:".len()..].trim();
                let parts: Vec<&str> = data.splitn(fields.len(), ',').collect();
                if parts.len() < fields.len() {
                    continue;
                }
                let start_index = fields
                    .iter()
                    .position(|&f| f.eq_ignore_ascii_case("start"))
                    .ok_or_else(|| {
                        SubXError::subtitle_format(
                            "ASS",
                            "Missing 'Start' field in Format declaration",
                        )
                    })?;
                let end_index = fields
                    .iter()
                    .position(|&f| f.eq_ignore_ascii_case("end"))
                    .ok_or_else(|| {
                        SubXError::subtitle_format(
                            "ASS",
                            "Missing 'End' field in Format declaration",
                        )
                    })?;
                let text_index = fields
                    .iter()
                    .position(|&f| f.eq_ignore_ascii_case("text"))
                    .ok_or_else(|| {
                        SubXError::subtitle_format(
                            "ASS",
                            "Missing 'Text' field in Format declaration",
                        )
                    })?;
                let start = parts[start_index].trim();
                let end = parts[end_index].trim();
                let text = parts[text_index..].join(",").replace("\\N", "\n");
                let start_time = parse_ass_time(start)?;
                let end_time = parse_ass_time(end)?;
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
                frame_rate: None,
                original_format: SubtitleFormatType::Ass,
            },
            format: SubtitleFormatType::Ass,
        })
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        let mut output = String::new();
        output.push_str("[Script Info]\n");
        output.push_str("; Script generated by SubX\n");
        output.push_str("ScriptType: v4.00+\n\n");
        output.push_str("[V4+ Styles]\n");
        output.push_str("Format: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\n");
        output.push_str("Style: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,10,10,10,1\n\n");
        output.push_str("[Events]\n");
        output.push_str("Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n");
        for entry in &subtitle.entries {
            let text = entry.text.replace('\n', "\\N");
            let start = format_ass_time(entry.start_time);
            let end = format_ass_time(entry.end_time);
            output.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0000,0000,0000,,{}\n",
                start, end, text
            ));
        }
        Ok(output)
    }

    fn detect(&self, content: &str) -> bool {
        content.contains("[Script Info]") || content.contains("Dialogue:")
    }

    fn format_name(&self) -> &'static str {
        "ASS"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["ass", "ssa"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ASS: &str = "[Script Info]\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,Hello\\NASS\n";

    #[test]
    fn test_detect_ass() {
        let fmt = AssFormat;
        assert!(fmt.detect(SAMPLE_ASS));
        assert!(!fmt.detect("Not ASS content"));
    }

    #[test]
    fn test_parse_ass_and_serialize() {
        let fmt = AssFormat;
        let subtitle = fmt.parse(SAMPLE_ASS).expect("ASS parse failed");
        assert_eq!(subtitle.entries.len(), 1);
        assert_eq!(subtitle.entries[0].text, "Hello\nASS");
        let out = fmt.serialize(&subtitle).expect("ASS serialize failed");
        assert!(out.contains("Dialogue: 0,0:00:01.00,0:00:02.50"));
        assert!(out.contains("Hello\\NASS"));
    }

    #[test]
    fn test_parse_ass_missing_start_field() {
        let fmt = AssFormat;
        // Format line omits the Start field.
        let content = "[Events]\nFormat: Layer,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:02.50,Default,,0000,0000,0000,,Hi\n";
        let err = fmt.parse(content).expect_err("missing Start must error");
        let msg = err.to_string();
        assert!(msg.contains("Start"), "unexpected error: {}", msg);
    }

    #[test]
    fn test_parse_ass_missing_end_field() {
        let fmt = AssFormat;
        let content = "[Events]\nFormat: Layer,Start,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,Default,,0000,0000,0000,,Hi\n";
        let err = fmt.parse(content).expect_err("missing End must error");
        assert!(err.to_string().contains("End"));
    }

    #[test]
    fn test_parse_ass_missing_text_field() {
        let fmt = AssFormat;
        let content = "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,\n";
        let err = fmt.parse(content).expect_err("missing Text must error");
        assert!(err.to_string().contains("Text"));
    }

    #[test]
    fn test_parse_ass_time_overflow() {
        // Hours value near u64::MAX to force checked_mul overflow.
        let overflow_time = format!("{}:00:00.00", u64::MAX);
        let err = parse_ass_time(&overflow_time).expect_err("overflow must be detected");
        assert!(err.to_string().to_lowercase().contains("overflow"));
    }

    #[test]
    fn test_parse_ass_time_valid() {
        let d = parse_ass_time("1:02:03.45").expect("valid time");
        assert_eq!(
            d,
            Duration::from_millis(3_600_000 + 2 * 60_000 + 3 * 1000 + 450)
        );
    }
}

fn parse_ass_time(time: &str) -> Result<Duration> {
    let parts: Vec<&str> = time.split(&[':', '.'][..]).collect();
    if parts.len() != 4 {
        return Err(SubXError::subtitle_format(
            "ASS",
            format!("Invalid time format: {}", time),
        ));
    }
    let hours: u64 = parts[0]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let minutes: u64 = parts[1]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let seconds: u64 = parts[2]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let centi: u64 = parts[3]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;

    let overflow =
        || SubXError::subtitle_format("ASS", format!("Timestamp arithmetic overflow: {}", time));
    let total_ms = hours
        .checked_mul(3_600_000)
        .ok_or_else(overflow)?
        .checked_add(minutes.checked_mul(60_000).ok_or_else(overflow)?)
        .ok_or_else(overflow)?
        .checked_add(seconds.checked_mul(1_000).ok_or_else(overflow)?)
        .ok_or_else(overflow)?
        .checked_add(centi.checked_mul(10).ok_or_else(overflow)?)
        .ok_or_else(overflow)?;
    Ok(Duration::from_millis(total_ms))
}

fn format_ass_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let centi = (total_ms % 1000) / 10;
    format!("{}:{:02}:{:02}.{:02}", hours, minutes, seconds, centi)
}
