//! SubRip (`.srt`) subtitle format support.
//!
//! This module is split across:
//!
//! - `parser`    — pure parsing (block splitting, validation, hardening).
//! - `serializer` — pure serialization to canonical SRT bytes.
//! - `time`      — timestamp parsing/formatting helpers.
//! - `tests`     — co-located unit tests.
//!
//! The public entry point remains `SrtFormat` which implements
//! [`crate::core::formats::SubtitleFormat`].

use crate::Result;
use crate::core::formats::{Subtitle, SubtitleFormat};
use regex::Regex;

mod parser;
mod serializer;
mod time;

#[cfg(test)]
mod tests;

/// SubRip (`.srt`) format parsing and serialization.
pub struct SrtFormat;

impl SubtitleFormat for SrtFormat {
    /// Parse SRT content into a [`Subtitle`].
    ///
    /// # Malformed-input dispositions
    ///
    /// Per the `subtitle-parser-hardening` spec matrix, this parser handles
    /// malformed inputs as follows:
    ///
    /// - **Empty input** → returns [`crate::error::SubXError::SubtitleFormat`].
    /// - **UTF-8 BOM prefix** → consumed as a defensive layer at the start of
    ///   `parse` (additive to the encoding-layer
    ///   `formats::encoding::converter::skip_bom`); BOM-only content reduces
    ///   to empty input and is rejected.
    /// - **Per-cue body exceeding `MAX_CUE_BYTES` (1 MiB)** → returns
    ///   [`crate::error::SubXError::SubtitleFormat`].
    /// - **Block with non-numeric sequence index** → skip-and-continue with
    ///   `debug!` log.
    /// - **Block with negative timestamp on the timing line** →
    ///   skip-and-continue with `debug!` log.
    /// - **Block whose timing line does not match the SRT regex** →
    ///   silently skipped (existing tolerated behavior).
    /// - **Out-of-order cues by start time** → preserved verbatim in
    ///   `entries`; no implicit sort.
    fn parse(&self, content: &str) -> Result<Subtitle> {
        parser::parse(content)
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        serializer::serialize(subtitle)
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
