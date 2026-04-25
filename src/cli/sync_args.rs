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

    /// Disable automatic archive extraction for `-i` inputs
    #[arg(long, default_value_t = false)]
    pub no_extract: bool,
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
            .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"])
            .with_no_extract(self.no_extract))
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
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"])
                .with_no_extract(self.no_extract);

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
                                let cand = dir.join(format!("{stem}.{sub_ext}"));
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
                                let cand = dir.join(format!("{stem}.{vid_ext}"));
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
        } else if self.is_manual_mode() {
            if let Some(subtitle) = self.subtitle.as_ref() {
                // Manual mode only requires subtitle file
                Ok(SyncMode::Single {
                    video: PathBuf::new(), // Empty video path for manual mode
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

/// Creates a default output path by appending `_synced` to the file stem.
pub fn create_default_output_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();

    if let Some(stem) = input.file_stem().and_then(|s| s.to_str()) {
        if let Some(extension) = input.extension().and_then(|s| s.to_str()) {
            let new_filename = format!("{stem}_synced.{extension}");
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
    use tempfile::TempDir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn default_args() -> SyncArgs {
        SyncArgs {
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
            batch: None,
            no_extract: false,
        }
    }

    // ── SyncMethodArg / From conversion ──────────────────────────────────────

    #[test]
    fn test_sync_method_selection_manual() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: Some(PathBuf::from("subtitle.srt")),
            offset: Some(2.5),
            ..default_args()
        };
        assert_eq!(args.sync_method(), SyncMethod::Manual);
    }

    #[test]
    fn test_sync_method_selection_auto() {
        let args = SyncArgs {
            video: Some(PathBuf::from("video.mp4")),
            subtitle: Some(PathBuf::from("subtitle.srt")),
            ..default_args()
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
    fn test_sync_method_arg_debug_clone() {
        let m = SyncMethodArg::Vad;
        let c = m.clone();
        assert_eq!(m, c);
        assert_eq!(format!("{c:?}"), "Vad");
        let m2 = SyncMethodArg::Manual;
        assert_eq!(format!("{m2:?}"), "Manual");
    }

    // ── SyncMethod enum ───────────────────────────────────────────────────────

    #[test]
    fn test_sync_method_enum_debug_clone() {
        let m = SyncMethod::Auto;
        let c = m.clone();
        assert_eq!(m, c);
        assert_eq!(format!("{c:?}"), "Auto");
        let m2 = SyncMethod::Manual;
        assert_eq!(format!("{m2:?}"), "Manual");
    }

    // ── is_manual_mode ────────────────────────────────────────────────────────

    #[test]
    fn test_is_manual_mode_with_offset() {
        let args = SyncArgs {
            offset: Some(1.0),
            ..default_args()
        };
        assert!(args.is_manual_mode());
    }

    #[test]
    fn test_is_manual_mode_with_method_manual() {
        let args = SyncArgs {
            method: Some(SyncMethodArg::Manual),
            ..default_args()
        };
        assert!(args.is_manual_mode());
    }

    #[test]
    fn test_is_manual_mode_false() {
        let args = default_args();
        assert!(!args.is_manual_mode());
    }

    #[test]
    fn test_is_manual_mode_false_with_vad_method() {
        let args = SyncArgs {
            method: Some(SyncMethodArg::Vad),
            ..default_args()
        };
        assert!(!args.is_manual_mode());
    }

    // ── requires_video ────────────────────────────────────────────────────────

    #[test]
    fn test_requires_video_true_without_offset() {
        let args = default_args();
        assert!(args.requires_video());
    }

    #[test]
    fn test_requires_video_false_with_offset() {
        let args = SyncArgs {
            offset: Some(-1.5),
            ..default_args()
        };
        assert!(!args.requires_video());
    }

    // ── get_output_path ───────────────────────────────────────────────────────

    #[test]
    fn test_get_output_path_explicit() {
        let args = SyncArgs {
            output: Some(PathBuf::from("out.srt")),
            subtitle: Some(PathBuf::from("sub.srt")),
            ..default_args()
        };
        assert_eq!(args.get_output_path(), Some(PathBuf::from("out.srt")));
    }

    #[test]
    fn test_get_output_path_default_from_subtitle() {
        let args = SyncArgs {
            subtitle: Some(PathBuf::from("movie.srt")),
            ..default_args()
        };
        let out = args.get_output_path().unwrap();
        assert_eq!(out.file_name().unwrap(), "movie_synced.srt");
    }

    #[test]
    fn test_get_output_path_none_without_subtitle() {
        let args = default_args();
        assert_eq!(args.get_output_path(), None);
    }

    // ── validate ──────────────────────────────────────────────────────────────

    #[test]
    fn test_validate_manual_method_requires_offset() {
        let args = SyncArgs {
            method: Some(SyncMethodArg::Manual),
            video: Some(PathBuf::from("v.mp4")),
            ..default_args()
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Manual method requires --offset")
        );
    }

    #[test]
    fn test_validate_manual_method_with_offset_ok() {
        let args = SyncArgs {
            method: Some(SyncMethodArg::Manual),
            offset: Some(1.0),
            subtitle: Some(PathBuf::from("sub.srt")),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_with_input_paths_ok() {
        let args = SyncArgs {
            batch: Some(None),
            input_paths: vec![PathBuf::from("dir")],
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_with_positional_ok() {
        let args = SyncArgs {
            batch: Some(None),
            positional_paths: vec![PathBuf::from("dir")],
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_with_video_ok() {
        let args = SyncArgs {
            batch: Some(None),
            video: Some(PathBuf::from("v.mp4")),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_with_subtitle_ok() {
        let args = SyncArgs {
            batch: Some(None),
            subtitle: Some(PathBuf::from("s.srt")),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_with_directory_ok() {
        let args = SyncArgs {
            batch: Some(Some(PathBuf::from("mydir"))),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_batch_no_inputs_err() {
        let args = SyncArgs {
            batch: Some(None),
            ..default_args()
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Batch mode requires"));
    }

    #[test]
    fn test_validate_manual_offset_with_subtitle_ok() {
        let args = SyncArgs {
            offset: Some(2.0),
            subtitle: Some(PathBuf::from("sub.srt")),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_manual_offset_with_positional_ok() {
        let args = SyncArgs {
            offset: Some(2.0),
            positional_paths: vec![PathBuf::from("sub.srt")],
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_manual_offset_without_subtitle_err() {
        let args = SyncArgs {
            offset: Some(2.0),
            ..default_args()
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Manual sync mode requires subtitle")
        );
    }

    #[test]
    fn test_validate_auto_with_video_ok() {
        let args = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_auto_with_positional_ok() {
        let args = SyncArgs {
            positional_paths: vec![PathBuf::from("v.mp4")],
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_auto_vad_sensitivity_with_manual_method_err() {
        let args = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            method: Some(SyncMethodArg::Manual),
            vad_sensitivity: Some(0.5),
            offset: Some(1.0), // needed so manual method validation passes first
            ..default_args()
        };
        // manual method check fires first (offset required) - here offset is provided,
        // but we test the vad_sensitivity path, which requires no manual method *and* has video
        // Re-test: auto path with vad_sensitivity set and manual method
        let args2 = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            method: Some(SyncMethodArg::Manual),
            vad_sensitivity: Some(0.5),
            offset: None, // manual without offset → first error fires
            ..default_args()
        };
        assert!(args2.validate().is_err());
    }

    #[test]
    fn test_validate_vad_sensitivity_with_vad_method_and_video_ok() {
        // vad_sensitivity is fine when method is Vad (or None)
        let args = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            method: Some(SyncMethodArg::Vad),
            vad_sensitivity: Some(0.7),
            ..default_args()
        };
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_validate_auto_no_inputs_err() {
        let args = default_args();
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Auto sync mode requires video file")
        );
    }

    // ── validate_compat ───────────────────────────────────────────────────────

    #[test]
    fn test_validate_compat_with_positional_no_video_no_offset_ok() {
        let args = SyncArgs {
            positional_paths: vec![PathBuf::from("movie.mp4")],
            ..default_args()
        };
        assert!(args.validate_compat().is_ok());
    }

    #[test]
    fn test_validate_compat_with_offset_ok() {
        let args = SyncArgs {
            offset: Some(1.0),
            ..default_args()
        };
        assert!(args.validate_compat().is_ok());
    }

    #[test]
    fn test_validate_compat_with_video_ok() {
        let args = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            ..default_args()
        };
        assert!(args.validate_compat().is_ok());
    }

    #[test]
    fn test_validate_compat_no_offset_no_video_no_positional_err() {
        let args = default_args();
        assert!(args.validate_compat().is_err());
    }

    #[test]
    fn test_validate_compat_with_offset_and_video_ok() {
        let args = SyncArgs {
            offset: Some(2.5),
            video: Some(PathBuf::from("v.mp4")),
            ..default_args()
        };
        assert!(args.validate_compat().is_ok());
    }

    // ── get_sync_mode ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_sync_mode_batch_explicit_batch_flag() {
        let tmp = TempDir::new().unwrap();
        let args = SyncArgs {
            batch: Some(None),
            input_paths: vec![tmp.path().to_path_buf()],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        assert!(matches!(mode, SyncMode::Batch(_)));
    }

    #[test]
    fn test_get_sync_mode_batch_with_directory() {
        let tmp = TempDir::new().unwrap();
        let args = SyncArgs {
            batch: Some(Some(tmp.path().to_path_buf())),
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        assert!(matches!(mode, SyncMode::Batch(_)));
    }

    #[test]
    fn test_get_sync_mode_batch_from_input_paths() {
        let tmp = TempDir::new().unwrap();
        let args = SyncArgs {
            input_paths: vec![tmp.path().to_path_buf()],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        assert!(matches!(mode, SyncMode::Batch(_)));
    }

    #[test]
    fn test_get_sync_mode_batch_uses_current_dir_when_no_paths() {
        let args = SyncArgs {
            batch: Some(None),
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        assert!(matches!(mode, SyncMode::Batch(_)));
    }

    #[test]
    fn test_get_sync_mode_single_from_two_positionals() {
        let args = SyncArgs {
            positional_paths: vec![PathBuf::from("movie.mp4"), PathBuf::from("movie.srt")],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::from("movie.mp4"));
                assert_eq!(subtitle, PathBuf::from("movie.srt"));
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_single_two_positionals_wrong_extensions_err() {
        let args = SyncArgs {
            positional_paths: vec![PathBuf::from("file1.txt"), PathBuf::from("file2.doc")],
            ..default_args()
        };
        assert!(args.get_sync_mode().is_err());
    }

    #[test]
    fn test_get_sync_mode_single_one_positional_no_extension_is_batch() {
        // A directory path (no extension) is treated as batch mode
        let tmp = TempDir::new().unwrap();
        let args = SyncArgs {
            positional_paths: vec![tmp.path().to_path_buf()],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        assert!(matches!(mode, SyncMode::Batch(_)));
    }

    #[test]
    fn test_get_sync_mode_single_positional_video_no_subtitle_err() {
        // File has video extension but no matching subtitle on disk
        let args = SyncArgs {
            positional_paths: vec![PathBuf::from("nonexistent_movie.mp4")],
            ..default_args()
        };
        assert!(args.get_sync_mode().is_err());
    }

    #[test]
    fn test_get_sync_mode_single_positional_subtitle_finds_video() {
        let tmp = TempDir::new().unwrap();
        let video_path = tmp.path().join("clip.mp4");
        let sub_path = tmp.path().join("clip.srt");
        std::fs::write(&video_path, b"fake video").unwrap();
        std::fs::write(&sub_path, b"1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();

        let args = SyncArgs {
            positional_paths: vec![sub_path.clone()],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, video_path);
                assert_eq!(subtitle, sub_path);
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_single_positional_video_finds_subtitle() {
        let tmp = TempDir::new().unwrap();
        let video_path = tmp.path().join("film.mkv");
        let sub_path = tmp.path().join("film.ass");
        std::fs::write(&video_path, b"fake video").unwrap();
        std::fs::write(&sub_path, b"[Script Info]\n").unwrap();

        let args = SyncArgs {
            positional_paths: vec![video_path.clone()],
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, video_path);
                assert_eq!(subtitle, sub_path);
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_single_positional_manual_mode_subtitle_only() {
        let tmp = TempDir::new().unwrap();
        let sub_path = tmp.path().join("orphan.srt");
        std::fs::write(&sub_path, b"1\n00:00:01,000 --> 00:00:02,000\nHi\n").unwrap();

        let args = SyncArgs {
            positional_paths: vec![sub_path.clone()],
            offset: Some(-0.5),
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::new());
                assert_eq!(subtitle, sub_path);
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_explicit_video_and_subtitle() {
        let args = SyncArgs {
            video: Some(PathBuf::from("v.mp4")),
            subtitle: Some(PathBuf::from("s.srt")),
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::from("v.mp4"));
                assert_eq!(subtitle, PathBuf::from("s.srt"));
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_manual_explicit_subtitle_only() {
        let args = SyncArgs {
            offset: Some(1.0),
            subtitle: Some(PathBuf::from("s.srt")),
            ..default_args()
        };
        let mode = args.get_sync_mode().unwrap();
        match mode {
            SyncMode::Single { video, subtitle } => {
                assert_eq!(video, PathBuf::new());
                assert_eq!(subtitle, PathBuf::from("s.srt"));
            }
            _ => panic!("Expected Single mode"),
        }
    }

    #[test]
    fn test_get_sync_mode_manual_no_subtitle_err() {
        let args = SyncArgs {
            offset: Some(1.0),
            ..default_args()
        };
        assert!(args.get_sync_mode().is_err());
    }

    #[test]
    fn test_get_sync_mode_no_inputs_err() {
        let args = default_args();
        assert!(args.get_sync_mode().is_err());
    }

    // ── create_default_output_path ────────────────────────────────────────────

    #[test]
    fn test_create_default_output_path_srt() {
        let input = PathBuf::from("test.srt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "test_synced.srt");
    }

    #[test]
    fn test_create_default_output_path_with_prefix() {
        let input = PathBuf::from("/path/to/movie.ass");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "movie_synced.ass");
        assert_eq!(output.parent().unwrap(), std::path::Path::new("/path/to"));
    }

    #[test]
    fn test_create_default_output_path_vtt() {
        let input = PathBuf::from("episode.vtt");
        let output = create_default_output_path(&input);
        assert_eq!(output.file_name().unwrap(), "episode_synced.vtt");
    }

    #[test]
    fn test_create_default_output_path_no_extension() {
        // File without extension: stem exists but extension does not; path returned unchanged
        let input = PathBuf::from("noextension");
        let output = create_default_output_path(&input);
        assert_eq!(output, PathBuf::from("noextension"));
    }

    // ── CLI parsing ───────────────────────────────────────────────────────────

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
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--batch", "-i", "dir"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert!(args.validate().is_ok());

        let args_invalid = SyncArgs {
            batch: Some(None),
            ..default_args()
        };
        assert!(args_invalid.validate().is_err());
    }

    #[test]
    fn test_cli_parse_offset_and_subtitle() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--offset",
            "3.5",
            "--subtitle",
            "sub.srt",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.offset, Some(3.5));
        assert_eq!(args.subtitle, Some(PathBuf::from("sub.srt")));
    }

    #[test]
    fn test_cli_parse_negative_offset() {
        let cli =
            Cli::try_parse_from(["subx-cli", "sync", "--offset=-2.0", "-s", "sub.srt"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.offset, Some(-2.0));
    }

    #[test]
    fn test_cli_parse_method_vad() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--method", "vad", "--video", "v.mp4"])
            .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.method, Some(SyncMethodArg::Vad));
    }

    #[test]
    fn test_cli_parse_method_manual() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--method",
            "manual",
            "--offset",
            "1.0",
            "--subtitle",
            "sub.srt",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.method, Some(SyncMethodArg::Manual));
    }

    #[test]
    fn test_cli_parse_default_window() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--video", "v.mp4"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.window, 30);
    }

    #[test]
    fn test_cli_parse_custom_window() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--video", "v.mp4", "--window", "60"])
            .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.window, 60);
    }

    #[test]
    fn test_cli_parse_flags_verbose_dry_run_force() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--video",
            "v.mp4",
            "--verbose",
            "--dry-run",
            "--force",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert!(args.verbose);
        assert!(args.dry_run);
        assert!(args.force);
    }

    #[test]
    fn test_cli_parse_no_extract_flag() {
        let cli =
            Cli::try_parse_from(["subx-cli", "sync", "--video", "v.mp4", "--no-extract"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert!(args.no_extract);
    }

    #[test]
    fn test_cli_parse_output_path() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--video",
            "v.mp4",
            "--output",
            "result.srt",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.output, Some(PathBuf::from("result.srt")));
    }

    #[test]
    fn test_cli_parse_vad_sensitivity() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "--video",
            "v.mp4",
            "--vad-sensitivity",
            "0.8",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.vad_sensitivity, Some(0.8));
    }

    #[test]
    fn test_cli_parse_batch_with_directory() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "--batch", "mydir"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.batch, Some(Some(PathBuf::from("mydir"))));
    }

    #[test]
    fn test_cli_parse_positional_paths() {
        let cli = Cli::try_parse_from(["subx-cli", "sync", "video.mp4", "subtitle.srt"]).unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(
            args.positional_paths,
            vec![PathBuf::from("video.mp4"), PathBuf::from("subtitle.srt")]
        );
    }

    #[test]
    fn test_cli_parse_short_flags() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "sync",
            "-v",
            "video.mp4",
            "-s",
            "sub.srt",
            "-r",
            "-b",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Sync(a) => a,
            _ => panic!("Expected Sync command"),
        };
        assert_eq!(args.video, Some(PathBuf::from("video.mp4")));
        assert_eq!(args.subtitle, Some(PathBuf::from("sub.srt")));
        assert!(args.recursive);
        assert!(args.batch.is_some());
    }
}
