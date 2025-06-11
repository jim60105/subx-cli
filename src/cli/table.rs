//! Table formatting utilities for displaying structured CLI output.
//!
//! This module provides specialized table formatting capabilities for displaying
//! various types of structured data in the SubX CLI. It focuses primarily on
//! match operation results but can be extended for other tabular data needs.
//!
//! # Features
//!
//! - **Match Results Display**: Formatted tables for AI matching operations
//! - **Consistent Styling**: Rounded borders and aligned columns
//! - **Internationalization**: Support for Chinese column headers
//! - **Flexible Layout**: Automatic column width adjustment
//!
//! # Table Styling
//!
//! All tables use a consistent rounded border style with left-aligned content
//! for optimal readability. The styling is designed to work well in both
//! light and dark terminal themes.
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::cli::table::{MatchDisplayRow, create_match_table};
//!
//! // Each match result is split into multiple lines for display: video, subtitle and new name
//! let rows = vec![
//!     MatchDisplayRow {
//!         file_type: "Video 1".to_string(),
//!         file_path: "movie.mp4".to_string(),
//!     },
//!     MatchDisplayRow {
//!         file_type: "Subtitle 1".to_string(),
//!         file_path: "subtitle.srt".to_string(),
//!     },
//!     MatchDisplayRow {
//!         file_type: "New name 1".to_string(),
//!         file_path: "movie.srt".to_string(),
//!     },
//! ];
//!
//! let table = create_match_table(rows);
//! println!("{}", table);
//! ```

use tabled::settings::{Alignment, Modify, Style, object::Rows};
use tabled::{Table, Tabled};

/// Display row structure for file matching operation results.
///
/// This structure represents a single row in the match results table,
/// containing file type identifier and complete file path. Each row shows
/// one piece of information from an AI-powered file matching operation,
/// with multiple rows grouped together to represent one complete match result.
///
/// # Field Descriptions
///
/// - `file_type`: File type identifier (Video 1, Subtitle 1, New name 1)
/// - `file_path`: Complete file path displayed without truncation
///
/// # File Type Identifiers
///
/// Standard file type values:
/// - `Video 1`, `Video 2`, etc.: Original video files used as reference
/// - `Subtitle 1`, `Subtitle 2`, etc.: Original subtitle files to be renamed
/// - `New name 1`, `New name 2`, etc.: Generated new names for subtitle files
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::table::MatchDisplayRow;
///
/// // Video file entry
/// let video_row = MatchDisplayRow {
///     file_type: "Video 1".to_string(),
///     file_path: "/path/to/Movie.2023.1080p.BluRay.mp4".to_string(),
/// };
///
/// // Subtitle file entry
/// let subtitle_row = MatchDisplayRow {
///     file_type: "Subtitle 1".to_string(),
///     file_path: "/path/to/random_subtitle.srt".to_string(),
/// };
///
/// // New name entry
/// let newname_row = MatchDisplayRow {
///     file_type: "New name 1".to_string(),
///     file_path: "/path/to/Movie.2023.1080p.BluRay.srt".to_string(),
/// };
/// ```
#[derive(Tabled)]
/// Match result table row for displaying file type and path in clean two-column layout
pub struct MatchDisplayRow {
    /// File type identifier (Video 1, Subtitle 1, New name 1)
    #[tabled(rename = "Type")]
    pub file_type: String,

    /// Complete file path without truncation
    #[tabled(rename = "Path")]
    pub file_path: String,
}

/// Create a formatted table string from match operation results.
///
/// Transforms a collection of match display rows into a beautifully formatted
/// table string suitable for terminal display. The table uses consistent
/// styling with rounded borders and proper column alignment for optimal
/// readability.
///
/// # Table Features
///
/// - **Rounded borders**: Modern, visually appealing table style
/// - **Left alignment**: Consistent text alignment for all content rows
/// - **Auto-sizing**: Columns automatically adjust to content width
/// - **Header styling**: Clear distinction between headers and data
/// - **Unicode support**: Proper handling of Chinese characters and symbols
///
/// # Arguments
///
/// * `rows` - Vector of `MatchDisplayRow` structures to be displayed
///
/// # Returns
///
/// A formatted table string ready for printing to the terminal
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::table::{MatchDisplayRow, create_match_table};
///
/// // Multi-line display of multiple match results
/// let results = vec![
///     MatchDisplayRow { file_type: "Video 1".to_string(), file_path: "Movie.mp4".to_string() },
///     MatchDisplayRow { file_type: "Subtitle 1".to_string(), file_path: "sub123.srt".to_string() },
///     MatchDisplayRow { file_type: "New name 1".to_string(), file_path: "Movie.srt".to_string() },
///     MatchDisplayRow { file_type: "Video 2".to_string(), file_path: "Episode.mkv".to_string() },
///     MatchDisplayRow { file_type: "Subtitle 2".to_string(), file_path: "unknown.srt".to_string() },
///     MatchDisplayRow { file_type: "New name 2".to_string(), file_path: "Episode.srt".to_string() },
/// ];
///
/// let table = create_match_table(results);
/// println!("{}", table);
/// ```
///
/// # Output Example
///
/// ```text
/// ╭──────┬──────────────┬──────────────┬──────────────╮
/// │ Status │ Video File     │ Subtitle File     │ New Name       │
/// ├──────┼──────────────┼──────────────┼──────────────┤
/// │ ✓    │ Movie.mp4    │ sub123.srt   │ Movie.srt    │
/// │ ⚠    │ Episode.mkv  │ unknown.srt  │ Episode.srt  │
/// ╰──────┴──────────────┴──────────────┴──────────────╯
/// ```
///
/// # Empty Input Handling
///
/// If an empty vector is provided, returns a table with only headers,
/// indicating no results to display.
pub fn create_match_table(rows: Vec<MatchDisplayRow>) -> String {
    let mut table = Table::new(rows);
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()));
    table.to_string()
}
