//! SRT parser — block-splitting and per-block validation logic.
//!
//! See [`super::SrtFormat`]'s `parse` method for the malformed-input disposition matrix
//! enforced here.

use super::time::parse_time;
use crate::Result;
use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use crate::error::SubXError;
use regex::Regex;

/// Maximum byte size of a single SRT cue block. Inputs whose individual
/// block exceeds this cap return a typed error rather than allocating an
/// unbounded string.
///
/// The value is a parser-local fixed constant per the
/// `subtitle-parser-hardening` spec — it deliberately does NOT consult
/// `general.max_subtitle_bytes` because `SubtitleFormat::parse` does not
/// receive a `ConfigService`. File-level size enforcement remains at the
/// command/file-read layer.
const MAX_CUE_BYTES: usize = 1024 * 1024;

/// Detect a `-` sign immediately preceding any timestamp digit cluster on
/// the timing line. Used purely to recognize the negative-timestamp
/// scenario for skip-and-continue.
fn timing_line_has_negative(line: &str) -> bool {
    // Matches optional leading `-` followed by `HH:MM:SS,mmm` on either
    // side of the `-->` arrow, capturing only when the sign is present.
    static NEG_PATTERN: &str = r"-\d+:\d{2}:\d{2},\d{3}";
    Regex::new(NEG_PATTERN).unwrap().is_match(line)
}

/// Parse SRT `content` into a [`Subtitle`].
pub(super) fn parse(content: &str) -> Result<Subtitle> {
    // Defensive parser-level BOM strip. This is additive to the encoding
    // layer's `skip_bom`; both coexist by design so that callers using
    // `FormatManager::parse_auto` or direct `SrtFormat::parse(&str)` on
    // in-memory strings also receive BOM tolerance.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);

    if content.is_empty() {
        return Err(SubXError::subtitle_format(
            "SRT",
            "Input is empty after BOM stripping",
        ));
    }

    let time_regex =
        Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3}) --> (\d{2}):(\d{2}):(\d{2}),(\d{3})")
            .map_err(|e| {
                SubXError::subtitle_format("SRT", format!("Time format compilation error: {}", e))
            })?;

    let mut entries = Vec::new();
    let blocks: Vec<&str> = content.split("\n\n").collect();

    for block in blocks {
        if block.trim().is_empty() {
            continue;
        }

        if block.len() > MAX_CUE_BYTES {
            return Err(SubXError::subtitle_format(
                "SRT",
                format!(
                    "Single cue block exceeds {}-byte cap (got {} bytes)",
                    MAX_CUE_BYTES,
                    block.len()
                ),
            ));
        }

        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 3 {
            continue;
        }

        let index: usize = match lines[0].trim().parse() {
            Ok(idx) => idx,
            Err(e) => {
                log::debug!(
                    "Skipping SRT block with invalid sequence number '{}': {}",
                    lines[0].trim(),
                    e
                );
                continue;
            }
        };

        if timing_line_has_negative(lines[1]) {
            log::debug!(
                "Skipping SRT block {} with negative timestamp on timing line: {}",
                index,
                lines[1]
            );
            continue;
        }

        if let Some(caps) = time_regex.captures(lines[1]) {
            let start_time = parse_time(&caps, 1)?;
            let end_time = parse_time(&caps, 5)?;
            let text = lines[2..].join("\n");

            entries.push(SubtitleEntry {
                index,
                start_time,
                end_time,
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
            original_format: SubtitleFormatType::Srt,
        },
        format: SubtitleFormatType::Srt,
    })
}

#[cfg(test)]
pub(super) const MAX_CUE_BYTES_FOR_TESTS: usize = MAX_CUE_BYTES;
