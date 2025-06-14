//! Audio-subtitle synchronization command-line arguments and options.
//!
//! This module defines the command-line interface for the `sync` subcommand,
//! which handles timing alignment between video audio tracks and subtitle files.
//! It supports both automatic synchronization using audio analysis and manual
//! adjustment with specified time offsets.
//!
//! # Synchronization Methods
//!
//! ## Automatic Synchronization
//! Uses advanced audio analysis to detect speech patterns and align them
//! with subtitle timestamps:
//! - **Speech detection**: Identifies speech segments in audio
//! - **Pattern matching**: Correlates speech timing with subtitle timing
//! - **Offset calculation**: Determines optimal time shift
//! - **Confidence scoring**: Evaluates alignment quality
//!
//! ## Manual Synchronization
//! Applies user-specified time offset to all subtitle entries:
//! - Positive offset: Delays subtitles (subtitles appear later)
//! - Negative offset: Advances subtitles (subtitles appear earlier)
//! - Preserves relative timing between subtitle entries
//!
//! # Examples
//!
//! ```bash
//! # Automatic synchronization
//! subx sync video.mp4 subtitle.srt
//!
//! # Manual offset: delay subtitles by 2.5 seconds
//! subx sync video.mp4 subtitle.srt --offset 2.5
//!
//! # Batch processing with custom parameters
//! subx sync video.mp4 subtitle.srt --batch --range 10.0 --threshold 0.8
//! ```

// src/cli/sync_args.rs
use crate::error::{SubXError, SubXResult};
use clap::Args;
use std::path::PathBuf;

/// Command-line arguments for audio-subtitle synchronization.
///
/// The sync command aligns subtitle timing with video audio tracks using
/// either automatic audio analysis or manual time offset adjustment.
/// It provides fine-tuned control over the synchronization process with
/// configurable parameters for different content types.
///
/// # Workflow
///
/// 1. **Audio Analysis**: Extract audio features from the video file
/// 2. **Speech Detection**: Identify speech segments and timing
/// 3. **Pattern Matching**: Correlate speech patterns with subtitle timing
/// 4. **Offset Calculation**: Determine optimal time adjustment
/// 5. **Validation**: Verify synchronization quality
/// 6. **Application**: Apply timing adjustments to subtitle file
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::SyncArgs;
/// use std::path::PathBuf;
///
/// // Automatic synchronization
/// let auto_args = SyncArgs {
///     video: Some(PathBuf::from("movie.mp4")),
///     subtitle: PathBuf::from("movie.srt"),
///     offset: None,
///     batch: false,
///     range: Some(15.0),
///     threshold: Some(0.75),
/// };
///
/// // Manual synchronization with 2-second delay
/// let manual_args = SyncArgs {
///     video: Some(PathBuf::from("movie.mp4")),
///     subtitle: PathBuf::from("movie.srt"),
///     offset: Some(2.0),
///     batch: false,
///     range: None,
///     threshold: None,
/// };
/// ```
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Video file path for audio analysis (required for automatic sync).
    ///
    /// The video file from which audio will be extracted and analyzed
    /// for speech pattern detection. Supports common video formats:
    ///
    /// # Supported Formats
    /// - MP4, MKV, AVI (most common)
    /// - MOV, WMV, FLV
    /// - M4V, 3GP, WEBM
    /// - VOB, TS, MTS
    ///
    /// # Requirements
    /// - File must contain at least one audio track
    /// - Audio track should contain speech (not just music/effects)
    /// - Minimum duration of 30 seconds recommended for accuracy
    ///
    /// # Examples
    /// ```bash
    /// # Standard video file
    /// subx sync /path/to/movie.mp4 subtitle.srt
    ///
    /// # High-definition video
    /// subx sync "./Movies/Film (2023) [1080p].mkv" subtitle.srt
    /// ```
    #[arg(required_unless_present = "offset")]
    pub video: Option<PathBuf>,

    /// Subtitle file path to be synchronized.
    ///
    /// The subtitle file whose timing will be adjusted to match the
    /// video's audio track. Supports all major subtitle formats:
    ///
    /// # Supported Formats
    /// - SRT (SubRip): Most common format
    /// - ASS/SSA (Advanced SubStation Alpha): Rich formatting
    /// - VTT (WebVTT): Web-optimized format
    /// - SUB (MicroDVD): Frame-based format
    ///
    /// # File Requirements
    /// - Must contain valid timestamps
    /// - Should have reasonable subtitle density (not too sparse)
    /// - Text content should roughly correspond to audio speech
    ///
    /// # Examples
    /// ```bash
    /// # Various subtitle formats
    /// subx sync video.mp4 subtitle.srt
    /// subx sync video.mp4 subtitle.ass
    /// subx sync video.mp4 subtitle.vtt
    /// ```
    pub subtitle: PathBuf,

    /// Manual time offset in seconds (overrides automatic detection).
    ///
    /// When specified, disables automatic synchronization and applies
    /// a fixed time offset to all subtitle entries. Positive values
    /// delay subtitles (appear later), negative values advance them.
    ///
    /// # Precision
    /// Supports fractional seconds with millisecond precision:
    /// - 2.5 = 2 seconds, 500 milliseconds
    /// - -1.25 = -1 second, -250 milliseconds
    /// - 0.1 = 100 milliseconds
    ///
    /// # Use Cases
    /// - **Fine-tuning**: Small adjustments after automatic sync
    /// - **Known offset**: When you know the exact timing difference
    /// - **Problematic audio**: When automatic detection fails
    /// - **Batch processing**: Apply same offset to multiple files
    ///
    /// # Examples
    /// ```bash
    /// # Delay subtitles by 2.5 seconds
    /// subx sync video.mp4 subtitle.srt --offset 2.5
    ///
    /// # Advance subtitles by 1 second
    /// subx sync video.mp4 subtitle.srt --offset -1.0
    ///
    /// # Fine adjustment by 300ms
    /// subx sync video.mp4 subtitle.srt --offset 0.3
    /// ```
    #[arg(long)]
    pub offset: Option<f64>,

    /// Enable batch processing mode for multiple file pairs.
    ///
    /// When enabled, optimizes processing for handling multiple video-subtitle
    /// pairs efficiently. This mode provides enhanced performance and
    /// consistent parameters across all processed files.
    ///
    /// # Batch Mode Features
    /// - **Parallel processing**: Multiple files processed simultaneously
    /// - **Consistent parameters**: Same sync settings for all files
    /// - **Progress tracking**: Overall progress across all files
    /// - **Error resilience**: Continues processing if individual files fail
    ///
    /// # File Discovery
    /// In batch mode, the command can automatically discover matching pairs:
    /// - Video and subtitle files with same base name
    /// - Common naming patterns (e.g., movie.mp4 + movie.srt)
    /// - Multiple subtitle languages (e.g., movie.en.srt, movie.es.srt)
    ///
    /// # Examples
    /// ```bash
    /// # Process multiple files in directory
    /// subx sync --batch /path/to/videos/ /path/to/subtitles/
    ///
    /// # Batch with custom parameters
    /// subx sync --batch --range 20.0 --threshold 0.85 videos/ subs/
    /// ```
    #[arg(long)]
    pub batch: bool,

    /// Maximum offset detection range in seconds.
    ///
    /// Defines the maximum time range (both positive and negative) within
    /// which the automatic synchronization algorithm will search for the
    /// optimal offset. This parameter balances detection accuracy with
    /// processing time.
    ///
    /// # Default Behavior
    /// If not specified, uses the value from configuration file
    /// (`max_offset_seconds`). Common defaults are 10-30 seconds depending
    /// on content type and expected synchronization accuracy.
    ///
    /// # Recommendations
    /// - **Precise timing**: 5-10 seconds for high-quality sources
    /// - **Standard content**: 10-20 seconds for most videos
    /// - **Problematic sync**: 30-60 seconds for heavily offset content
    /// - **Performance priority**: 5-15 seconds for faster processing
    ///
    /// # Trade-offs
    /// - **Larger range**: Higher chance of finding correct offset, slower processing
    /// - **Smaller range**: Faster processing, may miss large offsets
    ///
    /// # Examples
    /// ```bash
    /// # High precision with smaller range
    /// subx sync video.mp4 subtitle.srt --range 5.0
    ///
    /// # Handle large offsets
    /// subx sync video.mp4 subtitle.srt --range 60.0
    ///
    /// # Balanced approach
    /// subx sync video.mp4 subtitle.srt --range 15.0
    /// ```
    #[arg(long)]
    pub range: Option<f32>,

    /// Correlation threshold for automatic synchronization (0.0-1.0).
    ///
    /// Sets the minimum correlation coefficient required between audio
    /// speech patterns and subtitle timing for a synchronization to be
    /// considered successful. Higher values require stronger correlation
    /// but provide more reliable results.
    ///
    /// # Scale Interpretation
    /// - **0.9-1.0**: Excellent correlation (very reliable)
    /// - **0.8-0.9**: Good correlation (reliable for most content)
    /// - **0.7-0.8**: Acceptable correlation (may need verification)
    /// - **0.6-0.7**: Weak correlation (results questionable)
    /// - **Below 0.6**: Poor correlation (likely incorrect sync)
    ///
    /// # Configuration Override
    /// If not specified, uses the value from configuration file
    /// (`correlation_threshold`). This allows consistent behavior
    /// across different synchronization operations.
    ///
    /// # Content Type Recommendations
    /// - **Dialog-heavy content**: 0.8-0.9 (speech patterns clear)
    /// - **Action/music-heavy**: 0.7-0.8 (speech less prominent)
    /// - **Documentary/interview**: 0.85-0.95 (clear speech patterns)
    /// - **Animated content**: 0.75-0.85 (consistent voice patterns)
    ///
    /// # Examples
    /// ```bash
    /// # High precision requirement
    /// subx sync video.mp4 subtitle.srt --threshold 0.9
    ///
    /// # More permissive for difficult content
    /// subx sync video.mp4 subtitle.srt --threshold 0.7
    ///
    /// # Balanced approach
    /// subx sync video.mp4 subtitle.srt --threshold 0.8
    /// ```
    #[arg(long)]
    pub threshold: Option<f32>,
}

/// Synchronization method enumeration.
///
/// Defines the approach used for subtitle-audio alignment based on
/// the provided command-line arguments. The method is automatically
/// determined by the presence of manual offset parameters.
///
/// # Method Selection
/// - **Manual**: When `--offset` parameter is provided
/// - **Auto**: When no offset is specified (default behavior)
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::{SyncArgs, SyncMethod};
/// use std::path::PathBuf;
///
/// // Auto sync (no offset specified)
/// let auto_args = SyncArgs {
///     video: Some(PathBuf::from("video.mp4")),
///     subtitle: PathBuf::from("subtitle.srt"),
///     offset: None,
///     batch: false,
///     range: None,
///     threshold: None,
/// };
/// assert_eq!(auto_args.sync_method(), SyncMethod::Auto);
///
/// // Manual sync (offset specified)
/// let manual_args = SyncArgs {
///     video: Some(PathBuf::from("video.mp4")),
///     subtitle: PathBuf::from("subtitle.srt"),
///     offset: Some(2.5),
///     batch: false,
///     range: None,
///     threshold: None,
/// };
/// assert_eq!(manual_args.sync_method(), SyncMethod::Manual);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMethod {
    /// Automatic synchronization using audio analysis.
    ///
    /// Performs sophisticated audio-subtitle correlation analysis:
    /// 1. **Audio extraction**: Extract audio track from video
    /// 2. **Speech detection**: Identify speech segments and pauses
    /// 3. **Pattern analysis**: Create timing fingerprint
    /// 4. **Correlation**: Match patterns with subtitle timing
    /// 5. **Optimization**: Find best offset within specified range
    Auto,

    /// Manual synchronization using specified time offset.
    ///
    /// Applies a fixed time shift to all subtitle entries:
    /// - Simple and fast operation
    /// - Preserves relative timing between subtitles
    /// - Useful for known timing differences
    /// - No audio analysis required
    Manual,
}

impl SyncArgs {
    /// Determines the synchronization method based on provided arguments.
    ///
    /// This method automatically selects between manual and automatic
    /// synchronization based on whether a manual offset was specified.
    /// It provides a convenient way to branch synchronization logic.
    ///
    /// # Logic
    /// - Returns `SyncMethod::Manual` if `offset` field is `Some(value)`
    /// - Returns `SyncMethod::Auto` if `offset` field is `None`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::cli::{SyncArgs, SyncMethod};
    /// use std::path::PathBuf;
    ///
    /// let args = SyncArgs {
    ///     video: Some(PathBuf::from("video.mp4")),
    ///     subtitle: PathBuf::from("subtitle.srt"),
    ///     offset: Some(1.5),
    ///     batch: false,
    ///     range: None,
    ///     threshold: None,
    /// };
    ///
    /// match args.sync_method() {
    ///     SyncMethod::Manual => println!("Using manual offset: {}", args.offset.unwrap()),
    ///     SyncMethod::Auto => println!("Using automatic synchronization"),
    /// }
    /// ```
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }

    /// Validate SyncArgs combinations for manual or automatic modes.
    pub fn validate(&self) -> SubXResult<()> {
        match (self.offset.is_some(), self.video.is_some()) {
            // Manual mode: offset provided, video optional
            (true, _) => Ok(()),
            // Automatic mode: offset absent, video required
            (false, true) => Ok(()),
            // Automatic mode without video: invalid
            (false, false) => Err(SubXError::CommandExecution(
                "視頻檔案在自動同步模式下是必填的。\n\n\
使用方式:\n\
• 自動同步: subx-cli sync <video> <subtitle>\n\
• 手動同步: subx-cli sync --offset <seconds> <subtitle>\n\n\
需要幫助嗎？執行: subx-cli sync --help"
                    .to_string(),
            )),
        }
    }

    /// Returns true if video file is required (automatic sync).
    #[allow(dead_code)]
    pub fn requires_video(&self) -> bool {
        self.offset.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: Some(2.5),
            batch: false,
            range: None,
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: PathBuf::from("subtitle.srt"),
            offset: None,
            batch: false,
            range: None,
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Auto);
    }
}
