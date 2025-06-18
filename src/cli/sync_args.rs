//! Refactored sync command CLI argument definitions.
//!
//! Supports multiple synchronization methods: OpenAI Whisper API, local VAD,
//! automatic selection, and manual offset. Provides fine-grained parameter
//! control and intelligent defaults.
//!
//! # Synchronization Methods
//!
//! ## OpenAI Whisper API
//! Cloud transcription service providing high-precision speech detection.
//!
//! ## Local VAD
//! Voice Activity Detection using local processing for privacy and speed.
//!
//! ## Manual Offset
//! Direct time offset specification for precise manual synchronization.

/// Synchronization method selection for CLI arguments.
#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum SyncMethodArg {
    /// Use local voice activity detection only
    Vad,
    /// Apply manual offset (requires --offset parameter)
    Manual,
}

impl From<SyncMethodArg> for crate::core::sync::SyncMethod {
    fn from(arg: SyncMethodArg) -> Self {
        match arg {
            SyncMethodArg::Vad => Self::LocalVad,
            SyncMethodArg::Manual => Self::Manual,
        }
    }
}

use crate::cli::InputPathHandler;
use crate::error::{SubXError, SubXResult};
use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};

/// Refactored sync command arguments supporting multiple sync methods.
#[derive(Args, Debug, Clone)]
pub struct SyncArgs {
    /// Video file path for audio analysis.
    #[arg(
        short = 'v',
        long = "video",
        value_name = "VIDEO",
        help = "Video file path (required for auto sync, optional for manual offset)",
        required_unless_present = "offset"
    )]
    pub video: Option<PathBuf>,

    /// Subtitle file path to synchronize.
    #[arg(
        short = 's',
        long = "subtitle",
        value_name = "SUBTITLE",
        help = "Subtitle file path (required for single file, optional for batch mode)",
        required_unless_present_any = ["input_paths", "batch"]
    )]
    pub subtitle: Option<PathBuf>,
    /// Specify file or directory paths to process (new parameter), can be used multiple times
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Recursively process subdirectories (new parameter)
    #[arg(short, long)]
    pub recursive: bool,

    /// Manual time offset in seconds (positive delays subtitles, negative advances them).
    #[arg(
        long,
        value_name = "SECONDS",
        help = "Manual offset in seconds (positive delays subtitles, negative advances them)",
        conflicts_with_all = ["method", "window", "vad_sensitivity"]
    )]
    pub offset: Option<f32>,

    /// Sync method selection.
    #[arg(short, long, value_enum, help = "Synchronization method")]
    pub method: Option<SyncMethodArg>,

    /// Analysis time window in seconds.
    #[arg(
        short = 'w',
        long,
        value_name = "SECONDS",
        default_value = "30",
        help = "Time window around first subtitle for analysis (seconds)"
    )]
    pub window: u32,

    // === VAD Options ===
    /// VAD sensitivity threshold.
    #[arg(
        long,
        value_name = "SENSITIVITY",
        help = "VAD sensitivity threshold (0.0-1.0)"
    )]
    pub vad_sensitivity: Option<f32>,

    // === Output Options ===
    /// Output file path.
    #[arg(
        short = 'o',
        long,
        value_name = "PATH",
        help = "Output file path (default: input_synced.ext)"
    )]
    pub output: Option<PathBuf>,

    /// Verbose output.
    #[arg(
        long,
        help = "Enable verbose output with detailed progress information"
    )]
    pub verbose: bool,

    /// Dry run mode.
    #[arg(long, help = "Analyze and display results but don't save output file")]
    pub dry_run: bool,

    /// Force overwrite existing output file.
    #[arg(long, help = "Overwrite existing output file without confirmation")]
    pub force: bool,

    /// Enable batch processing mode.
    #[arg(short, long, help = "Enable batch processing mode")]
    pub batch: bool,

    // === Legacy/Hidden Options (Deprecated) ===
    /// Maximum offset search range (deprecated, use configuration file).
    #[arg(long, hide = true)]
    #[deprecated(note = "Use configuration file instead")]
    pub range: Option<f32>,

    /// Minimum correlation threshold (deprecated, use configuration file).
    #[arg(long, hide = true)]
    #[deprecated(note = "Use configuration file instead")]
    pub threshold: Option<f32>,
}

/// Sync method enumeration (backward compatible).
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMethod {
    /// Automatic sync using audio analysis.
    Auto,
    /// Manual sync using specified time offset.
    Manual,
}

impl SyncArgs {
    /// Validate parameter combination validity.
    pub fn validate(&self) -> Result<(), String> {
        // Check manual mode parameter combination
        if let Some(SyncMethodArg::Manual) = &self.method {
            if self.offset.is_none() {
                return Err("Manual method requires --offset parameter.".to_string());
            }
        }

        // Check auto mode requires video file
        if self.offset.is_none() && self.video.is_none() {
            return Err("Auto sync mode requires video file.\n\n\
Usage:\n\
• Auto sync: subx sync <video> <subtitle>\n\
• Manual sync: subx sync --offset <seconds> <subtitle>\n\n\
Need help? Run: subx sync --help"
                .to_string());
        }

        // Check VAD sensitivity option only used with VAD method
        if self.vad_sensitivity.is_some() {
            match &self.method {
                Some(SyncMethodArg::Vad) => {}
                _ => return Err("VAD options can only be used with --method vad.".to_string()),
            }
        }

        Ok(())
    }

    /// Get output file path.
    pub fn get_output_path(&self) -> Option<PathBuf> {
        if let Some(ref output) = self.output {
            Some(output.clone())
        } else {
            self.subtitle
                .as_ref()
                .map(|subtitle| create_default_output_path(subtitle))
        }
    }

    /// Check if in manual mode.
    pub fn is_manual_mode(&self) -> bool {
        self.offset.is_some() || matches!(self.method, Some(SyncMethodArg::Manual))
    }

    /// Determine sync method (backward compatible).
    pub fn sync_method(&self) -> SyncMethod {
        if self.offset.is_some() {
            SyncMethod::Manual
        } else {
            SyncMethod::Auto
        }
    }

    /// Validate parameters (backward compatible method).
    pub fn validate_compat(&self) -> SubXResult<()> {
        match (self.offset.is_some(), self.video.is_some()) {
            // Manual mode: offset provided, video optional
            (true, _) => Ok(()),
            // Auto mode: no offset, video required
            (false, true) => Ok(()),
            // Auto mode without video: invalid
            (false, false) => Err(SubXError::CommandExecution(
                "Auto sync mode requires video file.\n\n\
Usage:\n\
• Auto sync: subx sync <video> <subtitle>\n\
• Manual sync: subx sync --offset <seconds> <subtitle>\n\n\
Need help? Run: subx sync --help"
                    .to_string(),
            )),
        }
    }

    /// Return whether video file is required (auto sync).
    #[allow(dead_code)]
    pub fn requires_video(&self) -> bool {
        self.offset.is_none()
    }

    /// Get all input paths, combining video, subtitle and input_paths parameters
    /// Note: For sync command, both video and subtitle are valid input paths
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let optional_paths = vec![self.video.clone(), self.subtitle.clone()];
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &optional_paths,
            &self.input_paths,
            &[],
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]))
    }

    /// Get sync mode: single file or batch
    pub fn get_sync_mode(&self) -> Result<SyncMode, SubXError> {
        if !self.input_paths.is_empty() || self.batch {
            let paths = if !self.input_paths.is_empty() {
                self.input_paths.clone()
            } else if let Some(video) = &self.video {
                vec![video.clone()]
            } else {
                return Err(SubXError::NoInputSpecified);
            };

            let handler = InputPathHandler::from_args(&paths, self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]);

            Ok(SyncMode::Batch(handler))
        } else if let (Some(video), Some(subtitle)) = (self.video.as_ref(), self.subtitle.as_ref())
        {
            Ok(SyncMode::Single {
                video: video.clone(),
                subtitle: subtitle.clone(),
            })
        } else if let Some(subtitle) = self.subtitle.as_ref() {
            // Manual mode only requires subtitle file
            if self.offset.is_some() || matches!(self.method, Some(SyncMethodArg::Manual)) {
                // Create virtual video path for manual mode
                Ok(SyncMode::Single {
                    video: PathBuf::from(""), // Virtual video path, won't be used
                    subtitle: subtitle.clone(),
                })
            } else {
                Err(SubXError::InvalidSyncConfiguration)
            }
        } else {
            Err(SubXError::InvalidSyncConfiguration)
        }
    }
}

/// Sync mode: single file or batch
#[derive(Debug)]
pub enum SyncMode {
    /// Single file sync mode, specify video and subtitle files
    Single {
        /// Video file path
        video: PathBuf,
        /// Subtitle file path
        subtitle: PathBuf,
    },
    /// Batch sync mode, using InputPathHandler to process multiple paths
    Batch(InputPathHandler),
}

// Helper functions

fn create_default_output_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();

    if let Some(stem) = input.file_stem().and_then(|s| s.to_str()) {
        if let Some(extension) = input.extension().and_then(|s| s.to_str()) {
            let new_filename = format!("{}_synced.{}", stem, extension);
            output.set_file_name(new_filename);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: Some(PathBuf::from("subtitle.srt")),
            input_paths: Vec::new(),
            recursive: false,
            offset: Some(2.5),
            method: None,
            window: 30,
            vad_sensitivity: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_args_batch_input() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "-i",
            "dir",
            "--batch",
            "--recursive",
            "--video",
            "video.mp4",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.input_paths, vec![PathBuf::from("dir")]);
        assert!(args.batch);
        assert!(args.recursive);
        assert_eq!(args.video, Some(PathBuf::from("video.mp4")));
    }

    #[test]
    fn test_sync_args_invalid_combinations() {
        // batch mode requires video parameter
        let res = Cli::try_parse_from(["subx-cli", "sync", "--batch", "-i", "dir"]);
        assert!(res.is_err());
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: Some(PathBuf::from("subtitle.srt")),
            input_paths: Vec::new(),
            recursive: false,
            offset: None,
            method: None,
            window: 30,
            vad_sensitivity: None,
            output: None,
            verbose: false,
            dry_run: false,
            force: false,
            batch: false,
            #[allow(deprecated)]
            range: None,
            #[allow(deprecated)]
            threshold: None,
        };
        assert_eq!(args.sync_method(), SyncMethod::Auto);
    }

    #[test]
    fn test_method_arg_conversion() {
        assert_eq!(
            crate::core::sync::SyncMethod::from(SyncMethodArg::Vad),
            crate::core::sync::SyncMethod::LocalVad
        );
        assert_eq!(
            crate::core::sync::SyncMethod::from(SyncMethodArg::Manual),
            crate::core::sync::SyncMethod::Manual
        );
    }

    #[test]
    fn test_create_default_output_path() {
        let input = PathBuf::from("test.srt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "test_synced.srt");

        let input = PathBuf::from("/path/to/movie.ass");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "movie_synced.ass");
    }
}
