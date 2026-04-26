//! SRT timestamp parsing and formatting helpers.
//!
//! SRT uses the `HH:MM:SS,mmm` form; durations in this codebase use
//! [`std::time::Duration`].

use crate::Result;
use crate::error::SubXError;
use std::time::Duration;

/// Parse a four-group SRT timestamp out of a `regex::Captures` starting at
/// `start_group` (groups: hours, minutes, seconds, milliseconds).
pub(super) fn parse_time(caps: &regex::Captures, start_group: usize) -> Result<Duration> {
    let hours: u64 = caps[start_group].parse().map_err(|e| {
        SubXError::subtitle_format("SRT", format!("Time value parsing failed: {}", e))
    })?;
    let minutes: u64 = caps[start_group + 1].parse().map_err(|e| {
        SubXError::subtitle_format("SRT", format!("Time value parsing failed: {}", e))
    })?;
    let seconds: u64 = caps[start_group + 2].parse().map_err(|e| {
        SubXError::subtitle_format("SRT", format!("Time value parsing failed: {}", e))
    })?;
    let milliseconds: u64 = caps[start_group + 3].parse().map_err(|e| {
        SubXError::subtitle_format("SRT", format!("Time value parsing failed: {}", e))
    })?;

    Ok(Duration::from_millis(
        hours * 3600000 + minutes * 60000 + seconds * 1000 + milliseconds,
    ))
}

/// Format a `start --> end` SRT timing line, including the trailing newline.
pub(super) fn format_time_range(start: Duration, end: Duration) -> String {
    format!("{} --> {}\n", format_duration(start), format_duration(end))
}

/// Format a single [`Duration`] as `HH:MM:SS,mmm`.
pub(super) fn format_duration(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let milliseconds = total_ms % 1000;

    format!(
        "{:02}:{:02}:{:02},{:03}",
        hours, minutes, seconds, milliseconds
    )
}
