//! ASS/SSA timestamp parsing and formatting helpers.
//!
//! ASS timestamps use the `H:MM:SS.cc` format where `cc` is centiseconds
//! (hundredths of a second).

use crate::Result;
use crate::error::SubXError;
use std::time::Duration;

/// Parse an ASS timestamp (`H:MM:SS.cc`) into a [`Duration`].
///
/// Returns `SubXError::SubtitleFormat` for malformed input or arithmetic
/// overflow. Negative timestamps fail to parse here (because the integer
/// components are unsigned); callers can pre-check for a leading `-` if
/// they need to distinguish "negative" from "garbage" input.
pub(super) fn parse_ass_time(time: &str) -> Result<Duration> {
    let parts: Vec<&str> = time.split(&[':', '.'][..]).collect();
    if parts.len() != 4 {
        return Err(SubXError::subtitle_format(
            "ASS",
            format!("Invalid time format: {}", time),
        ));
    }
    let hours: u64 = parts[0]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let minutes: u64 = parts[1]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let seconds: u64 = parts[2]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;
    let centi: u64 = parts[3]
        .parse()
        .map_err(|e: std::num::ParseIntError| SubXError::subtitle_format("ASS", e.to_string()))?;

    let overflow =
        || SubXError::subtitle_format("ASS", format!("Timestamp arithmetic overflow: {}", time));
    let total_ms = hours
        .checked_mul(3_600_000)
        .ok_or_else(overflow)?
        .checked_add(minutes.checked_mul(60_000).ok_or_else(overflow)?)
        .ok_or_else(overflow)?
        .checked_add(seconds.checked_mul(1_000).ok_or_else(overflow)?)
        .ok_or_else(overflow)?
        .checked_add(centi.checked_mul(10).ok_or_else(overflow)?)
        .ok_or_else(overflow)?;
    Ok(Duration::from_millis(total_ms))
}

/// Format a [`Duration`] as an ASS timestamp (`H:MM:SS.cc`).
pub(super) fn format_ass_time(duration: Duration) -> String {
    let total_ms = duration.as_millis();
    let hours = total_ms / 3600000;
    let minutes = (total_ms % 3600000) / 60000;
    let seconds = (total_ms % 60000) / 1000;
    let centi = (total_ms % 1000) / 10;
    format!("{}:{:02}:{:02}.{:02}", hours, minutes, seconds, centi)
}
