//! Subtitle format manager that detects and dispatches to the appropriate parser.
//!
//! This module provides the `FormatManager`, which automatically detects
//! subtitle formats and selects the correct parser for loading and saving.
//!
//! # Examples
//!
//! ```rust,no_run
//! use subx_cli::core::formats::manager::FormatManager;
//! let manager = FormatManager::new();
//! let content = "1\n00:00:01,000 --> 00:00:02,000\nHello world\n";
//! let subtitle = manager.parse_auto(content).unwrap();
//! ```

use crate::core::formats::{Subtitle, SubtitleFormat};
use log::{info, warn};

/// Manager for subtitle format detection and parser dispatch.
///
/// The `FormatManager` handles format inference based on file contents
/// or extensions and routes parsing and serialization requests accordingly.
pub struct FormatManager {
    formats: Vec<Box<dyn SubtitleFormat>>,
}

impl Default for FormatManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatManager {
    /// Create manager and register all formats
    pub fn new() -> Self {
        Self {
            formats: vec![
                Box::new(crate::core::formats::ass::AssFormat),
                Box::new(crate::core::formats::vtt::VttFormat),
                Box::new(crate::core::formats::srt::SrtFormat),
                Box::new(crate::core::formats::sub::SubFormat),
            ],
        }
    }

    /// Auto-detect format and parse
    pub fn parse_auto(&self, content: &str) -> crate::Result<Subtitle> {
        for fmt in &self.formats {
            if fmt.detect(content) {
                return fmt.parse(content);
            }
        }
        Err(crate::error::SubXError::subtitle_format(
            "Unknown",
            "Unknown subtitle format",
        ))
    }

    /// Get parser by format name
    pub fn get_format(&self, name: &str) -> Option<&dyn SubtitleFormat> {
        let lname = name.to_lowercase();
        self.formats
            .iter()
            .find(|f| f.format_name().to_lowercase() == lname)
            .map(|f| f.as_ref())
    }

    /// Get parser by file extension
    pub fn get_format_by_extension(&self, ext: &str) -> Option<&dyn SubtitleFormat> {
        let ext_lc = ext.to_lowercase();
        self.formats
            .iter()
            .find(|f| f.file_extensions().contains(&ext_lc.as_str()))
            .map(|f| f.as_ref())
    }

    /// Read subtitle and auto-detect encoding, convert to UTF-8
    pub fn read_subtitle_with_encoding_detection(&self, file_path: &str) -> crate::Result<String> {
        let detector = crate::core::formats::encoding::EncodingDetector::with_defaults();
        let info = detector.detect_file_encoding(file_path)?;
        let converter = crate::core::formats::encoding::EncodingConverter::new();
        let result = converter.convert_file_to_utf8(file_path, &info)?;
        let validation = converter.validate_conversion(&result);
        if !validation.is_valid {
            warn!("Encoding conversion warnings: {:?}", validation.warnings);
        }
        info!(
            "Detected encoding: {:?} (confidence: {:.2})",
            info.charset, info.confidence
        );
        Ok(result.converted_text)
    }

    /// Get file encoding information
    pub fn get_encoding_info(
        &self,
        file_path: &str,
    ) -> crate::Result<crate::core::formats::encoding::EncodingInfo> {
        let detector = crate::core::formats::encoding::EncodingDetector::with_defaults();
        detector.detect_file_encoding(file_path)
    }

    /// Load subtitle from file with encoding detection and parsing
    pub fn load_subtitle(&self, file_path: &std::path::Path) -> crate::Result<Subtitle> {
        let content =
            self.read_subtitle_with_encoding_detection(file_path.to_str().ok_or_else(|| {
                crate::error::SubXError::subtitle_format("", "Invalid file path encoding")
            })?)?;
        self.parse_auto(&content)
    }

    /// Save subtitle to file in the same format as extension
    pub fn save_subtitle(
        &self,
        subtitle: &Subtitle,
        file_path: &std::path::Path,
    ) -> crate::Result<()> {
        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let fmt = self.get_format_by_extension(ext).ok_or_else(|| {
            crate::error::SubXError::subtitle_format(ext, "Unsupported subtitle format for saving")
        })?;
        let out = fmt.serialize(subtitle)?;
        std::fs::write(file_path, out)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::formats::SubtitleFormatType;
    use std::time::Duration;

    const SAMPLE_SRT: &str = "1\n00:00:00,000 --> 00:00:01,000\nOne\n";
    const SAMPLE_VTT: &str = "WEBVTT\n\n1\n00:00:00.000 --> 00:00:01.000\nOne\n";
    const SAMPLE_WEBVTT_THREE_LINES: &str = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.000\n第一句字幕內容\n\n2\n00:00:04.000 --> 00:00:06.000\n第二句字幕內容\n\n3\n00:00:07.000 --> 00:00:09.000\n第三句字幕內容\n";

    const COMPLEX_WEBVTT: &str = "WEBVTT\n\nNOTE 這是註解，應該被忽略\n\nSTYLE\n::cue {\n  background-color: black;\n  color: white;\n}\n\n1\n00:00:01.000 --> 00:00:03.500\n第一句字幕內容\n包含多行文字\n\n2\n00:00:04.200 --> 00:00:07.800\n第二句字幕內容\n\n3\n00:00:08.000 --> 00:00:10.000\n第三句字幕內容\n";

    #[test]
    fn test_get_format_by_name_and_extension() {
        let mgr = FormatManager::new();
        let srt = mgr.get_format("srt").expect("get_format srt");
        assert_eq!(srt.format_name(), "SRT");
        let vtt = mgr
            .get_format_by_extension("vtt")
            .expect("get_format_by_extension vtt");
        assert_eq!(vtt.format_name(), "VTT");
    }

    #[test]
    fn test_parse_auto_supported_and_error() {
        let mgr = FormatManager::new();
        let sub = mgr.parse_auto(SAMPLE_SRT).expect("parse_auto srt");
        assert_eq!(sub.format, SubtitleFormatType::Srt);
        let subv = mgr.parse_auto(SAMPLE_VTT).expect("parse_auto vtt");
        assert_eq!(subv.format, SubtitleFormatType::Vtt);
        let err = mgr.parse_auto("no format");
        assert!(err.is_err());
    }

    #[test]
    fn test_webvtt_parse_auto_first_subtitle_content() {
        let mgr = FormatManager::new();

        let subtitle = mgr
            .parse_auto(SAMPLE_WEBVTT_THREE_LINES)
            .expect("Failed to parse WEBVTT format using parse_auto");

        // Verify auto-detection as WEBVTT format
        assert_eq!(
            subtitle.format,
            SubtitleFormatType::Vtt,
            "Auto detection should identify as WEBVTT format"
        );

        // Verify 3 subtitles were parsed
        assert_eq!(
            subtitle.entries.len(),
            3,
            "Should parse exactly 3 subtitle entries"
        );

        // Verify first subtitle content, index and timeline
        let first = &subtitle.entries[0];
        assert_eq!(
            first.text, "第一句字幕內容",
            "First subtitle content should be correctly parsed"
        );
        assert_eq!(first.index, 1, "First subtitle should have index 1");
        assert_eq!(
            first.start_time,
            Duration::from_millis(1000),
            "First subtitle start time should be 1 second"
        );
        assert_eq!(
            first.end_time,
            Duration::from_millis(3000),
            "First subtitle end time should be 3 seconds"
        );

        // Verify other subtitle content
        assert_eq!(subtitle.entries[1].text, "第二句字幕內容");
        assert_eq!(subtitle.entries[2].text, "第三句字幕內容");
    }

    #[test]
    fn test_webvtt_parse_auto_with_complex_content() {
        let mgr = FormatManager::new();
        let subtitle = mgr
            .parse_auto(COMPLEX_WEBVTT)
            .expect("Failed to parse complex WEBVTT");

        // Verify auto-detection as WEBVTT format and parse three subtitles (ignore NOTE and STYLE)
        assert_eq!(subtitle.format, SubtitleFormatType::Vtt);
        assert_eq!(subtitle.entries.len(), 3);

        // Verify first subtitle contains multi-line text and correct time parsing
        let first = &subtitle.entries[0];
        assert_eq!(first.text, "第一句字幕內容\n包含多行文字");
        assert_eq!(first.start_time, Duration::from_millis(1000));
        assert_eq!(first.end_time, Duration::from_millis(3500));
    }
}
