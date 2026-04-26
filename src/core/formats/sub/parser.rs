//! Parser for the MicroDVD/SubViewer SUB subtitle format.
//!
//! # Malformed-input disposition
//!
//! The parser implements the malformed-input matrix from
//! `subtitle-parser-hardening` for the SUB format:
//!
//! | Scenario | Disposition |
//! | --- | --- |
//! | Empty input file | return [`SubXError::SubtitleFormat`] |
//! | Per-cue body exceeds [`MAX_CUE_BYTES`] (1 MiB) | return [`SubXError::SubtitleFormat`] |
//! | Non-numeric frame range (line does not match `{\d+}{\d+}`) | skip-and-continue with `debug!` log |
//! | Frame number that decodes to > 24 h | skip-and-continue with `debug!` log |
//!
//! UTF-8 BOM consumption is *not* performed at the parser level for SUB
//! (the malformed-input matrix scopes that to SRT/VTT/ASS); a BOM-prefixed
//! SUB file simply fails to match the frame-range regex on its first line
//! and that line is skipped.

use crate::Result;
use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use crate::error::SubXError;
use regex::Regex;

use super::time::{DEFAULT_SUB_FPS, MAX_DURATION_MS, frame_to_duration, frame_to_ms};

/// Maximum allowed size, in bytes, of a single SUB cue body.
///
/// Any cue whose body exceeds this cap causes the parser to return
/// [`SubXError::SubtitleFormat`] rather than allocating an unbounded
/// string. The value is deliberately a parser-local constant so the
/// limit does not depend on configuration; see the
/// `subtitle-parser-hardening` spec for rationale.
pub(super) const MAX_CUE_BYTES: usize = 1024 * 1024;

const FORMAT_NAME: &str = "SUB";

/// Parse SUB content into a [`Subtitle`].
///
/// See the [module-level documentation](self) for the malformed-input
/// disposition matrix.
pub(super) fn parse(content: &str) -> Result<Subtitle> {
    if content.is_empty() {
        return Err(SubXError::subtitle_format(FORMAT_NAME, "empty input"));
    }

    let fps = DEFAULT_SUB_FPS;
    let re = Regex::new(r"^\{(\d+)\}\{(\d+)\}(.*)")
        .map_err(|e| SubXError::subtitle_format(FORMAT_NAME, e.to_string()))?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let Some(cap) = re.captures(l) else {
            log::debug!(
                "Skipping SUB line that does not match `{{start}}{{end}}text`: {:?}",
                l
            );
            continue;
        };

        let raw_text = &cap[3];
        if raw_text.len() > MAX_CUE_BYTES {
            return Err(SubXError::subtitle_format(
                FORMAT_NAME,
                format!(
                    "cue body of {} bytes exceeds per-cue cap of {} bytes",
                    raw_text.len(),
                    MAX_CUE_BYTES
                ),
            ));
        }

        let start_frame: u64 = cap[1].parse().map_err(|e: std::num::ParseIntError| {
            SubXError::subtitle_format(FORMAT_NAME, e.to_string())
        })?;
        let end_frame: u64 = cap[2].parse().map_err(|e: std::num::ParseIntError| {
            SubXError::subtitle_format(FORMAT_NAME, e.to_string())
        })?;
        let text = raw_text.replace('|', "\n");

        let start_ms = frame_to_ms(start_frame, fps);
        let end_ms = frame_to_ms(end_frame, fps);
        if start_ms > MAX_DURATION_MS || end_ms > MAX_DURATION_MS {
            log::debug!(
                "Skipping SUB entry with out-of-range frames: {{{}}}{{{}}} (computed {}ms -> {}ms, limit {}ms)",
                start_frame,
                end_frame,
                start_ms,
                end_ms,
                MAX_DURATION_MS
            );
            continue;
        }

        entries.push(SubtitleEntry {
            index: entries.len() + 1,
            start_time: frame_to_duration(start_frame, fps),
            end_time: frame_to_duration(end_frame, fps),
            text,
            styling: None,
        });
    }

    Ok(Subtitle {
        entries,
        metadata: SubtitleMetadata {
            title: None,
            language: None,
            encoding: "utf-8".to_string(),
            frame_rate: Some(fps),
            original_format: SubtitleFormatType::Sub,
        },
        format: SubtitleFormatType::Sub,
    })
}

/// Cheap heuristic that returns `true` when `content` looks like a
/// MicroDVD/SubViewer SUB file (its first non-leading-whitespace bytes
/// match `{N}{N}`).
pub(super) fn detect(content: &str) -> bool {
    if let Ok(re) = Regex::new(r"^\{\d+\}\{\d+\}") {
        return re.is_match(content.trim_start());
    }
    false
}
