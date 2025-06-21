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
    /// Positional file or directory paths to process. Can include video, subtitle, or directories.
    #[arg(value_name = "PATH", num_args = 0..)]
    pub positional_paths: Vec<PathBuf>,

    /// Video file path (optional if using positional paths or manual offset).
    #[arg(
        short = 'v',
        long = "video",
        value_name = "VIDEO",
        help = "Video file path (optional if using positional or manual offset)"
    )]
    pub video: Option<PathBuf>,

    /// Subtitle file path (optional if using positional paths or manual offset).
    #[arg(
        short = 's',
        long = "subtitle",
        value_name = "SUBTITLE",
        help = "Subtitle file path (optional if using positional or manual offset)"
    )]
    pub subtitle: Option<PathBuf>,
    /// Specify file or directory paths to process (via -i), can be used multiple times
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Recursively process subdirectories (new parameter)
    #[arg(short, long)]
    pub recursive: bool,

    /// Manual time offset in seconds (positive delays subtitles, negative advances them).
    #[arg(
        long,
        value_name = "SECONDS",
        help = "Manual offset in seconds (positive delays subtitles, negative advances them)"
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

    /// Enable batch processing mode. Can optionally specify a directory path.
    #[arg(
        short = 'b',
        long = "batch",
        value_name = "DIRECTORY",
        help = "Enable batch processing mode. Can optionally specify a directory path.",
        num_args = 0..=1,
        require_equals = false
    )]
    pub batch: Option<Option<PathBuf>>,
    // === Legacy/Hidden Options (Deprecated) ===
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

        // In batch mode, check if we have some input source
        if self.batch.is_some() {
            let has_input_paths = !self.input_paths.is_empty();
            let has_positional = !self.positional_paths.is_empty();
            let has_video_or_subtitle = self.video.is_some() || self.subtitle.is_some();
            let has_batch_directory = matches!(&self.batch, Some(Some(_)));

            // Batch mode needs at least one input source
            if has_input_paths || has_positional || has_video_or_subtitle || has_batch_directory {
                return Ok(());
            }

            return Err("Batch mode requires at least one input source.\n\n\
Usage:\n\
• Batch with directory: subx sync -b <directory>\n\
• Batch with input paths: subx sync -b -i <path>\n\
• Batch with positional: subx sync -b <path>\n\n\
Need help? Run: subx sync --help"
                .to_string());
        }

        // For single file mode, check if we have enough information
        let has_video = self.video.is_some();
        let has_subtitle = self.subtitle.is_some();
        let has_positional = !self.positional_paths.is_empty();
        let is_manual = self.offset.is_some();

        // Manual mode only requires subtitle (can be provided via positional args)
        if is_manual {
            if has_subtitle || has_positional {
                return Ok(());
            }
            return Err("Manual sync mode requires subtitle file.\n\n\
Usage:\n\
• Manual sync: subx sync --offset <seconds> <subtitle>\n\
• Manual sync: subx sync --offset <seconds> -s <subtitle>\n\n\
Need help? Run: subx sync --help"
                .to_string());
        }

        // Auto mode: needs video, or positional args
        if has_video || has_positional {
            // Check VAD sensitivity option only used with VAD method
            if self.vad_sensitivity.is_some() {
                if let Some(SyncMethodArg::Manual) = &self.method {
                    return Err("VAD options can only be used with --method vad.".to_string());
                }
            }
            return Ok(());
        }

        Err("Auto sync mode requires video file or positional path.\n\n\
Usage:\n\
• Auto sync: subx sync <video> <subtitle> or subx sync <video_path>\n\
• Auto sync: subx sync -v <video> -s <subtitle>\n\
• Manual sync: subx sync --offset <seconds> <subtitle>\n\
• Batch mode: subx sync -b [directory]\n\n\
Need help? Run: subx sync --help"
            .to_string())
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
        // Allow positional path for auto sync without explicit video
        if self.offset.is_none() && self.video.is_none() && !self.positional_paths.is_empty() {
            return Ok(());
        }
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
        let string_paths: Vec<String> = self
            .positional_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &optional_paths,
            &self.input_paths,
            &string_paths,
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]))
    }

    /// Get sync mode: single file or batch
    pub fn get_sync_mode(&self) -> Result<SyncMode, SubXError> {
        // Batch mode: process directories or multiple inputs when -b, -i, or directory positional used
        if self.batch.is_some()
            || !self.input_paths.is_empty()
            || self
                .positional_paths
                .iter()
                .any(|p| p.extension().is_none())
        {
            let mut paths = Vec::new();

            // Include batch directory argument if provided
            if let Some(Some(batch_dir)) = &self.batch {
                paths.push(batch_dir.clone());
            }

            // Include input paths (-i) and any positional paths
            paths.extend(self.input_paths.clone());
            paths.extend(self.positional_paths.clone());

            // If still no paths, use current directory
            if paths.is_empty() {
                paths.push(PathBuf::from("."));
            }

            let handler = InputPathHandler::from_args(&paths, self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]);

            return Ok(SyncMode::Batch(handler));
        }

        // Single file positional mode: auto-infer video/subtitle pairing
        if !self.positional_paths.is_empty() {
            if self.positional_paths.len() == 1 {
                let path = &self.positional_paths[0];
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let mut video = None;
                let mut subtitle = None;
                match ext.as_str() {
                    "mp4" | "mkv" | "avi" | "mov" => {
                        video = Some(path.clone());
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let dir = path.parent().unwrap_or_else(|| Path::new("."));
                            for sub_ext in &["srt", "ass", "vtt", "sub"] {
                                let cand = dir.join(format!("{}.{}", stem, sub_ext));
                                if cand.exists() {
                                    subtitle = Some(cand);
                                    break;
                                }
                            }
                        }
                    }
                    "srt" | "ass" | "vtt" | "sub" => {
                        subtitle = Some(path.clone());
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let dir = path.parent().unwrap_or_else(|| Path::new("."));
                            for vid_ext in &["mp4", "mkv", "avi", "mov"] {
                                let cand = dir.join(format!("{}.{}", stem, vid_ext));
                                if cand.exists() {
                                    video = Some(cand);
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                // For manual mode, we don't need video file if we have subtitle
                if self.is_manual_mode() {
                    if let Some(subtitle_path) = subtitle {
                        return Ok(SyncMode::Single {
                            video: PathBuf::new(), // Empty video path for manual mode
                            subtitle: subtitle_path,
                        });
                    }
                }
                if let (Some(v), Some(s)) = (video, subtitle) {
                    return Ok(SyncMode::Single {
                        video: v,
                        subtitle: s,
                    });
                }
                return Err(SubXError::InvalidSyncConfiguration);
            } else if self.positional_paths.len() == 2 {
                let mut video = None;
                let mut subtitle = None;
                for p in &self.positional_paths {
                    if let Some(ext) = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase())
                    {
                        if ["mp4", "mkv", "avi", "mov"].contains(&ext.as_str()) {
                            video = Some(p.clone());
                        }
                        if ["srt", "ass", "vtt", "sub"].contains(&ext.as_str()) {
                            subtitle = Some(p.clone());
                        }
                    }
                }
                if let (Some(v), Some(s)) = (video, subtitle) {
                    return Ok(SyncMode::Single {
                        video: v,
                        subtitle: s,
                    });
                }
                return Err(SubXError::InvalidSyncConfiguration);
            }
        }

        // Explicit mode: video and subtitle options
        if let (Some(video), Some(subtitle)) = (self.video.as_ref(), self.subtitle.as_ref()) {
            Ok(SyncMode::Single {
                video: video.clone(),
                subtitle: subtitle.clone(),
            })
        } else if self.is_manual_mode() && self.subtitle.is_some() {
            // Manual mode only requires subtitle file
            Ok(SyncMode::Single {
                video: PathBuf::new(), // Empty video path for manual mode
                subtitle: self.subtitle.as_ref().unwrap().clone(),
            })
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
            positional_paths: Vec::new(),
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
            batch: None,
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
        assert!(args.batch.is_some());
        assert!(args.recursive);
        assert_eq!(args.video, Some(PathBuf::from("video.mp4")));
    }

    #[test]
    fn test_sync_args_invalid_combinations() {
        // batch mode with input paths should be valid now
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--batch", "-i", "dir"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };

        // This should now be valid
        assert!(args.validate().is_ok());

        // Test a truly invalid combination: batch mode with no input sources
        let args_invalid = SyncArgs {
            positional_paths: Vec::new(),
            video: None,
            subtitle: None,
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
            batch: Some(None), // batch mode but no inputs
        };

        assert!(args_invalid.validate().is_err());
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            positional_paths: Vec::new(),
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
            batch: None,
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
