//! Time helpers for the WebVTT subtitle format.
//!
//! Provides parsing and formatting of WebVTT timestamps in the form
//! `HH:MM:SS.mmm`.

use crate::Result;
use crate::error::SubXError;
use std::time::Duration;

/// Parse a WebVTT timestamp from a regex capture group starting at index `start`.
///
/// The capture is expected to expose four consecutive groups: hours, minutes,
/// seconds, and milliseconds, all as decimal integers.
pub(super) fn parse_vtt_time(caps: &regex::Captures, start: usize) -> Result<Duration> {
    let hours: u64 = caps[start]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let minutes: u64 = caps[start + 1]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let seconds: u64 = caps[start + 2]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    let millis: u64 = caps[start + 3]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("VTT", e.to_string()))?;
    Ok(Duration::from_millis(
        hours * 3600 * 1000 + minutes * 60 * 1000 + seconds * 1000 + millis,
    ))
}

/// Format a [`Duration`] as a WebVTT timestamp `HH:MM:SS.mmm`.
pub(super) fn format_vtt_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let millis = total_ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
}

/// Format a `start --> end` WebVTT cue timing line, including the trailing
/// newline.
pub(super) fn format_vtt_time_range(start: Duration, end: Duration) -> String {
    format!("{} --> {}\n", format_vtt_time(start), format_vtt_time(end))
}
