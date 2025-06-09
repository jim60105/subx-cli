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

// src/cli/convert_args.rs
use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Command-line arguments for subtitle format conversion.
///
/// The convert command transforms subtitle files between different formats
/// while preserving timing information and content structure. It supports
/// both single file and batch directory processing.
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ConvertArgs;
/// use subx_cli::cli::OutputSubtitleFormat;
/// use std::path::PathBuf;
///
/// let args = ConvertArgs {
///     input: PathBuf::from("input.srt"),
///     format: Some(OutputSubtitleFormat::Ass),
///     output: Some(PathBuf::from("output.ass")),
///     keep_original: true,
///     encoding: "utf-8".to_string(),
/// };
/// ```
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input file or directory path containing subtitle files.
    ///
    /// For single file conversion, specify the exact file path.
    /// For batch processing, specify a directory path and all
    /// supported subtitle files will be processed.
    pub input: PathBuf,

    /// Target output format for converted subtitles.
    ///
    /// If not specified, the default format from configuration
    /// will be used. Supported formats include SRT, ASS, VTT, and SUB.
    ///
    /// # Examples
    ///
    /// ```bash
    /// --format srt    # Convert to SubRip format
    /// --format ass    # Convert to Advanced SubStation Alpha
    /// --format vtt    # Convert to WebVTT format
    /// --format sub    # Convert to MicroDVD/SubViewer format
    /// ```
    #[arg(long, value_enum)]
    pub format: Option<OutputSubtitleFormat>,

    /// Output file path for the converted subtitle.
    ///
    /// If not specified for single file conversion, the output will use
    /// the same name as input with the appropriate extension.
    /// For batch processing, files are saved with new extensions in the
    /// same directory or a format-specific subdirectory.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Preserve the original files after conversion.
    ///
    /// By default, original files are preserved. Use this flag to explicitly
    /// keep originals during batch processing operations.
    #[arg(long)]
    pub keep_original: bool,

    /// Character encoding for input and output files.
    ///
    /// Specifies the character encoding to use when reading input files
    /// and writing output files. UTF-8 is the default and recommended
    /// encoding for maximum compatibility.
    ///
    /// # Supported Encodings
    ///
    /// - UTF-8 (default, recommended)
    /// - UTF-16LE, UTF-16BE
    /// - Windows-1252 (Western European)
    /// - ISO-8859-1 (Latin-1)
    /// - GBK, GB2312 (Chinese)
    /// - Shift_JIS (Japanese)
    ///
    /// # Examples
    ///
    /// ```bash
    /// --encoding utf-8        # UTF-8 encoding (default)
    /// --encoding windows-1252 # Windows Western European
    /// --encoding gbk          # Chinese GBK encoding
    /// ```
    #[arg(long, default_value = "utf-8")]
    pub encoding: String,
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

// 測試參數解析行為
#[cfg(test)]
mod tests {
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
        assert_eq!(args.input, PathBuf::from("in_path"));
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
        assert_eq!(args.input, PathBuf::from("in"));
        assert_eq!(args.format.unwrap(), OutputSubtitleFormat::Vtt);
        assert_eq!(args.output, Some(PathBuf::from("out")));
        assert!(args.keep_original);
        assert_eq!(args.encoding, "gbk");
    }
}
