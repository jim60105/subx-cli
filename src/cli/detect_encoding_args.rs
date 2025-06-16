//! File encoding detection command-line arguments and options.
//!
//! This module defines the command-line interface for the `detect-encoding` subcommand,
//! which analyzes text files to determine their character encoding. This is particularly
//! useful for subtitle files that may be encoded in various character sets, especially
//! legacy encodings or region-specific formats.
//!
//! # Supported Detection
//!
//! The encoding detection can identify a wide range of character encodings including:
//! - UTF-8, UTF-16LE, UTF-16BE (Unicode variants)
//! - Windows-1252, ISO-8859-1 (Western European)
//! - GBK, GB2312, Big5 (Chinese variants)
//! - Shift_JIS, EUC-JP (Japanese)
//! - KOI8-R, Windows-1251 (Cyrillic)
//! - And many more regional encodings
//!
//! # Examples
//!
//! ```bash
//! # Detect encoding of a single file
//! subx detect-encoding subtitle.srt
//!
//! # Detect encoding of multiple files with verbose output
//! subx detect-encoding --verbose *.srt *.sub
//!
//! # Batch detect all subtitle files in current directory
//! subx detect-encoding *.srt *.ass *.vtt *.sub
//! ```

use crate::cli::InputPathHandler;
use crate::error::SubXError;
use clap::Args;
use std::path::PathBuf;

/// Command-line arguments for file encoding detection.
///
/// The detect-encoding command analyzes the byte patterns and character
/// distributions in text files to determine their most likely character
/// encoding. This is essential for processing subtitle files that may
/// have been created with different encodings.
///
/// # Detection Algorithm
///
/// The detection process uses multiple approaches:
/// 1. **BOM (Byte Order Mark) detection** for Unicode files
/// 2. **Statistical analysis** of byte patterns
/// 3. **Character frequency analysis** for specific languages
/// 4. **Heuristic rules** based on encoding characteristics
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::DetectEncodingArgs;
/// use std::path::PathBuf;
///
/// // 使用傳統檔案路徑參數
/// let args = DetectEncodingArgs {
///     verbose: true,
///     file_paths: vec![
///         "subtitle1.srt".to_string(),
///         "subtitle2.ass".to_string(),
///     ],
///     input_paths: vec![],
///     recursive: false,
/// };
///
/// // 使用新的 -i 參數進行批次處理
/// let batch_args = DetectEncodingArgs {
///     verbose: true,
///     file_paths: vec![],
///     input_paths: vec![
///         PathBuf::from("./subtitles/"),
///         PathBuf::from("./more_subs/"),
///     ],
///     recursive: true,
/// };
/// ```
#[derive(Args, Debug)]
pub struct DetectEncodingArgs {
    /// Display detailed sample text and confidence information.
    ///
    /// When enabled, shows additional information about the detection process:
    /// - Confidence percentage for the detected encoding
    /// - Sample text decoded with the detected encoding
    /// - Alternative encoding candidates with their confidence scores
    /// - Detected language hints (if available)
    ///
    /// This is useful for verifying detection accuracy and troubleshooting
    /// encoding issues with problematic files.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Basic detection
    /// subx detect-encoding file.srt
    /// # Output: file.srt: UTF-8
    ///
    /// # Verbose detection with details
    /// subx detect-encoding --verbose file.srt
    /// # Output:
    /// # file.srt: UTF-8 (99.5% confidence)
    /// # Sample: "1\n00:00:01,000 --> 00:00:03,000\nHello World"
    /// # Alternatives: ISO-8859-1 (15.2%), Windows-1252 (12.8%)
    /// ```
    #[arg(short, long)]
    pub verbose: bool,

    /// 指定要處理的檔案或目錄路徑（新增參數，與 file_paths 互斥）
    #[arg(
        short = 'i',
        long = "input",
        value_name = "PATH",
        conflicts_with = "file_paths"
    )]
    pub input_paths: Vec<PathBuf>,

    /// 遞迴處理子目錄（新增參數）
    #[arg(short, long)]
    pub recursive: bool,

    /// File paths to analyze for encoding detection.
    ///
    /// Accepts multiple file paths or glob patterns. All specified files
    /// will be analyzed and their detected encodings reported. The command
    /// supports both absolute and relative paths.
    ///
    /// # Supported File Types
    ///
    /// While primarily designed for subtitle files, the detection works
    /// with any text-based file:
    /// - Subtitle files: .srt, .ass, .vtt, .sub, .ssa, .smi
    /// - Text files: .txt, .md, .csv, .json, .xml
    /// - Script files: .py, .js, .html, .css
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Single file
    /// subx detect-encoding subtitle.srt
    ///
    /// # Multiple specific files
    /// subx detect-encoding file1.srt file2.ass file3.vtt
    ///
    /// # Glob patterns (shell expansion)
    /// subx detect-encoding *.srt
    /// subx detect-encoding subtitles/*.{srt,ass}
    ///
    /// # Mixed paths
    /// subx detect-encoding /absolute/path/file.srt ./relative/file.ass
    /// ```
    ///
    /// # Error Handling
    ///
    /// If a file cannot be read or analyzed:
    /// - The error is reported for that specific file
    /// - Processing continues with remaining files
    /// - Non-text files are skipped with a warning
    /// - Permission errors are clearly indicated
    #[arg(required = true, conflicts_with = "input_paths")]
    pub file_paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_detect_encoding_args_file_paths() {
        let cli = Cli::try_parse_from(["subx-cli", "detect-encoding", "a.srt", "b.ass"]).unwrap();
        let args = match cli.command {
            Commands::DetectEncoding(a) => a,
            _ => panic!("Expected DetectEncoding command"),
        };
        assert!(args.input_paths.is_empty());
        assert_eq!(
            args.file_paths,
            vec!["a.srt".to_string(), "b.ass".to_string()]
        );
        assert!(!args.recursive);
    }

    #[test]
    fn test_detect_encoding_args_input_paths() {
        let cli = Cli::try_parse_from([
            "subx-cli",
            "detect-encoding",
            "-i",
            "dir1",
            "-i",
            "dir2",
            "--recursive",
            "--verbose",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::DetectEncoding(a) => a,
            _ => panic!("Expected DetectEncoding command"),
        };
        assert!(args.file_paths.is_empty());
        assert_eq!(
            args.input_paths,
            vec![PathBuf::from("dir1"), PathBuf::from("dir2")]
        );
        assert!(args.recursive);
        assert!(args.verbose);
    }

    #[test]
    fn test_detect_encoding_args_conflict_file_and_input() {
        let res = Cli::try_parse_from(["subx-cli", "detect-encoding", "file.srt", "-i", "dir"]);
        assert!(res.is_err());
    }
}

impl DetectEncodingArgs {
    /// 取得所有輸入路徑，合併 file_paths 和 input_paths 參數
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &[],
            &self.input_paths,
            &self.file_paths,
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["srt", "ass", "vtt", "ssa", "sub", "txt"]))
    }

    /// 取得所有要處理的檔案路徑
    pub fn get_file_paths(&self) -> Result<Vec<PathBuf>, SubXError> {
        if !self.input_paths.is_empty() {
            let handler = InputPathHandler::from_args(&self.input_paths, self.recursive)?
                .with_extensions(&["srt", "ass", "vtt", "ssa", "sub", "txt"]);
            return handler.collect_files();
        }
        if !self.file_paths.is_empty() {
            return Ok(self.file_paths.iter().map(PathBuf::from).collect());
        }
        Err(SubXError::NoInputSpecified)
    }
}
