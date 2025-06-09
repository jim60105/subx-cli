//! Core subtitle formats module.
//!
//! Defines common types and interfaces for parsing, converting, and managing
//! subtitle formats such as SRT, ASS, VTT, and SUB.
#![allow(dead_code)]

pub mod ass;
pub mod converter;
pub mod encoding;
pub mod manager;
pub mod srt;
pub mod styling;
pub mod sub;
pub mod transformers;
pub mod vtt;

use std::time::Duration;

/// Supported subtitle format types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleFormatType {
    Srt,
    Ass,
    Vtt,
    Sub,
}

impl SubtitleFormatType {
    /// Get the format as a lowercase string slice (e.g., "srt").
    pub fn as_str(&self) -> &'static str {
        match self {
            SubtitleFormatType::Srt => "srt",
            SubtitleFormatType::Ass => "ass",
            SubtitleFormatType::Vtt => "vtt",
            SubtitleFormatType::Sub => "sub",
        }
    }
}

impl std::fmt::Display for SubtitleFormatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unified subtitle data structure containing entries, metadata, and format.
#[derive(Debug, Clone)]
pub struct Subtitle {
    pub entries: Vec<SubtitleEntry>,
    pub metadata: SubtitleMetadata,
    pub format: SubtitleFormatType,
}

/// Single subtitle entry containing timing, index, and text information.
///
/// # Fields
///
/// - `index`: Sequence number of the subtitle entry.
/// - `start_time`: Start timestamp of the subtitle entry.
/// - `end_time`: End timestamp of the subtitle entry.
/// - `text`: Text content of the subtitle entry.
/// - `styling`: Optional styling information (font, color, formatting).
#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: usize,
    pub start_time: Duration,
    pub end_time: Duration,
    pub text: String,
    pub styling: Option<StylingInfo>,
}

/// Metadata associated with a subtitle file.
///
/// Contains optional title, language, encoding, frame rate, and original format.
#[derive(Debug, Clone)]
pub struct SubtitleMetadata {
    pub title: Option<String>,
    pub language: Option<String>,
    pub encoding: String,
    pub frame_rate: Option<f32>,
    pub original_format: SubtitleFormatType,
}

/// Optional styling information for subtitle entries.
///
/// Includes font name, size, color, and text decoration options.
#[derive(Debug, Clone, Default)]
pub struct StylingInfo {
    pub font_name: Option<String>,
    pub font_size: Option<u32>,
    pub color: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// Trait defining parsing, serialization, and detection for subtitle formats.
pub trait SubtitleFormat {
    /// Parse subtitle content into a `Subtitle` data structure.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw subtitle file content.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails due to invalid format.
    fn parse(&self, content: &str) -> crate::Result<Subtitle>;

    /// Serialize a `Subtitle` into the specific format text.
    ///
    /// # Arguments
    ///
    /// * `subtitle` - Subtitle data to serialize.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String>;

    /// Detect whether the provided content matches this format.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw subtitle file content.
    fn detect(&self, content: &str) -> bool;

    /// Returns the human-readable name of this subtitle format.
    fn format_name(&self) -> &'static str;

    /// Returns the supported file extensions for this format.
    fn file_extensions(&self) -> &'static [&'static str];
}
