//! Styling adjustments for subtitle formats.
//!
//! This module provides functions to manipulate styling (e.g., fonts,
//! colors) for subtitle entries.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::formats::styling::apply_styling;
//! // ... apply styling adjustments to entries
//! ```

use regex::Regex;

use crate::core::formats::StylingInfo;
use crate::core::formats::converter::FormatConverter;

impl FormatConverter {
    /// Extract styling information from SRT tags
    pub(crate) fn extract_srt_styling(&self, text: &str) -> crate::Result<StylingInfo> {
        let mut styling = StylingInfo::default();
        if text.contains("<b>") || text.contains("<B>") {
            styling.bold = true;
        }
        if text.contains("<i>") || text.contains("<I>") {
            styling.italic = true;
        }
        if text.contains("<u>") || text.contains("<U>") {
            styling.underline = true;
        }
        if let Some(color) = self.extract_color_from_tags(text) {
            styling.color = Some(color);
        }
        Ok(styling)
    }

    /// Convert SRT tags to ASS tags
    pub(crate) fn convert_srt_tags_to_ass(&self, text: &str) -> String {
        let mut result = text.to_string();
        result = result.replace("<b>", "{\\b1}").replace("</b>", "{\\b0}");
        result = result.replace("<i>", "{\\i1}").replace("</i>", "{\\i0}");
        result = result.replace("<u>", "{\\u1}").replace("</u>", "{\\u0}");
        let color_regex = Regex::new(r#"<font color=\"([^\"]+)\">"#).unwrap();
        result = color_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let color = &caps[1];
                format!("{{\\c&H{}&}}", self.convert_color_to_ass(color))
            })
            .to_string();
        result = result.replace("</font>", "{\\c}");
        result
    }

    /// Remove ASS tags
    pub(crate) fn strip_ass_tags(&self, text: &str) -> String {
        let tag_regex = Regex::new(r"\{[^}]*\}").unwrap();
        tag_regex.replace_all(text, "").to_string()
    }

    /// Convert ASS tags to SRT tags
    pub(crate) fn convert_ass_tags_to_srt(&self, text: &str) -> String {
        let mut result = text.to_string();
        let bold_regex = Regex::new(r"\{\\b1\}([^\{]*)\{\\b0\}").unwrap();
        result = bold_regex.replace_all(&result, "<b>$1</b>").to_string();
        let italic_regex = Regex::new(r"\{\\i1\}([^\{]*)\{\\i0\}").unwrap();
        result = italic_regex.replace_all(&result, "<i>$1</i>").to_string();
        let underline_regex = Regex::new(r"\{\\u1\}([^\{]*)\{\\u0\}").unwrap();
        result = underline_regex
            .replace_all(&result, "<u>$1</u>")
            .to_string();
        result
    }

    /// Extract color from tags (simple implementation)
    pub(crate) fn extract_color_from_tags(&self, _text: &str) -> Option<String> {
        None
    }

    /// Convert color string to ASS color code
    pub(crate) fn convert_color_to_ass(&self, color: &str) -> String {
        color.trim_start_matches('#').to_string()
    }

    /// Convert SRT tags to VTT tags (simple implementation)
    pub(crate) fn convert_srt_tags_to_vtt(&self, text: &str) -> String {
        text.to_string()
    }
    /// Convert VTT tags to SRT tags (simple implementation)
    pub(crate) fn convert_vtt_tags_to_srt(&self, text: &str) -> String {
        // VTT uses HTML-like tags, SRT also supports basic tags, default preserve
        text.to_string()
    }
    /// Remove VTT tags (simple implementation)
    pub(crate) fn strip_vtt_tags(&self, text: &str) -> String {
        let tag_regex = Regex::new(r"</?[^>]+>").unwrap();
        tag_regex.replace_all(text, "").to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::core::formats::converter::{ConversionConfig, FormatConverter};

    fn make_converter() -> FormatConverter {
        FormatConverter::new(ConversionConfig {
            preserve_styling: true,
            target_encoding: "UTF-8".to_string(),
            keep_original: false,
            validate_output: false,
        })
    }

    // --- extract_srt_styling ---

    #[test]
    fn test_extract_srt_styling_empty() {
        let c = make_converter();
        let s = c.extract_srt_styling("").unwrap();
        assert!(!s.bold);
        assert!(!s.italic);
        assert!(!s.underline);
        assert!(s.color.is_none());
    }

    #[test]
    fn test_extract_srt_styling_bold_lowercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<b>Hello</b>").unwrap();
        assert!(s.bold);
        assert!(!s.italic);
        assert!(!s.underline);
    }

    #[test]
    fn test_extract_srt_styling_bold_uppercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<B>Hello</B>").unwrap();
        assert!(s.bold);
    }

    #[test]
    fn test_extract_srt_styling_italic_lowercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<i>Hello</i>").unwrap();
        assert!(s.italic);
        assert!(!s.bold);
        assert!(!s.underline);
    }

    #[test]
    fn test_extract_srt_styling_italic_uppercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<I>text</I>").unwrap();
        assert!(s.italic);
    }

    #[test]
    fn test_extract_srt_styling_underline_lowercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<u>text</u>").unwrap();
        assert!(s.underline);
        assert!(!s.bold);
        assert!(!s.italic);
    }

    #[test]
    fn test_extract_srt_styling_underline_uppercase() {
        let c = make_converter();
        let s = c.extract_srt_styling("<U>text</U>").unwrap();
        assert!(s.underline);
    }

    #[test]
    fn test_extract_srt_styling_all_tags() {
        let c = make_converter();
        let s = c
            .extract_srt_styling("<b><i><u>styled</u></i></b>")
            .unwrap();
        assert!(s.bold);
        assert!(s.italic);
        assert!(s.underline);
    }

    #[test]
    fn test_extract_srt_styling_mixed_case_independent() {
        let c = make_converter();
        let s = c.extract_srt_styling("<B>bold</B> <i>italic</i>").unwrap();
        assert!(s.bold);
        assert!(s.italic);
        assert!(!s.underline);
    }

    #[test]
    fn test_extract_srt_styling_no_tags() {
        let c = make_converter();
        let s = c.extract_srt_styling("plain text").unwrap();
        assert!(!s.bold);
        assert!(!s.italic);
        assert!(!s.underline);
        assert!(s.color.is_none());
    }

    // --- extract_color_from_tags ---

    #[test]
    fn test_extract_color_from_tags_always_none() {
        let c = make_converter();
        assert!(
            c.extract_color_from_tags(r##"<font color="#FF0000">text</font>"##)
                .is_none()
        );
        assert!(c.extract_color_from_tags("").is_none());
        assert!(c.extract_color_from_tags("anything").is_none());
    }

    // --- convert_color_to_ass ---

    #[test]
    fn test_convert_color_to_ass_strips_hash() {
        let c = make_converter();
        assert_eq!(c.convert_color_to_ass("#FF0000"), "FF0000");
    }

    #[test]
    fn test_convert_color_to_ass_no_hash() {
        let c = make_converter();
        assert_eq!(c.convert_color_to_ass("FF0000"), "FF0000");
    }

    #[test]
    fn test_convert_color_to_ass_empty() {
        let c = make_converter();
        assert_eq!(c.convert_color_to_ass(""), "");
    }

    #[test]
    fn test_convert_color_to_ass_named_color() {
        let c = make_converter();
        assert_eq!(c.convert_color_to_ass("red"), "red");
    }

    // --- convert_srt_tags_to_ass ---

    #[test]
    fn test_convert_srt_to_ass_bold() {
        let c = make_converter();
        assert_eq!(
            c.convert_srt_tags_to_ass("<b>Hello</b>"),
            "{\\b1}Hello{\\b0}"
        );
    }

    #[test]
    fn test_convert_srt_to_ass_italic() {
        let c = make_converter();
        assert_eq!(
            c.convert_srt_tags_to_ass("<i>Hello</i>"),
            "{\\i1}Hello{\\i0}"
        );
    }

    #[test]
    fn test_convert_srt_to_ass_underline() {
        let c = make_converter();
        assert_eq!(
            c.convert_srt_tags_to_ass("<u>Hello</u>"),
            "{\\u1}Hello{\\u0}"
        );
    }

    #[test]
    fn test_convert_srt_to_ass_font_color_with_hash() {
        let c = make_converter();
        let result = c.convert_srt_tags_to_ass(r##"<font color="#FF0000">red</font>"##);
        assert!(result.contains("FF0000"), "got: {result}");
        assert!(result.contains("{\\c}"), "got: {result}");
    }

    #[test]
    fn test_convert_srt_to_ass_font_color_without_hash() {
        let c = make_converter();
        let result = c.convert_srt_tags_to_ass(r##"<font color="AABBCC">text</font>"##);
        assert!(result.contains("AABBCC"), "got: {result}");
    }

    #[test]
    fn test_convert_srt_to_ass_empty() {
        let c = make_converter();
        assert_eq!(c.convert_srt_tags_to_ass(""), "");
    }

    #[test]
    fn test_convert_srt_to_ass_no_tags() {
        let c = make_converter();
        assert_eq!(c.convert_srt_tags_to_ass("plain"), "plain");
    }

    #[test]
    fn test_convert_srt_to_ass_combined() {
        let c = make_converter();
        let result = c.convert_srt_tags_to_ass("<b><i>bold italic</i></b>");
        assert!(result.contains("{\\b1}"), "got: {result}");
        assert!(result.contains("{\\b0}"), "got: {result}");
        assert!(result.contains("{\\i1}"), "got: {result}");
        assert!(result.contains("{\\i0}"), "got: {result}");
    }

    #[test]
    fn test_convert_srt_to_ass_font_close_tag() {
        let c = make_converter();
        let result = c.convert_srt_tags_to_ass("</font>");
        assert_eq!(result, "{\\c}");
    }

    // --- strip_ass_tags ---

    #[test]
    fn test_strip_ass_tags_basic() {
        let c = make_converter();
        assert_eq!(c.strip_ass_tags("{\\b1}Hello{\\b0}"), "Hello");
    }

    #[test]
    fn test_strip_ass_tags_multiple() {
        let c = make_converter();
        assert_eq!(
            c.strip_ass_tags("{\\i1}italic{\\i0} and {\\b1}bold{\\b0}"),
            "italic and bold"
        );
    }

    #[test]
    fn test_strip_ass_tags_empty() {
        let c = make_converter();
        assert_eq!(c.strip_ass_tags(""), "");
    }

    #[test]
    fn test_strip_ass_tags_no_tags() {
        let c = make_converter();
        assert_eq!(c.strip_ass_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_ass_tags_only_tag() {
        let c = make_converter();
        assert_eq!(c.strip_ass_tags("{\\pos(320,240)}"), "");
    }

    #[test]
    fn test_strip_ass_tags_color_tag() {
        let c = make_converter();
        let result = c.strip_ass_tags("{\\c&HFF0000&}colored text{\\c}");
        assert_eq!(result, "colored text");
    }

    // --- convert_ass_tags_to_srt ---

    #[test]
    fn test_convert_ass_to_srt_bold() {
        let c = make_converter();
        assert_eq!(
            c.convert_ass_tags_to_srt("{\\b1}Hello{\\b0}"),
            "<b>Hello</b>"
        );
    }

    #[test]
    fn test_convert_ass_to_srt_italic() {
        let c = make_converter();
        assert_eq!(
            c.convert_ass_tags_to_srt("{\\i1}Hello{\\i0}"),
            "<i>Hello</i>"
        );
    }

    #[test]
    fn test_convert_ass_to_srt_underline() {
        let c = make_converter();
        assert_eq!(
            c.convert_ass_tags_to_srt("{\\u1}Hello{\\u0}"),
            "<u>Hello</u>"
        );
    }

    #[test]
    fn test_convert_ass_to_srt_empty() {
        let c = make_converter();
        assert_eq!(c.convert_ass_tags_to_srt(""), "");
    }

    #[test]
    fn test_convert_ass_to_srt_no_tags() {
        let c = make_converter();
        assert_eq!(c.convert_ass_tags_to_srt("plain"), "plain");
    }

    #[test]
    fn test_convert_ass_to_srt_multiple() {
        let c = make_converter();
        let result = c.convert_ass_tags_to_srt("{\\b1}bold{\\b0} and {\\i1}italic{\\i0}");
        assert_eq!(result, "<b>bold</b> and <i>italic</i>");
    }

    // --- convert_srt_tags_to_vtt ---

    #[test]
    fn test_convert_srt_to_vtt_passthrough() {
        let c = make_converter();
        assert_eq!(c.convert_srt_tags_to_vtt("Hello"), "Hello");
        assert_eq!(c.convert_srt_tags_to_vtt("<b>bold</b>"), "<b>bold</b>");
        assert_eq!(c.convert_srt_tags_to_vtt(""), "");
    }

    // --- convert_vtt_tags_to_srt ---

    #[test]
    fn test_convert_vtt_to_srt_passthrough() {
        let c = make_converter();
        assert_eq!(c.convert_vtt_tags_to_srt("Hello"), "Hello");
        assert_eq!(c.convert_vtt_tags_to_srt("<b>bold</b>"), "<b>bold</b>");
        assert_eq!(c.convert_vtt_tags_to_srt(""), "");
    }

    // --- strip_vtt_tags ---

    #[test]
    fn test_strip_vtt_tags_basic() {
        let c = make_converter();
        assert_eq!(c.strip_vtt_tags("<b>Hello</b>"), "Hello");
    }

    #[test]
    fn test_strip_vtt_tags_multiple() {
        let c = make_converter();
        assert_eq!(
            c.strip_vtt_tags("<i>italic</i> and <b>bold</b>"),
            "italic and bold"
        );
    }

    #[test]
    fn test_strip_vtt_tags_empty() {
        let c = make_converter();
        assert_eq!(c.strip_vtt_tags(""), "");
    }

    #[test]
    fn test_strip_vtt_tags_no_tags() {
        let c = make_converter();
        assert_eq!(c.strip_vtt_tags("plain text"), "plain text");
    }

    #[test]
    fn test_strip_vtt_tags_nested() {
        let c = make_converter();
        assert_eq!(c.strip_vtt_tags("<b><i>nested</i></b>"), "nested");
    }

    #[test]
    fn test_strip_vtt_tags_with_attributes() {
        let c = make_converter();
        assert_eq!(
            c.strip_vtt_tags(r##"<font color="red">text</font>"##),
            "text"
        );
    }

    #[test]
    fn test_strip_vtt_tags_self_closing_like() {
        let c = make_converter();
        assert_eq!(c.strip_vtt_tags("<br>text"), "text");
    }

    // --- round-trip: SRT -> ASS -> strip ---

    #[test]
    fn test_roundtrip_srt_to_ass_strip() {
        let c = make_converter();
        let srt = "<b>Hello World</b>";
        let ass = c.convert_srt_tags_to_ass(srt);
        let stripped = c.strip_ass_tags(&ass);
        assert_eq!(stripped, "Hello World");
    }

    #[test]
    fn test_roundtrip_srt_to_ass_to_srt() {
        let c = make_converter();
        let original = "<b>text</b>";
        let ass = c.convert_srt_tags_to_ass(original);
        let back = c.convert_ass_tags_to_srt(&ass);
        assert_eq!(back, original);
    }

    #[test]
    fn test_roundtrip_italic_srt_ass_srt() {
        let c = make_converter();
        let original = "<i>text</i>";
        let ass = c.convert_srt_tags_to_ass(original);
        let back = c.convert_ass_tags_to_srt(&ass);
        assert_eq!(back, original);
    }

    #[test]
    fn test_roundtrip_underline_srt_ass_srt() {
        let c = make_converter();
        let original = "<u>text</u>";
        let ass = c.convert_srt_tags_to_ass(original);
        let back = c.convert_ass_tags_to_srt(&ass);
        assert_eq!(back, original);
    }
}
