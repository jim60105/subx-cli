//! Dialogue segment representing either speech or silence interval.
//!
//! This struct stores timing and confidence information for detected
//! dialogue or silence regions in audio tracks.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::core::sync::dialogue::segment::DialogueSegment;
//! let speech = DialogueSegment::new_speech(0.0, 1.5);
//! ```

/// Represents a segment of audio containing dialogue or silence.
///
/// Used by the audio synchronization system to identify speech patterns
/// in audio files for subtitle timing alignment.
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    /// Start time of the segment in seconds
    pub start_time: f64,
    /// End time of the segment in seconds
    pub end_time: f64,
    /// Whether this segment contains speech
    pub is_speech: bool,
    /// Confidence level of the speech detection (0.0 to 1.0)
    pub confidence: f32,
}

impl DialogueSegment {
    /// Creates a new `DialogueSegment` representing a speech interval.
    pub fn new_speech(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: true,
            confidence: 1.0,
        }
    }

    /// Creates a new `DialogueSegment` representing a silence interval.
    pub fn new_silence(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            is_speech: false,
            confidence: 1.0,
        }
    }

    /// Returns the duration of the segment in seconds.
    pub fn duration(&self) -> f64 {
        self.end_time - self.start_time
    }

    /// Determines if this segment overlaps with another segment.
    pub fn overlaps_with(&self, other: &DialogueSegment) -> bool {
        self.start_time < other.end_time && self.end_time > other.start_time
    }

    /// 與其他同類型片段合併，更新邊界與信心度
    pub fn merge_with(&mut self, other: &DialogueSegment) {
        if self.is_speech == other.is_speech && self.overlaps_with(other) {
            self.start_time = self.start_time.min(other.start_time);
            self.end_time = self.end_time.max(other.end_time);
            self.confidence = (self.confidence + other.confidence) / 2.0;
        }
    }
}

/// Silence segment data structure for additional processing or analysis.
///
/// Represents periods of silence in audio that can be used for
/// subtitle timing adjustments and synchronization optimization.
#[derive(Debug, Clone)]
pub struct SilenceSegment {
    /// Start time of the silence segment in seconds
    pub start_time: f64,
    /// End time of the silence segment in seconds
    pub end_time: f64,
    /// Duration of the silence segment in seconds
    pub duration: f64,
}

impl SilenceSegment {
    /// Creates a new silence segment with the specified time range.
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            start_time: start,
            end_time: end,
            duration: end - start,
        }
    }

    /// 檢查此靜默片段是否超過最小長度
    pub fn is_significant(&self, min_duration: f64) -> bool {
        self.duration >= min_duration
    }
}
