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
//! let rows = vec![
//!     MatchDisplayRow {
//!         status: "✓".to_string(),
//!         video_file: "movie.mp4".to_string(),
//!         subtitle_file: "subtitle.srt".to_string(),
//!         new_name: "movie.srt".to_string(),
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
/// containing all the information needed to display the outcome of an
/// AI-powered file matching operation. Each row represents one video-subtitle
/// pair and its processing status.
///
/// # Field Descriptions
///
/// - `status`: Visual indicator of the operation result (✓, ✗, ⚠, etc.)
/// - `video_file`: Original video file name or path
/// - `subtitle_file`: Original subtitle file name or path  
/// - `new_name`: Proposed or applied new subtitle file name
///
/// # Status Symbols
///
/// Common status values and their meanings:
/// - `✓`: Successfully matched and renamed
/// - `⚠`: Matched with low confidence (manual review recommended)
/// - `✗`: Failed to match or process
/// - `≈`: Approximate match (confidence below threshold)
/// - `→`: Dry-run mode (would be renamed)
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::table::MatchDisplayRow;
///
/// // Successful match
/// let success_row = MatchDisplayRow {
///     status: "✓".to_string(),
///     video_file: "Movie.2023.1080p.BluRay.mp4".to_string(),
///     subtitle_file: "sub_12345.srt".to_string(),
///     new_name: "Movie.2023.1080p.BluRay.srt".to_string(),
/// };
///
/// // Low confidence match
/// let warning_row = MatchDisplayRow {
///     status: "⚠".to_string(),
///     video_file: "Episode.S01E01.mkv".to_string(),
///     subtitle_file: "subtitles_v2.srt".to_string(),
///     new_name: "Episode.S01E01.srt".to_string(),
/// };
///
/// // Failed match
/// let error_row = MatchDisplayRow {
///     status: "✗".to_string(),
///     video_file: "Documentary.mp4".to_string(),
///     subtitle_file: "random_subtitle.srt".to_string(),
///     new_name: "Cannot match".to_string(),
/// };
/// ```
#[derive(Tabled)]
pub struct MatchDisplayRow {
    /// Operation status indicator with visual symbol.
    ///
    /// Provides immediate visual feedback about the matching operation result.
    /// Uses Unicode symbols for clear status communication across different
    /// terminal environments and languages.
    #[tabled(rename = "Status")]
    pub status: String,

    /// Original video file name or path.
    ///
    /// Displays the video file that was used as the reference for matching.
    /// May be shortened or formatted for display purposes while maintaining
    /// enough information for user identification.
    #[tabled(rename = "Video File")]
    pub video_file: String,

    /// Original subtitle file name or path.
    ///
    /// Shows the subtitle file that was processed for matching. This is
    /// typically the file with a non-descriptive or incorrect name that
    /// needs to be renamed to match the video.
    #[tabled(rename = "Subtitle File")]
    pub subtitle_file: String,

    /// Proposed or applied new subtitle file name.
    ///
    /// Displays the new name that was generated for the subtitle file based
    /// on the AI matching results. In dry-run mode, this shows what the name
    /// would be. In actual operation mode, this shows the applied name.
    #[tabled(rename = "New Name")]
    pub new_name: String,
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
/// let results = vec![
///     MatchDisplayRow {
///         status: "✓".to_string(),
///         video_file: "Movie.mp4".to_string(),
///         subtitle_file: "sub123.srt".to_string(),
///         new_name: "Movie.srt".to_string(),
///     },
///     MatchDisplayRow {
///         status: "⚠".to_string(),
///         video_file: "Episode.mkv".to_string(),
///         subtitle_file: "unknown.srt".to_string(),
///         new_name: "Episode.srt".to_string(),
///     },
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
