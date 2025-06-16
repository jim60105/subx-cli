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
#[derive(Args, Debug)]
pub struct DetectEncodingArgs {
    /// Display detailed sample text and confidence information
    #[arg(short, long)]
    pub verbose: bool,

    /// Specify file or directory paths to process (new parameter, mutually exclusive with file_paths)
    #[arg(
        short = 'i',
        long = "input",
        value_name = "PATH",
        conflicts_with = "file_paths"
    )]
    pub input_paths: Vec<PathBuf>,

    /// Recursively process subdirectories (new parameter)
    #[arg(short, long)]
    pub recursive: bool,

    /// File paths to analyze for encoding detection
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
    /// Get all input paths, combining file_paths and input_paths parameters
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &[],
            &self.input_paths,
            &self.file_paths,
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["srt", "ass", "vtt", "ssa", "sub", "txt"]))
    }

    /// Get all file paths to process
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
