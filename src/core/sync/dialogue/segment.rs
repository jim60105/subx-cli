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
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub is_speech: bool,
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

/// 靜默片段資料結構，可用於額外處理或分析
#[derive(Debug, Clone)]
pub struct SilenceSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
}

impl SilenceSegment {
    /// 建立靜默片段
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
