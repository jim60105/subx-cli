//! Web Video Text Tracks (WebVTT) subtitle format implementation.
//!
//! This module exposes the [`VttFormat`] adapter that implements the
//! [`SubtitleFormat`] trait. The actual logic is split across:
//!
//! - `parser`: parsing logic and the malformed-input disposition matrix
//! - `serializer`: serialization logic
//! - `time`: WebVTT timestamp parsing and formatting helpers
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::formats::{SubtitleFormat, vtt::VttFormat};
//! let vtt = VttFormat;
//! let content = "WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello";
//! let subtitle = vtt.parse(content).unwrap();
//! ```

use crate::Result;
use crate::core::formats::{Subtitle, SubtitleFormat};

mod parser;
mod serializer;
mod time;

#[cfg(test)]
mod tests;

/// Subtitle format implementation for WebVTT.
///
/// The `VttFormat` struct implements parsing, serialization, and
/// detection for WebVTT files. See the `parser` module documentation
/// for the malformed-input disposition matrix observed by
/// [`VttFormat::parse`].
pub struct VttFormat;

impl SubtitleFormat for VttFormat {
    /// Parse WebVTT content.
    ///
    /// # Malformed-input dispositions
    ///
    /// - Empty input → returns [`crate::error::SubXError::SubtitleFormat`].
    /// - Missing `WEBVTT` signature → returns
    ///   [`crate::error::SubXError::SubtitleFormat`].
    /// - Leading UTF-8 BOM with valid content → BOM is consumed and the
    ///   file is parsed normally. This parser-level strip is *additive*
    ///   to [`crate::core::formats::encoding::converter`]'s file-layer
    ///   strip; both layers coexist by design.
    /// - Leading UTF-8 BOM with otherwise invalid content → returns
    ///   [`crate::error::SubXError::SubtitleFormat`] (the post-BOM
    ///   `WEBVTT` check fails).
    /// - Out-of-order cues → preserved in original file order; no
    ///   implicit sort is performed.
    /// - Cue marker line with a negative timestamp → that block is
    ///   skipped, a `debug!` log records the skip, and parsing
    ///   continues.
    /// - Cue body exceeding the per-cue cap (1 MiB) → returns
    ///   [`crate::error::SubXError::SubtitleFormat`]. The cap is enforced
    ///   on raw pre-normalization bytes, so `\r\n` padding cannot bypass
    ///   the limit by shrinking under line-ending normalization.
    /// - CRLF (`\r\n`), bare-CR (`\r`), or mixed line endings → input is
    ///   normalized to LF after the `WEBVTT` header check and before
    ///   block splitting; CRLF and mixed inputs parse to the same entry
    ///   count and text content as their LF counterpart.
    /// - Final cue not followed by a trailing blank line → still
    ///   recognized; the file parses successfully.
    fn parse(&self, content: &str) -> Result<Subtitle> {
        parser::parse(content)
    }

    fn serialize(&self, subtitle: &Subtitle) -> Result<String> {
        serializer::serialize(subtitle)
    }

    fn detect(&self, content: &str) -> bool {
        parser::detect(content)
    }

    fn format_name(&self) -> &'static str {
        "VTT"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["vtt"]
    }
}
