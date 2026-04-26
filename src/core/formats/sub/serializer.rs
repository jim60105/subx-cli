//! Serializer for the MicroDVD/SubViewer SUB subtitle format.

use crate::Result;
use crate::core::formats::Subtitle;

use super::time::{DEFAULT_SUB_FPS, duration_to_frame};

/// Serialize a [`Subtitle`] into MicroDVD/SubViewer SUB text.
///
/// Newlines inside cue text are encoded as the SUB-native `|` separator.
/// The frame rate stored in `Subtitle::metadata.frame_rate` is used
/// when present; otherwise [`DEFAULT_SUB_FPS`] is assumed.
pub(super) fn serialize(subtitle: &Subtitle) -> Result<String> {
    let fps = subtitle.metadata.frame_rate.unwrap_or(DEFAULT_SUB_FPS);
    let mut output = String::new();
    for entry in &subtitle.entries {
        let start_frame = duration_to_frame(entry.start_time, fps);
        let end_frame = duration_to_frame(entry.end_time, fps);
        let text = entry.text.replace('\n', "|");
        output.push_str(&format!("{{{}}}{{{}}}{}\n", start_frame, end_frame, text));
    }
    Ok(output)
}
