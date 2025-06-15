#![allow(clippy::needless_borrows_for_generic_args)]
//! Command-line arguments for the AI-powered subtitle matching command.

use crate::cli::InputPathHandler;
use crate::error::SubXError;
use clap::Args;
use std::path::PathBuf;

/// Arguments for AI-powered subtitle file matching and renaming.
///
/// This structure defines all command-line options available for the `match`
/// subcommand, which uses artificial intelligence to analyze video and subtitle
/// files and automatically rename subtitles to match their corresponding videos.
///
/// # Operation Modes
///
/// - **Normal Mode**: Performs actual file operations
/// - **Dry Run Mode**: Simulates operations without making changes (`--dry-run`)
/// - **Recursive Mode**: Processes subdirectories (`--recursive`)
/// - **Backup Mode**: Creates backups before renaming (`--backup`)
/// - **Copy Mode**: Copy matched subtitle files to video folders (`--copy`)
/// - **Move Mode**: Move matched subtitle files to video folders (`--move`)
///
/// # AI Matching Process
///
/// 1. Scans the target directory for video and subtitle files
/// 2. Extracts content samples from both file types
/// 3. Uses AI to analyze content similarity
/// 4. Matches files based on confidence threshold
/// 5. Renames subtitle files to match video file names
/// 6. Optionally relocates subtitle files to video directories
///
/// # Examples
///
/// ```bash
/// # Basic matching in current directory
/// subx match ./videos
///
/// # Dry run with high confidence threshold
/// subx match ./videos --dry-run --confidence 90
///
/// # Recursive matching with backup and copy to video folders
/// subx match ./media --recursive --backup --copy
/// ```
#[derive(Args, Debug)]
pub struct MatchArgs {
    /// Target directory path containing video and subtitle files.
    ///
    /// The directory should contain both video files and subtitle files
    /// that need to be matched and renamed. Supported video formats include
    /// MP4, MKV, AVI, etc. Supported subtitle formats include SRT, ASS, VTT, etc.
    pub path: Option<PathBuf>,

    /// 指定要處理的檔案或目錄路徑（新增參數），可以多次使用
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    pub input_paths: Vec<PathBuf>,

    /// Enable dry-run mode to preview operations without making changes.
    ///
    /// When enabled, the command will analyze files and show what operations
    /// would be performed, but won't actually rename any files. This is useful
    /// for testing matching accuracy before committing to changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Minimum confidence threshold for file matching (0-100).
    ///
    /// Only file pairs with similarity confidence above this threshold
    /// will be matched and renamed. Higher values result in more conservative
    /// matching with fewer false positives, while lower values are more
    /// aggressive but may include incorrect matches.
    ///
    /// # Recommended Values
    /// - 90-100: Very conservative, highest accuracy
    /// - 80-89: Balanced approach (default)
    /// - 70-79: More aggressive matching
    /// - Below 70: Not recommended for automatic operations
    #[arg(long, default_value = "80", value_parser = clap::value_parser!(u8).range(0..=100))]
    pub confidence: u8,

    /// Recursively process subdirectories.
    ///
    /// When enabled, the matching process will descend into subdirectories
    /// and process video and subtitle files found there. Each subdirectory
    /// is processed independently, so files are only matched within the
    /// same directory level.
    #[arg(short, long)]
    pub recursive: bool,

    /// Create backup copies of original files before renaming.
    ///
    /// When enabled, original subtitle files are copied to `.bak` files
    /// before being renamed. This provides a safety net in case the
    /// matching algorithm makes incorrect decisions.
    #[arg(long)]
    pub backup: bool,

    /// Copy matched subtitle files to the same folder as their corresponding video files.
    ///
    /// When enabled along with recursive search, subtitle files that are matched
    /// with video files in different directories will be copied to the video file's
    /// directory. This ensures that media players can automatically load subtitles.
    /// The original subtitle files are preserved in their original locations.
    /// Cannot be used together with --move.
    #[arg(long, short = 'c')]
    pub copy: bool,

    /// Move matched subtitle files to the same folder as their corresponding video files.
    ///
    /// When enabled along with recursive search, subtitle files that are matched
    /// with video files in different directories will be moved to the video file's
    /// directory. This ensures that media players can automatically load subtitles.
    /// The original subtitle files are removed from their original locations.
    /// Cannot be used together with --copy.
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

    /// 取得所有輸入路徑，優先使用 -i 參數
    pub fn get_input_handler(&self) -> Result<InputPathHandler, SubXError> {
        if !self.input_paths.is_empty() {
            Ok(
                InputPathHandler::from_args(&self.input_paths, self.recursive)?
                    .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]),
            )
        } else if let Some(p) = &self.path {
            Ok(InputPathHandler::from_args(&[p.clone()], self.recursive)?
                .with_extensions(&["mp4", "mkv", "avi", "mov", "srt", "ass", "vtt", "sub"]))
        } else {
            Err(SubXError::NoInputSpecified)
        }
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
