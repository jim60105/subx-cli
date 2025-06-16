#![allow(clippy::needless_borrows_for_generic_args)]
//! Command-line arguments for the AI-powered subtitle matching command.

use crate::cli::InputPathHandler;
use crate::error::SubXError;
use clap::Args;
use std::path::PathBuf;

/// Arguments for AI-powered subtitle file matching and renaming.
#[derive(Args, Debug)]
pub struct MatchArgs {
    /// Target directory path containing video and subtitle files
    pub path: Option<PathBuf>,

    /// Specify file or directory paths to process (new parameter), can be used multiple times
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Enable dry-run mode to preview operations without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Minimum confidence threshold for file matching (0-100)
    #[arg(long, default_value = "80", value_parser = clap::value_parser!(u8).range(0..=100))]
    pub confidence: u8,

    /// Recursively process subdirectories
    #[arg(short, long)]
    pub recursive: bool,

    /// Create backup copies of original files before renaming
    #[arg(long)]
    pub backup: bool,

    /// Copy matched subtitle files to the same folder as their corresponding video files
    #[arg(long, short = 'c')]
    pub copy: bool,

    /// Move matched subtitle files to the same folder as their corresponding video files
    #[arg(long = "move", short = 'm')]
    pub move_files: bool,
}

impl MatchArgs {
    /// Validate that copy and move arguments are not used together
    pub fn validate(&self) -> Result<(), String> {
        if self.copy && self.move_files {
            return Err(
                "Cannot use --copy and --move together. Please choose one operation mode."
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Get all input paths, combining path and input_paths parameters
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        let optional_paths = vec![self.path.clone()];
        let merged_paths = InputPathHandler::merge_paths_from_multiple_sources(
            &optional_paths,
            &self.input_paths,
            &[],
        )?;

        Ok(InputPathHandler::from_args(&merged_paths, self.recursive)?
            .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]))
    }
}

// Test parameter parsing behavior
#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn test_match_args_default_values() {
        let cli = Cli::try_parse_from(&["subx-cli", "match", "path"]).unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert_eq!(args.path, Some(PathBuf::from("path")));
        assert!(args.input_paths.is_empty());
        assert!(!args.dry_run);
        assert!(!args.recursive);
        assert!(!args.backup);
        assert_eq!(args.confidence, 80);
    }

    #[test]
    fn test_match_args_parsing() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "match",
            "path",
            "--dry-run",
            "--recursive",
            "--backup",
            "--confidence",
            "50",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert_eq!(args.path, Some(PathBuf::from("path")));
        assert!(args.input_paths.is_empty());
        assert!(args.dry_run);
        assert!(args.recursive);
        assert!(args.backup);
        assert_eq!(args.confidence, 50);
    }

    #[test]
    fn test_match_args_invalid_confidence() {
        let res = Cli::try_parse_from(&["subx-cli", "match", "path", "--confidence", "150"]);
        assert!(res.is_err());
    }

    #[test]
    fn test_match_args_copy_and_move_mutually_exclusive() {
        let cli = Cli::try_parse_from(&["subx-cli", "match", "path", "--copy", "--move"]).unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        let result = args.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Cannot use --copy and --move together")
        );
    }

    #[test]
    fn test_match_args_copy_parameter() {
        let cli = Cli::try_parse_from(&["subx-cli", "match", "path", "--copy"]).unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert!(args.copy);
        assert!(!args.move_files);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_match_args_move_parameter() {
        let cli = Cli::try_parse_from(&["subx-cli", "match", "path", "--move"]).unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert!(!args.copy);
        assert!(args.move_files);
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_match_args_input_paths() {
        let cli = Cli::try_parse_from(&[
            "subx-cli",
            "match",
            "-i",
            "dir1",
            "-i",
            "dir2",
            "--recursive",
        ])
        .unwrap();
        let args = match cli.command {
            Commands::Match(m) => m,
            _ => panic!("Expected Match command"),
        };
        assert!(args.path.is_none());
        assert_eq!(
            args.input_paths,
            vec![PathBuf::from("dir1"), PathBuf::from("dir2")]
        );
        assert!(args.recursive);
    }
}
