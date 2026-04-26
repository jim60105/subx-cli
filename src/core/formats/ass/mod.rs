//! Advanced SubStation Alpha (ASS/SSA) subtitle format implementation.
//!
//! This module provides parsing, serialization, and detection capabilities
//! for the ASS/SSA subtitle format, including style and color definitions.
//!
//! Implementation is split across the following submodules:
//!
//! - `parser`: pure parsing from `&str` into [`Subtitle`].
//! - `serializer`: pure serialization from [`Subtitle`] back to ASS text.
//! - `time`: timestamp parsing/formatting helpers.
//! - `tests` (test-only): co-located unit tests for the format.
//!
//! `AssStyle` and `Color` style descriptors are kept here in `mod.rs`
//! because they are small data structures shared across this module.
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
use crate::core::formats::{Subtitle, SubtitleFormat};

mod parser;
mod serializer;
mod time;

#[cfg(test)]
mod tests;

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
/// for the ASS/SSA subtitle format. The actual logic is delegated to the
/// `parser`, `serializer`, and `time` submodules.
pub struct AssFormat;

impl SubtitleFormat for AssFormat {
    /// Parse ASS/SSA subtitle content into a [`Subtitle`].
    ///
    /// # Malformed-input dispositions
    ///
    /// Per the `subtitle-parser-hardening` capability matrix, this parser
    /// classifies malformed inputs as follows:
    ///
    /// | Scenario | Disposition |
    /// | --- | --- |
    /// | Empty input | return `SubXError::SubtitleFormat` |
    /// | Missing `[Events]` section | return `SubXError::SubtitleFormat` |
    /// | UTF-8 BOM prefix on valid content | consumed; parse continues |
    /// | UTF-8 BOM prefix on invalid content | return `SubXError::SubtitleFormat` |
    /// | `Format:` line missing `Start`, `End`, or `Text` | return `SubXError::SubtitleFormat` |
    /// | `Dialogue:` row column count mismatches `Format:` | skip-and-continue (`debug!`) |
    /// | Negative timestamp on a `Dialogue:` row | skip-and-continue (`debug!`) |
    /// | Timestamp arithmetic overflow | return `SubXError::SubtitleFormat` |
    /// | Cue body exceeding `MAX_CUE_BYTES` (1 MiB) | return `SubXError::SubtitleFormat` |
    fn parse(&self, content: &str) -> Result<Subtitle> {
        parser::parse(content)
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        serializer::serialize(subtitle)
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
