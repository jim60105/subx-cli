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
