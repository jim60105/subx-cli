//! Serializer for the WebVTT subtitle format.

use crate::Result;
use crate::core::formats::Subtitle;

use super::time::format_vtt_time_range;

/// Serialize a [`Subtitle`] into canonical WebVTT bytes.
///
/// The output begins with the `WEBVTT` signature followed by a blank
/// line and one block per entry: a numeric index, the cue timing line,
/// the cue text, and a trailing blank line.
pub(super) fn serialize(subtitle: &Subtitle) -> Result<String> {
    let mut output = String::new();
    output.push_str("WEBVTT\n\n");
    for entry in &subtitle.entries {
        output.push_str(&format!("{}\n", entry.index));
        output.push_str(&format_vtt_time_range(entry.start_time, entry.end_time));
        output.push_str(&format!("{}\n\n", entry.text));
    }
    Ok(output)
}
