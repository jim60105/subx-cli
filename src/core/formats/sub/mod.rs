//! MicroDVD/SubViewer SUB subtitle format implementation.
//!
//! This module exposes the [`SubFormat`] adapter that implements the
//! [`SubtitleFormat`] trait. The actual logic is split across:
//!
//! - `parser`: parsing logic and the malformed-input disposition matrix
//! - `serializer`: serialization logic
//! - `time`: frame ↔ time helpers and shared constants
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::formats::{SubtitleFormat, sub::SubFormat};
//! let sub = SubFormat;
//! let content = "{0}{25}Hello\n";
//! let subtitle = sub.parse(content).unwrap();
//! ```

use crate::Result;
use crate::core::formats::{Subtitle, SubtitleFormat};

mod parser;
mod serializer;
mod time;

#[cfg(test)]
mod tests;

/// Subtitle format implementation for MicroDVD/SubViewer SUB.
///
/// The `SubFormat` struct implements parsing, serialization, and
/// detection for SUB files using frame-based timing. See the
/// `parser` module documentation for the malformed-input disposition
/// matrix observed by [`SubFormat::parse`].
pub struct SubFormat;

impl SubtitleFormat for SubFormat {
    /// Parse SUB content.
    ///
    /// # Malformed-input dispositions
    ///
    /// - Empty input → returns [`crate::error::SubXError::SubtitleFormat`].
    /// - A single cue whose body exceeds the per-cue cap (1 MiB)
    ///   → returns [`crate::error::SubXError::SubtitleFormat`].
    /// - Non-numeric frame range (line that does not match
    ///   `{\d+}{\d+}`) → skipped with a `debug!` log; parsing continues.
    /// - Frame number that decodes to > 24 h → skipped with a `debug!`
    ///   log; parsing continues.
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
        "SUB"
    }

    fn file_extensions(&self) -> &'static [&'static str] {
        &["sub"]
    }
}
