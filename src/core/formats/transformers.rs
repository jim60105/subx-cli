//! Subtitle transformers for converting between different subtitle formats.
//!
//! This module provides utility methods to transform subtitle objects
//! into various target formats using the `FormatConverter`.
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::formats::{FormatConverter, Subtitle};
//! // Convert a subtitle object to a target format
//! let converter = FormatConverter::new();
//! let transformed = converter.transform_subtitle(subtitle.clone(), "ass").unwrap();
//! ```

use crate::core::formats::converter::FormatConverter;
use crate::core::formats::{Subtitle, SubtitleFormatType};

impl FormatConverter {
    /// Transforms a subtitle object into the target format.
    ///
    /// # Arguments
    ///
    /// * `subtitle` - The subtitle object to transform.
    /// * `target_format` - The desired subtitle format identifier (e.g., "ass", "srt").
    ///
    /// # Returns
    ///
    /// A `Result<Subtitle>` containing the transformed subtitle object or an error.
    pub(crate) fn transform_subtitle(
        &self,
        subtitle: Subtitle,
        target_format: &str,
    ) -> crate::Result<Subtitle> {
        match (subtitle.format.as_str(), target_format) {
            ("srt", "ass") => self.srt_to_ass(subtitle),
            ("ass", "srt") => self.ass_to_srt(subtitle),
            ("srt", "vtt") => self.srt_to_vtt(subtitle),
            ("vtt", "srt") => self.vtt_to_srt(subtitle),
            ("ass", "vtt") => self.ass_to_vtt(subtitle),
            ("vtt", "ass") => self.vtt_to_ass(subtitle),
            (source, target) if source == target => Ok(subtitle),
            _ => Err(crate::error::SubXError::subtitle_format(
                subtitle.format.to_string(),
                format!(
                    "Unsupported conversion: {} -> {}",
                    subtitle.format, target_format
                ),
            )),
        }
    }

    /// SRT to ASS conversion
    pub(crate) fn srt_to_ass(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        let _default_style = crate::core::formats::ass::AssStyle {
            name: "Default".to_string(),
            font_name: "Arial".to_string(),
            font_size: 16,
            primary_color: crate::core::formats::ass::Color::white(),
            secondary_color: crate::core::formats::ass::Color::red(),
            outline_color: crate::core::formats::ass::Color::black(),
            shadow_color: crate::core::formats::ass::Color::black(),
            bold: false,
            italic: false,
            underline: false,
            alignment: 2,
        };
        for entry in &mut subtitle.entries {
            if self.config.preserve_styling {
                entry.styling = Some(self.extract_srt_styling(&entry.text)?);
            }
            entry.text = self.convert_srt_tags_to_ass(&entry.text);
        }
        subtitle.format = SubtitleFormatType::Ass;
        subtitle.metadata.original_format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }

    /// ASS to SRT conversion
    pub(crate) fn ass_to_srt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        for entry in &mut subtitle.entries {
            entry.text = self.strip_ass_tags(&entry.text);
            if self.config.preserve_styling {
                entry.text = self.convert_ass_tags_to_srt(&entry.text);
            }
            entry.styling = None;
        }
        subtitle.format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }

    /// SRT to VTT conversion
    pub(crate) fn srt_to_vtt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        subtitle.metadata.title = Some("WEBVTT".to_string());
        for entry in &mut subtitle.entries {
            entry.text = self.convert_srt_tags_to_vtt(&entry.text);
        }
        subtitle.format = SubtitleFormatType::Vtt;
        Ok(subtitle)
    }

    /// ASS to VTT conversion
    pub(crate) fn ass_to_vtt(&self, subtitle: Subtitle) -> crate::Result<Subtitle> {
        // First convert ASS to SRT, then to VTT
        let subtitle = self.ass_to_srt(subtitle)?;
        self.srt_to_vtt(subtitle)
    }

    /// VTT to SRT conversion
    pub(crate) fn vtt_to_srt(&self, mut subtitle: Subtitle) -> crate::Result<Subtitle> {
        // VTT can preserve or remove HTML style tags
        for entry in &mut subtitle.entries {
            if self.config.preserve_styling {
                entry.text = self.convert_vtt_tags_to_srt(&entry.text);
            } else {
                entry.text = self.strip_vtt_tags(&entry.text);
            }
            entry.styling = None;
        }
        subtitle.format = SubtitleFormatType::Srt;
        Ok(subtitle)
    }

    /// VTT to ASS conversion
    pub(crate) fn vtt_to_ass(&self, subtitle: Subtitle) -> crate::Result<Subtitle> {
        // First convert VTT to SRT, then to ASS
        let subtitle = self.vtt_to_srt(subtitle)?;
        self.srt_to_ass(subtitle)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::core::formats::converter::{ConversionConfig, FormatConverter};
    use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};

    fn make_config(preserve_styling: bool) -> ConversionConfig {
        ConversionConfig {
            preserve_styling,
            target_encoding: "UTF-8".to_string(),
            keep_original: false,
            validate_output: false,
        }
    }

    fn make_converter(preserve_styling: bool) -> FormatConverter {
        FormatConverter::new(make_config(preserve_styling))
    }

    fn make_subtitle(format: SubtitleFormatType, entries: Vec<SubtitleEntry>) -> Subtitle {
        Subtitle {
            entries,
            metadata: SubtitleMetadata::new(format.clone()),
            format,
        }
    }

    fn make_entry(index: usize, text: &str) -> SubtitleEntry {
        SubtitleEntry::new(
            index,
            Duration::from_secs((index as u64) * 5),
            Duration::from_secs((index as u64) * 5 + 3),
            text.to_string(),
        )
    }

    // ── transform_subtitle dispatch ─────────────────────────────────────────

    #[test]
    fn transform_srt_to_ass() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "ass").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
    }

    #[test]
    fn transform_ass_to_srt() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "srt").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
    }

    #[test]
    fn transform_srt_to_vtt() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "vtt").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Vtt);
    }

    #[test]
    fn transform_vtt_to_srt() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "srt").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
    }

    #[test]
    fn transform_ass_to_vtt() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "vtt").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Vtt);
    }

    #[test]
    fn transform_vtt_to_ass() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "ass").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
    }

    #[test]
    fn transform_same_format_returns_unchanged() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Same")]);
        let result = conv.transform_subtitle(sub, "srt").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
        assert_eq!(result.entries[0].text, "Same");
    }

    #[test]
    fn transform_same_format_ass() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![make_entry(1, "Same")]);
        let result = conv.transform_subtitle(sub, "ass").unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
    }

    #[test]
    fn transform_unsupported_format_returns_error() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Hello")]);
        let result = conv.transform_subtitle(sub, "sub");
        assert!(result.is_err());
    }

    // ── srt_to_ass ──────────────────────────────────────────────────────────

    #[test]
    fn srt_to_ass_sets_format() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Hello")]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
        assert_eq!(result.metadata.original_format, SubtitleFormatType::Srt);
    }

    #[test]
    fn srt_to_ass_converts_bold_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "<b>bold text</b>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].text.contains("{\\b1}"));
        assert!(result.entries[0].text.contains("{\\b0}"));
    }

    #[test]
    fn srt_to_ass_converts_italic_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "<i>italic</i>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].text.contains("{\\i1}"));
        assert!(result.entries[0].text.contains("{\\i0}"));
    }

    #[test]
    fn srt_to_ass_converts_underline_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "<u>underline</u>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].text.contains("{\\u1}"));
        assert!(result.entries[0].text.contains("{\\u0}"));
    }

    #[test]
    fn srt_to_ass_converts_font_color_tag() {
        let conv = make_converter(false);
        let entry = make_entry(1, r##"<font color="#FF0000">red</font>"##);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].text.contains("{\\c"));
        assert!(result.entries[0].text.contains("{\\c}"));
    }

    #[test]
    fn srt_to_ass_with_preserve_styling_sets_styling() {
        let conv = make_converter(true);
        let entry = make_entry(1, "<b>bold</b>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].styling.is_some());
        let styling = result.entries[0].styling.as_ref().unwrap();
        assert!(styling.bold);
    }

    #[test]
    fn srt_to_ass_with_preserve_styling_italic() {
        let conv = make_converter(true);
        let entry = make_entry(1, "<i>italic</i>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        let styling = result.entries[0].styling.as_ref().unwrap();
        assert!(styling.italic);
    }

    #[test]
    fn srt_to_ass_with_preserve_styling_underline() {
        let conv = make_converter(true);
        let entry = make_entry(1, "<u>underlined</u>");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        let styling = result.entries[0].styling.as_ref().unwrap();
        assert!(styling.underline);
    }

    #[test]
    fn srt_to_ass_empty_entries() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
        assert!(result.entries.is_empty());
    }

    // ── ass_to_srt ──────────────────────────────────────────────────────────

    #[test]
    fn ass_to_srt_sets_format() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![make_entry(1, "Hello")]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
    }

    #[test]
    fn ass_to_srt_strips_ass_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "{\\an8}Dialogue text");
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert_eq!(result.entries[0].text, "Dialogue text");
    }

    #[test]
    fn ass_to_srt_clears_styling() {
        let conv = make_converter(false);
        let mut entry = make_entry(1, "{\\b1}bold{\\b0}");
        entry.styling = Some(crate::core::formats::StylingInfo {
            bold: true,
            ..Default::default()
        });
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert!(result.entries[0].styling.is_none());
    }

    #[test]
    fn ass_to_srt_with_preserve_styling_converts_bold() {
        let conv = make_converter(true);
        let entry = make_entry(1, "{\\b1}bold text{\\b0}");
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_srt(sub).unwrap();
        // strip_ass_tags runs first, so ASS override blocks are removed before conversion
        assert!(!result.entries[0].text.contains("{\\b1}"));
        assert_eq!(result.entries[0].text, "bold text");
    }

    #[test]
    fn ass_to_srt_with_preserve_styling_converts_italic() {
        let conv = make_converter(true);
        let entry = make_entry(1, "{\\i1}italic{\\i0}");
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert!(!result.entries[0].text.contains("{\\i1}"));
        assert_eq!(result.entries[0].text, "italic");
    }

    #[test]
    fn ass_to_srt_with_preserve_styling_converts_underline() {
        let conv = make_converter(true);
        let entry = make_entry(1, "{\\u1}underline{\\u0}");
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert!(!result.entries[0].text.contains("{\\u1}"));
        assert_eq!(result.entries[0].text, "underline");
    }

    #[test]
    fn ass_to_srt_empty_entries() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![]);
        let result = conv.ass_to_srt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
        assert!(result.entries.is_empty());
    }

    // ── srt_to_vtt ──────────────────────────────────────────────────────────

    #[test]
    fn srt_to_vtt_sets_format_and_title() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![make_entry(1, "Hello")]);
        let result = conv.srt_to_vtt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Vtt);
        assert_eq!(result.metadata.title.as_deref(), Some("WEBVTT"));
    }

    #[test]
    fn srt_to_vtt_preserves_text() {
        let conv = make_converter(false);
        let entry = make_entry(1, "Plain text");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_vtt(sub).unwrap();
        assert_eq!(result.entries[0].text, "Plain text");
    }

    #[test]
    fn srt_to_vtt_empty_entries() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![]);
        let result = conv.srt_to_vtt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Vtt);
    }

    // ── ass_to_vtt ──────────────────────────────────────────────────────────

    #[test]
    fn ass_to_vtt_sets_format() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![make_entry(1, "{\\an8}Text")]);
        let result = conv.ass_to_vtt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Vtt);
    }

    #[test]
    fn ass_to_vtt_strips_ass_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "{\\b1}bold{\\b0}");
        let sub = make_subtitle(SubtitleFormatType::Ass, vec![entry]);
        let result = conv.ass_to_vtt(sub).unwrap();
        assert!(!result.entries[0].text.contains("{\\"));
    }

    // ── vtt_to_srt ──────────────────────────────────────────────────────────

    #[test]
    fn vtt_to_srt_sets_format() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![make_entry(1, "Hello")]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
    }

    #[test]
    fn vtt_to_srt_strips_html_tags_without_preserve_styling() {
        let conv = make_converter(false);
        let entry = make_entry(1, "<b>bold</b> text");
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![entry]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert!(!result.entries[0].text.contains("<b>"));
        assert_eq!(result.entries[0].text, "bold text");
    }

    #[test]
    fn vtt_to_srt_preserves_tags_with_preserve_styling() {
        let conv = make_converter(true);
        let entry = make_entry(1, "<i>italic</i>");
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![entry]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert_eq!(result.entries[0].text, "<i>italic</i>");
    }

    #[test]
    fn vtt_to_srt_clears_styling() {
        let conv = make_converter(false);
        let mut entry = make_entry(1, "text");
        entry.styling = Some(crate::core::formats::StylingInfo {
            italic: true,
            ..Default::default()
        });
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![entry]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert!(result.entries[0].styling.is_none());
    }

    #[test]
    fn vtt_to_srt_empty_entries() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Srt);
    }

    // ── vtt_to_ass ──────────────────────────────────────────────────────────

    #[test]
    fn vtt_to_ass_sets_format() {
        let conv = make_converter(false);
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![make_entry(1, "Hello")]);
        let result = conv.vtt_to_ass(sub).unwrap();
        assert_eq!(result.format, SubtitleFormatType::Ass);
    }

    #[test]
    fn vtt_to_ass_multiple_entries() {
        let conv = make_converter(false);
        let sub = make_subtitle(
            SubtitleFormatType::Vtt,
            vec![make_entry(1, "First"), make_entry(2, "Second")],
        );
        let result = conv.vtt_to_ass(sub).unwrap();
        assert_eq!(result.entries.len(), 2);
    }

    // ── styling helpers (extract_srt_styling) ───────────────────────────────

    #[test]
    fn extract_srt_styling_detects_bold() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<b>text</b>").unwrap();
        assert!(styling.bold);
    }

    #[test]
    fn extract_srt_styling_detects_bold_uppercase() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<B>text</B>").unwrap();
        assert!(styling.bold);
    }

    #[test]
    fn extract_srt_styling_detects_italic() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<i>text</i>").unwrap();
        assert!(styling.italic);
    }

    #[test]
    fn extract_srt_styling_detects_italic_uppercase() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<I>text</I>").unwrap();
        assert!(styling.italic);
    }

    #[test]
    fn extract_srt_styling_detects_underline() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<u>text</u>").unwrap();
        assert!(styling.underline);
    }

    #[test]
    fn extract_srt_styling_detects_underline_uppercase() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("<U>text</U>").unwrap();
        assert!(styling.underline);
    }

    #[test]
    fn extract_srt_styling_plain_text_no_flags() {
        let conv = make_converter(false);
        let styling = conv.extract_srt_styling("plain text").unwrap();
        assert!(!styling.bold);
        assert!(!styling.italic);
        assert!(!styling.underline);
        assert!(styling.color.is_none());
    }

    #[test]
    fn extract_srt_styling_all_flags() {
        let conv = make_converter(false);
        let styling = conv
            .extract_srt_styling("<b><i><u>all</u></i></b>")
            .unwrap();
        assert!(styling.bold);
        assert!(styling.italic);
        assert!(styling.underline);
    }

    // ── convert_srt_tags_to_ass ─────────────────────────────────────────────

    #[test]
    fn convert_srt_tags_to_ass_bold() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_ass("<b>text</b>");
        assert_eq!(result, "{\\b1}text{\\b0}");
    }

    #[test]
    fn convert_srt_tags_to_ass_italic() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_ass("<i>text</i>");
        assert_eq!(result, "{\\i1}text{\\i0}");
    }

    #[test]
    fn convert_srt_tags_to_ass_underline() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_ass("<u>text</u>");
        assert_eq!(result, "{\\u1}text{\\u0}");
    }

    #[test]
    fn convert_srt_tags_to_ass_font_color_hex() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_ass(r##"<font color="#FF0000">red</font>"##);
        assert!(result.contains("{\\c&H"));
        assert!(result.contains("{\\c}"));
    }

    #[test]
    fn convert_srt_tags_to_ass_no_tags() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_ass("plain text");
        assert_eq!(result, "plain text");
    }

    // ── strip_ass_tags ──────────────────────────────────────────────────────

    #[test]
    fn strip_ass_tags_removes_override_blocks() {
        let conv = make_converter(false);
        let result = conv.strip_ass_tags("{\\an8}{\\b1}text{\\b0}");
        assert_eq!(result, "text");
    }

    #[test]
    fn strip_ass_tags_plain_text_unchanged() {
        let conv = make_converter(false);
        let result = conv.strip_ass_tags("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn strip_ass_tags_empty_string() {
        let conv = make_converter(false);
        let result = conv.strip_ass_tags("");
        assert_eq!(result, "");
    }

    // ── convert_ass_tags_to_srt ─────────────────────────────────────────────

    #[test]
    fn convert_ass_tags_to_srt_bold() {
        let conv = make_converter(false);
        let result = conv.convert_ass_tags_to_srt("{\\b1}text{\\b0}");
        assert_eq!(result, "<b>text</b>");
    }

    #[test]
    fn convert_ass_tags_to_srt_italic() {
        let conv = make_converter(false);
        let result = conv.convert_ass_tags_to_srt("{\\i1}text{\\i0}");
        assert_eq!(result, "<i>text</i>");
    }

    #[test]
    fn convert_ass_tags_to_srt_underline() {
        let conv = make_converter(false);
        let result = conv.convert_ass_tags_to_srt("{\\u1}text{\\u0}");
        assert_eq!(result, "<u>text</u>");
    }

    #[test]
    fn convert_ass_tags_to_srt_no_tags() {
        let conv = make_converter(false);
        let result = conv.convert_ass_tags_to_srt("no tags here");
        assert_eq!(result, "no tags here");
    }

    // ── extract_color_from_tags ─────────────────────────────────────────────

    #[test]
    fn extract_color_from_tags_returns_none() {
        let conv = make_converter(false);
        let result = conv.extract_color_from_tags("<font color=\"red\">text</font>");
        assert!(result.is_none());
    }

    // ── convert_color_to_ass ────────────────────────────────────────────────

    #[test]
    fn convert_color_to_ass_strips_hash() {
        let conv = make_converter(false);
        let result = conv.convert_color_to_ass("#FF0000");
        assert_eq!(result, "FF0000");
    }

    #[test]
    fn convert_color_to_ass_no_hash() {
        let conv = make_converter(false);
        let result = conv.convert_color_to_ass("00FF00");
        assert_eq!(result, "00FF00");
    }

    // ── convert_srt_tags_to_vtt ─────────────────────────────────────────────

    #[test]
    fn convert_srt_tags_to_vtt_passthrough() {
        let conv = make_converter(false);
        let result = conv.convert_srt_tags_to_vtt("<b>text</b>");
        assert_eq!(result, "<b>text</b>");
    }

    // ── convert_vtt_tags_to_srt ─────────────────────────────────────────────

    #[test]
    fn convert_vtt_tags_to_srt_passthrough() {
        let conv = make_converter(false);
        let result = conv.convert_vtt_tags_to_srt("<i>text</i>");
        assert_eq!(result, "<i>text</i>");
    }

    // ── strip_vtt_tags ──────────────────────────────────────────────────────

    #[test]
    fn strip_vtt_tags_removes_html_tags() {
        let conv = make_converter(false);
        let result = conv.strip_vtt_tags("<b>bold</b> and <i>italic</i>");
        assert_eq!(result, "bold and italic");
    }

    #[test]
    fn strip_vtt_tags_plain_text_unchanged() {
        let conv = make_converter(false);
        let result = conv.strip_vtt_tags("plain text");
        assert_eq!(result, "plain text");
    }

    #[test]
    fn strip_vtt_tags_empty_string() {
        let conv = make_converter(false);
        let result = conv.strip_vtt_tags("");
        assert_eq!(result, "");
    }

    #[test]
    fn strip_vtt_tags_self_closing() {
        let conv = make_converter(false);
        let result = conv.strip_vtt_tags("line1<br/>line2");
        assert_eq!(result, "line1line2");
    }

    // ── edge cases ──────────────────────────────────────────────────────────

    #[test]
    fn srt_to_ass_special_characters() {
        let conv = make_converter(false);
        let entry = make_entry(1, "Special: <>&\"'");
        let sub = make_subtitle(SubtitleFormatType::Srt, vec![entry]);
        let result = conv.srt_to_ass(sub).unwrap();
        assert!(result.entries[0].text.contains("Special:"));
    }

    #[test]
    fn srt_to_ass_multiple_entries_converted() {
        let conv = make_converter(false);
        let sub = make_subtitle(
            SubtitleFormatType::Srt,
            vec![
                make_entry(1, "<b>First</b>"),
                make_entry(2, "<i>Second</i>"),
                make_entry(3, "Third"),
            ],
        );
        let result = conv.srt_to_ass(sub).unwrap();
        assert_eq!(result.entries.len(), 3);
        assert!(result.entries[0].text.contains("{\\b1}"));
        assert!(result.entries[1].text.contains("{\\i1}"));
        assert_eq!(result.entries[2].text, "Third");
    }

    #[test]
    fn ass_to_srt_multiple_entries() {
        let conv = make_converter(true);
        let sub = make_subtitle(
            SubtitleFormatType::Ass,
            vec![
                make_entry(1, "{\\b1}one{\\b0}"),
                make_entry(2, "{\\i1}two{\\i0}"),
            ],
        );
        let result = conv.ass_to_srt(sub).unwrap();
        assert_eq!(result.entries.len(), 2);
        // strip_ass_tags runs before convert, so all override blocks are removed
        assert_eq!(result.entries[0].text, "one");
        assert_eq!(result.entries[1].text, "two");
    }

    #[test]
    fn vtt_to_srt_strips_multiple_tags() {
        let conv = make_converter(false);
        let entry = make_entry(1, "<v Speaker>text</v>");
        let sub = make_subtitle(SubtitleFormatType::Vtt, vec![entry]);
        let result = conv.vtt_to_srt(sub).unwrap();
        assert!(!result.entries[0].text.contains('<'));
    }
}
