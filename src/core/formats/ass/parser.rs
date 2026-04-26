//! ASS/SSA parser.
//!
//! Implements the malformed-input dispositions documented on
//! [`crate::core::formats::ass::AssFormat`]'s `parse` method.

use crate::Result;
use crate::core::formats::{Subtitle, SubtitleEntry, SubtitleFormatType, SubtitleMetadata};
use crate::error::SubXError;
use log::debug;

use super::time::parse_ass_time;

/// Per-cue body byte cap. Hostile inputs that exceed this for a single
/// `Dialogue:` text payload are rejected with `SubXError::SubtitleFormat`
/// to bound parser-side allocation.
///
/// Chosen to comfortably exceed any realistic cue (typical cue text is
/// well under 1 KiB; even verbose ASS karaoke `{\k...}` lines stay below
/// 100 KiB) while still capping pathological inputs. The constant is
/// intentionally parser-local and does NOT consult any configuration
/// value because [`crate::core::formats::SubtitleFormat::parse`] does
/// not receive a `ConfigService`.
pub(super) const MAX_CUE_BYTES: usize = 1024 * 1024;

/// Parse ASS/SSA content into a [`Subtitle`].
///
/// See [`crate::core::formats::ass::AssFormat`]'s `parse` method for the full
/// malformed-input disposition matrix.
pub(super) fn parse(content: &str) -> Result<Subtitle> {
    // Defensive parser-level BOM consumption. This is additive to the
    // encoding-layer BOM strip in `formats::encoding::converter::skip_bom`
    // and exists so callers using `parse_auto` or invoking
    // `AssFormat::parse(&str)` directly on in-memory strings also tolerate
    // a leading UTF-8 BOM.
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);

    if content.trim().is_empty() {
        return Err(SubXError::subtitle_format("ASS", "Empty input"));
    }

    let mut entries = Vec::new();
    let mut in_events = false;
    let mut saw_events_header = false;
    let mut fields: Vec<&str> = Vec::new();

    for line in content.lines() {
        let l = line.trim_start();
        if l.eq_ignore_ascii_case("[events]") {
            in_events = true;
            saw_events_header = true;
            continue;
        }
        if !in_events {
            continue;
        }
        // Other section headers terminate the events block.
        if l.starts_with('[') && l.ends_with(']') {
            in_events = false;
            continue;
        }
        if l.to_lowercase().starts_with("format:") {
            fields = l["Format:".len()..].split(',').map(|s| s.trim()).collect();
            continue;
        }
        if l.to_lowercase().starts_with("dialogue:") {
            if fields.is_empty() {
                debug!(
                    "ASS parser: skipping Dialogue row before Format declaration: {}",
                    l
                );
                continue;
            }
            let data = l["Dialogue:".len()..].trim();
            let parts: Vec<&str> = data.splitn(fields.len(), ',').collect();
            if parts.len() != fields.len() {
                debug!(
                    "ASS parser: Dialogue column count {} mismatches Format column count {}; skipping row",
                    parts.len(),
                    fields.len()
                );
                continue;
            }
            let start_index = fields
                .iter()
                .position(|&f| f.eq_ignore_ascii_case("start"))
                .ok_or_else(|| {
                    SubXError::subtitle_format("ASS", "Missing 'Start' field in Format declaration")
                })?;
            let end_index = fields
                .iter()
                .position(|&f| f.eq_ignore_ascii_case("end"))
                .ok_or_else(|| {
                    SubXError::subtitle_format("ASS", "Missing 'End' field in Format declaration")
                })?;
            let text_index = fields
                .iter()
                .position(|&f| f.eq_ignore_ascii_case("text"))
                .ok_or_else(|| {
                    SubXError::subtitle_format("ASS", "Missing 'Text' field in Format declaration")
                })?;
            let start = parts[start_index].trim();
            let end = parts[end_index].trim();

            // Skip-and-continue on negative timestamps. The integer
            // parser inside `parse_ass_time` would reject these as well,
            // but doing the check here lets us classify "negative" as
            // skip rather than letting any other malformed-time error
            // tunnel through with the same disposition.
            if start.starts_with('-') || end.starts_with('-') {
                debug!(
                    "ASS parser: skipping Dialogue row with negative timestamp (start={}, end={})",
                    start, end
                );
                continue;
            }

            let text = parts[text_index..].join(",").replace("\\N", "\n");
            if text.len() > MAX_CUE_BYTES {
                return Err(SubXError::subtitle_format(
                    "ASS",
                    format!(
                        "Dialogue text exceeds per-cue byte cap ({} > {})",
                        text.len(),
                        MAX_CUE_BYTES
                    ),
                ));
            }

            let start_time = parse_ass_time(start)?;
            let end_time = parse_ass_time(end)?;
            entries.push(SubtitleEntry {
                index: entries.len() + 1,
                start_time,
                end_time,
                text,
                styling: None,
            });
        }
    }

    if !saw_events_header {
        return Err(SubXError::subtitle_format(
            "ASS",
            "Missing [Events] section",
        ));
    }

    Ok(Subtitle {
        entries,
        metadata: SubtitleMetadata {
            title: None,
            language: None,
            encoding: "utf-8".to_string(),
            frame_rate: None,
            original_format: SubtitleFormatType::Ass,
        },
        format: SubtitleFormatType::Ass,
    })
}
