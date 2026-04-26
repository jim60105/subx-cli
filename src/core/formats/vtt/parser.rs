//! Parser for the WebVTT subtitle format.
//!
//! # Malformed-input disposition
//!
//! The parser implements the malformed-input matrix from
//! `subtitle-parser-hardening` for the VTT format:
//!
//! | Scenario | Disposition |
//! | --- | --- |
//! | Empty input file | return [`SubXError::SubtitleFormat`] |
//! | Truncated header (no `WEBVTT` signature) | return [`SubXError::SubtitleFormat`] |
//! | UTF-8 BOM-prefixed valid content | parse successfully (BOM consumed at parser level) |
//! | UTF-8 BOM-prefixed invalid content | return [`SubXError::SubtitleFormat`] (header check fails after BOM) |
//! | Out-of-order cues by timestamp | parse successfully (original order preserved, no implicit sort) |
//! | Cue marker line with a negative timestamp (e.g. `-00:00:01.000 --> ...`) | skip-and-continue with `debug!` log |
//! | Per-cue body exceeding [`MAX_CUE_BYTES`] (1 MiB) | return [`SubXError::SubtitleFormat`] |
//! | File missing trailing blank line at EOF | parse successfully (final cue still recognized) |
//!
//! The parser-level BOM strip is intentionally additive to the
//! encoding-layer BOM strip in
//! [`crate::core::formats::encoding::converter`]; both layers coexist by
//! design so callers using `FormatManager::parse_auto` or direct
//! [`super::VttFormat`]'s `parse` method on in-memory strings (which bypass the
//! encoding layer) still receive BOM tolerance.

use crate::Result;
use crate::core::formats::line_endings::{normalize_line_endings, raw_blocks};
use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use crate::error::SubXError;
use regex::Regex;

use super::time::parse_vtt_time;

/// Maximum allowed size, in bytes, of a single VTT cue body.
///
/// Any cue whose body exceeds this cap causes the parser to return
/// [`SubXError::SubtitleFormat`] rather than allocating an unbounded
/// string. The value is deliberately a parser-local constant so the
/// limit does not depend on configuration; see the
/// `subtitle-parser-hardening` spec for rationale.
pub(super) const MAX_CUE_BYTES: usize = 1024 * 1024;

const FORMAT_NAME: &str = "VTT";

/// UTF-8 byte-order mark, encoded as a `&str`.
const UTF8_BOM: &str = "\u{feff}";

/// Parse WebVTT content into a [`Subtitle`].
///
/// See the [module-level documentation](self) for the malformed-input
/// disposition matrix.
pub(super) fn parse(content: &str) -> Result<Subtitle> {
    if content.is_empty() {
        return Err(SubXError::subtitle_format(FORMAT_NAME, "empty input"));
    }

    // Defensive parser-level BOM strip. The encoding layer
    // (`encoding::converter::skip_bom`) already strips a BOM for files
    // read via `FormatManager::parse_file`, but callers operating on
    // in-memory strings bypass that layer. Both layers coexist by design.
    let content = content.strip_prefix(UTF8_BOM).unwrap_or(content);

    if !content.trim_start().starts_with("WEBVTT") {
        return Err(SubXError::subtitle_format(
            FORMAT_NAME,
            "missing WEBVTT signature",
        ));
    }

    // Enforce the per-cue size cap on the *raw* (pre-normalization)
    // byte length of every cue block, BEFORE collapsing CR / CRLF to
    // LF. This prevents an attacker from padding a multi-MiB cue with
    // `\r` bytes that would disappear after normalization. Non-cue
    // blocks (the `WEBVTT` header, `NOTE`, `STYLE`) are exempt to
    // preserve the pre-existing skip behavior — only blocks that the
    // post-normalization loop would treat as cues are size-checked
    // here.
    for raw_block in raw_blocks(content) {
        let trimmed = raw_block.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("WEBVTT")
            || trimmed.starts_with("NOTE")
            || trimmed.starts_with("STYLE")
        {
            continue;
        }
        if raw_block.len() > MAX_CUE_BYTES {
            return Err(SubXError::subtitle_format(
                FORMAT_NAME,
                format!(
                    "cue block of {} bytes exceeds per-cue cap of {} bytes",
                    raw_block.len(),
                    MAX_CUE_BYTES
                ),
            ));
        }
    }

    // Normalize CR / CRLF terminators to LF so the existing
    // `split("\n\n")` block-splitter, the per-block `block.lines()`
    // walk, and the timing regex all see a single canonical form.
    // LF-only inputs take the zero-allocation `Cow::Borrowed` fast
    // path.
    let normalized = normalize_line_endings(content);
    let content: &str = &normalized;

    let time_re =
        Regex::new(r"(?m)^(\d{2}):(\d{2}):(\d{2})\.(\d{3}) --> (\d{2}):(\d{2}):(\d{2})\.(\d{3})")
            .map_err(|e: regex::Error| SubXError::subtitle_format(FORMAT_NAME, e.to_string()))?;

    // Detects a cue marker line whose start or end timestamp begins with
    // a leading minus sign. Such lines never match `time_re` (which is
    // unsigned), so we detect them explicitly to satisfy the
    // "negative timestamp → skip-and-continue with debug log"
    // disposition from the hardening matrix.
    let neg_signed_re = Regex::new(r"(^|\s|>)-\d{1,}:\d{2}:\d{2}\.\d{3}")
        .map_err(|e: regex::Error| SubXError::subtitle_format(FORMAT_NAME, e.to_string()))?;

    let mut entries = Vec::new();
    for block in content.split("\n\n") {
        let block = block.trim();
        if block.is_empty()
            || block.starts_with("WEBVTT")
            || block.starts_with("NOTE")
            || block.starts_with("STYLE")
        {
            continue;
        }

        if block.len() > MAX_CUE_BYTES {
            return Err(SubXError::subtitle_format(
                FORMAT_NAME,
                format!(
                    "cue block of {} bytes exceeds per-cue cap of {} bytes",
                    block.len(),
                    MAX_CUE_BYTES
                ),
            ));
        }

        let lines: Vec<&str> = block.lines().collect();
        let mut idx = 0;
        if !time_re.is_match(lines[0]) {
            idx = 1;
            if idx >= lines.len() {
                continue;
            }
        }

        let marker_line = lines[idx];

        // Skip-and-continue on a negative-timestamp cue marker.
        if neg_signed_re.is_match(marker_line) {
            log::debug!(
                "Skipping VTT cue with negative timestamp: {:?}",
                marker_line
            );
            continue;
        }

        if let Some(caps) = time_re.captures(marker_line) {
            let start = parse_vtt_time(&caps, 1)?;
            let end = parse_vtt_time(&caps, 5)?;
            let text = lines[(idx + 1)..].join("\n");

            if text.len() > MAX_CUE_BYTES {
                return Err(SubXError::subtitle_format(
                    FORMAT_NAME,
                    format!(
                        "cue body of {} bytes exceeds per-cue cap of {} bytes",
                        text.len(),
                        MAX_CUE_BYTES
                    ),
                ));
            }

            entries.push(SubtitleEntry {
                index: entries.len() + 1,
                start_time: start,
                end_time: end,
                text,
                styling: None,
            });
        }
    }

    Ok(Subtitle {
        entries,
        metadata: SubtitleMetadata {
            title: None,
            language: None,
            encoding: "utf-8".to_string(),
            frame_rate: None,
            original_format: SubtitleFormatType::Vtt,
        },
        format: SubtitleFormatType::Vtt,
    })
}

/// Cheap heuristic that returns `true` when `content` begins with the
/// `WEBVTT` signature (after optional leading whitespace and an optional
/// UTF-8 BOM).
pub(super) fn detect(content: &str) -> bool {
    let stripped = content.strip_prefix(UTF8_BOM).unwrap_or(content);
    stripped.trim_start().starts_with("WEBVTT")
}
