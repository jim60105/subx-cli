//! Command-line arguments for the AI-powered subtitle matching command.

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
///
/// # AI Matching Process
///
/// 1. Scans the target directory for video and subtitle files
/// 2. Extracts content samples from both file types
/// 3. Uses AI to analyze content similarity
/// 4. Matches files based on confidence threshold
/// 5. Renames subtitle files to match video file names
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
/// # Recursive matching with backup
/// subx match ./media --recursive --backup
/// ```
#[derive(Args, Debug)]
pub struct MatchArgs {
    /// Target directory path containing video and subtitle files.
    ///
    /// The directory should contain both video files and subtitle files
    /// that need to be matched and renamed. Supported video formats include
    /// MP4, MKV, AVI, etc. Supported subtitle formats include SRT, ASS, VTT, etc.
    pub path: PathBuf,

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
        assert_eq!(args.path, PathBuf::from("path"));
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
}
