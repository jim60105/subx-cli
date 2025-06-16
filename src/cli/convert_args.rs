//! Subtitle format conversion command-line arguments and options.
//!
//! This module defines the command-line interface for the `convert` subcommand,
//! which handles subtitle format conversion between different standards like SRT,
//! ASS, VTT, and SUB. It provides comprehensive format conversion capabilities
//! with encoding support and optional file preservation.
//!
//! # Supported Formats
//!
//! - **SRT (SubRip)**: Most widely used subtitle format
//! - **ASS (Advanced SubStation Alpha)**: Rich formatting and styling support
//! - **VTT (WebVTT)**: Web-optimized subtitle format for HTML5 video
//! - **SUB (MicroDVD/SubViewer)**: Frame-based subtitle format
//!
//! # Examples
//!
//! ```bash
//! # Convert SRT to ASS format
//! subx convert input.srt --format ass --output output.ass
//!
//! # Batch convert all SRT files in a directory to VTT
//! subx convert ./subtitles/ --format vtt
//!
//! # Convert with specific encoding
//! subx convert input.srt --format ass --encoding utf-8 --keep-original
//! ```

#![allow(clippy::needless_borrows_for_generic_args)]
// src/cli/convert_args.rs
use crate::cli::InputPathHandler;
use crate::error::SubXError;
use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Command-line arguments for subtitle format conversion.
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input file or directory path containing subtitle files
    pub input: Option<PathBuf>,

    /// Specify file or directory paths to process (new parameter), can be used multiple times
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Recursively process subdirectories (new parameter)
    #[arg(short, long)]
    pub recursive: bool,

    /// Target output format for converted subtitles
    #[arg(long, value_enum)]
    pub format: Option<OutputSubtitleFormat>,

    /// Output file path for the converted subtitle
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Preserve the original files after conversion
    #[arg(long)]
    pub keep_original: bool,

    /// Character encoding for input and output files
    #[arg(long, default_value = "utf-8")]
    pub encoding: String,
}

impl ConvertArgs {
    /// Get all input paths
    /// Get all input paths, combining input and input_paths parameters
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let optional_paths = vec![self.input.clone()];
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &optional_paths,
            &self.input_paths,
            &[],
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["srt", "ass", "vtt", "sub", "ssa"]))
    }
}

/// Supported output subtitle formats for conversion operations.
///
/// This enum defines all subtitle formats that SubX can generate as output.
/// Each format has specific characteristics and use cases:
///
/// - **SRT**: Simple, widely supported, good for basic subtitles
/// - **ASS**: Advanced formatting, styling, and positioning capabilities
/// - **VTT**: Web-optimized, supports HTML5 video elements
/// - **SUB**: Frame-based timing, used in some legacy systems
///
/// # Format Characteristics
///
/// | Format | Timing | Styling | Web Support | Compatibility |
/// |--------|--------|---------|-------------|---------------|
/// | SRT    | Time   | Basic   | Good        | Excellent     |
/// | ASS    | Time   | Rich    | Limited     | Good          |
/// | VTT    | Time   | Medium  | Excellent   | Good          |
/// | SUB    | Frame  | Basic   | Poor        | Limited       |
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::OutputSubtitleFormat;
///
/// let srt_format = OutputSubtitleFormat::Srt;
/// assert_eq!(srt_format.as_str(), "srt");
/// assert_eq!(srt_format.file_extension(), ".srt");
/// ```
#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum OutputSubtitleFormat {
    /// SubRip (.srt) format - most widely supported subtitle format.
    ///
    /// Features:
    /// - Simple time-based format
    /// - Basic text formatting (bold, italic, underline)
    /// - Excellent player compatibility
    /// - Small file size
    Srt,

    /// Advanced SubStation Alpha (.ass) format - professional subtitle format.
    ///
    /// Features:
    /// - Rich styling and formatting options
    /// - Precise positioning and animation
    /// - Multiple font and color support
    /// - Advanced timing controls
    Ass,

    /// WebVTT (.vtt) format - web-optimized subtitle format.
    ///
    /// Features:
    /// - HTML5 video element support
    /// - CSS-like styling capabilities
    /// - Cue positioning and alignment
    /// - Web accessibility features
    Vtt,

    /// MicroDVD/SubViewer (.sub) format - frame-based subtitle format.
    ///
    /// Features:
    /// - Frame-based timing (not time-based)
    /// - Basic text formatting
    /// - Legacy format support
    /// - Compact file structure
    Sub,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_convert_args_default_and_format() {
        let cli =
            Cli::try_parse_from(&["subx-cli", "convert", "movie.srt", "--format", "ass"]).unwrap();
        let args = match cli.command {
            Commands::Convert(a) => a,
            _ => panic!("Expected Convert command"),
        };
        assert!(args.input_paths.is_empty());
        assert_eq!(args.input, Some(PathBuf::from("movie.srt")));
        assert!(!args.recursive);
        assert_eq!(args.format, Some(OutputSubtitleFormat::Ass));
        assert!(!args.keep_original);
        assert_eq!(args.encoding, "utf-8");
    }

    #[test]
    fn test_convert_args_multiple_input_recursive_and_keep_original() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "convert",
            "-i",
            "d1",
            "-i",
            "f.srt",
            "--recursive",
            "--format",
            "vtt",
            "--encoding",
            "utf-16",
            "--keep-original",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Convert(a) => a,
            _ => panic!("Expected Convert command"),
        };
        assert_eq!(
            args.input_paths,
            vec![PathBuf::from("d1"), PathBuf::from("f.srt")]
        );
        assert_eq!(args.input, None);
        assert!(args.recursive);
        assert_eq!(args.format, Some(OutputSubtitleFormat::Vtt));
        assert!(args.keep_original);
        assert_eq!(args.encoding, "utf-16");
    }
}

impl OutputSubtitleFormat {
    /// Returns the format identifier as a string.
    ///
    /// This method provides the lowercase string representation of the format,
    /// which is used for command-line arguments and configuration files.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::cli::OutputSubtitleFormat;
    ///
    /// assert_eq!(OutputSubtitleFormat::Srt.as_str(), "srt");
    /// assert_eq!(OutputSubtitleFormat::Ass.as_str(), "ass");
    /// assert_eq!(OutputSubtitleFormat::Vtt.as_str(), "vtt");
    /// assert_eq!(OutputSubtitleFormat::Sub.as_str(), "sub");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputSubtitleFormat::Srt => "srt",
            OutputSubtitleFormat::Ass => "ass",
            OutputSubtitleFormat::Vtt => "vtt",
            OutputSubtitleFormat::Sub => "sub",
        }
    }

    /// Returns the file extension for this format including the dot prefix.
    ///
    /// This method provides the standard file extension used for each
    /// subtitle format, which is useful for generating output filenames.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::cli::OutputSubtitleFormat;
    ///
    /// assert_eq!(OutputSubtitleFormat::Srt.file_extension(), ".srt");
    /// assert_eq!(OutputSubtitleFormat::Ass.file_extension(), ".ass");
    /// assert_eq!(OutputSubtitleFormat::Vtt.file_extension(), ".vtt");
    /// assert_eq!(OutputSubtitleFormat::Sub.file_extension(), ".sub");
    /// ```
    pub fn file_extension(&self) -> &'static str {
        match self {
            OutputSubtitleFormat::Srt => ".srt",
            OutputSubtitleFormat::Ass => ".ass",
            OutputSubtitleFormat::Vtt => ".vtt",
            OutputSubtitleFormat::Sub => ".sub",
        }
    }
}

impl std::fmt::Display for OutputSubtitleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
// Test parameter parsing behavior
#[cfg(test)]
mod tests_parse {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_convert_args_default_values() {
        let cli = Cli::try_parse_from(&["subx-cli", "convert", "in_path"]).unwrap();
        let args = match cli.command {
            Commands::Convert(c) => c,
            _ => panic!("Expected Convert command"),
        };
        assert_eq!(args.input, Some(PathBuf::from("in_path")));
        assert!(args.input_paths.is_empty());
        assert!(!args.recursive);
        assert_eq!(args.format, None);
        assert_eq!(args.output, None);
        assert!(!args.keep_original);
        assert_eq!(args.encoding, "utf-8");
    }

    #[test]
    fn test_convert_args_parsing() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "convert",
            "in",
            "--format",
            "vtt",
            "--output",
            "out",
            "--keep-original",
            "--encoding",
            "gbk",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Convert(c) => c,
            _ => panic!("Expected Convert command"),
        };
        assert_eq!(args.input, Some(PathBuf::from("in")));
        assert!(args.input_paths.is_empty());
        assert!(!args.recursive);
        assert_eq!(args.format.unwrap(), OutputSubtitleFormat::Vtt);
        assert_eq!(args.output, Some(PathBuf::from("out")));
        assert!(args.keep_original);
        assert_eq!(args.encoding, "gbk");
    }
}
