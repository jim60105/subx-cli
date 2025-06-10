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
