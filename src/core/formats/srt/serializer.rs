//! SRT serializer — emits the canonical `index / timing / text / blank-line`
//! form for each entry.

use super::time::format_time_range;
use crate::Result;
use crate::core::formats::Subtitle;

/// Serialize a [`Subtitle`] to canonical SRT bytes.
///
/// Entry indices are renumbered starting at 1 to match the canonical form
/// regardless of the input `entry.index` values.
pub(super) fn serialize(subtitle: &Subtitle) -> Result<String> {
    let mut output = String::new();

    for (i, entry) in subtitle.entries.iter().enumerate() {
        output.push_str(&format!("{}\n", i + 1));
        output.push_str(&format_time_range(entry.start_time, entry.end_time));
        output.push_str(&format!("{}\n\n", entry.text));
    }

    Ok(output)
}
