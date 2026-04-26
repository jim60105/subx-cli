//! Frame ↔ time conversion helpers for the MicroDVD/SubViewer SUB format.
//!
//! SUB encodes cue boundaries as integer frame numbers; converting those to
//! [`std::time::Duration`] and back requires a frame-rate value. The default
//! is [`DEFAULT_SUB_FPS`].

use std::time::Duration;

/// Default frame rate assumed when a SUB file does not encode one explicitly.
pub(super) const DEFAULT_SUB_FPS: f32 = 25.0;

/// 24-hour upper bound (in milliseconds) used to discard absurd frame numbers.
///
/// A SUB cue whose computed start or end time exceeds this limit is treated
/// as malformed and skipped by the parser (see the parser's malformed-input
/// disposition matrix).
pub(super) const MAX_DURATION_MS: u64 = 86_400_000;

/// Convert a frame number to milliseconds at the given frame rate.
///
/// Rounds to the nearest millisecond.
pub(super) fn frame_to_ms(frame: u64, fps: f32) -> u64 {
    (frame as f64 * 1000.0 / fps as f64).round() as u64
}

/// Convert a [`Duration`] to a frame count at the given frame rate.
///
/// Rounds to the nearest frame.
pub(super) fn duration_to_frame(d: Duration, fps: f32) -> u64 {
    (d.as_secs_f64() * fps as f64).round() as u64
}

/// Convenience wrapper that converts a frame number directly to a
/// [`Duration`].
pub(super) fn frame_to_duration(frame: u64, fps: f32) -> Duration {
    Duration::from_millis(frame_to_ms(frame, fps))
}
