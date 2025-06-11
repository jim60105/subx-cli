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
//! // 每個匹配結果拆分為多行顯示：影片、字幕與新檔名
//! let rows = vec![
//!     MatchDisplayRow {
//!         status: "✓".to_string(),
//!         filename: "Video 1: movie.mp4".to_string(),
//!     },
//!     MatchDisplayRow {
//!         status: "".to_string(),
//!         filename: "├ Subtitle 1: subtitle.srt".to_string(),
//!     },
//!     MatchDisplayRow {
//!         status: "".to_string(),
//!         filename: "└ New name 1: movie.srt".to_string(),
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
/// // 成功匹配
/// let success_row = MatchDisplayRow {
///     status: "✓".to_string(),
///     filename: "Video 1: Movie.2023.1080p.BluRay.mp4".to_string(),
/// };
///
/// // 低信心匹配
/// let warning_row = MatchDisplayRow {
///     status: "⚠".to_string(),
///     filename: "Video 2: Episode.S01E01.mkv".to_string(),
/// };
///
/// // 匹配失敗示例只需展示狀態
/// let error_row = MatchDisplayRow {
///     status: "✗".to_string(),
///     filename: String::new(),
/// };
/// ```
#[derive(Tabled)]
/// Match 結果表格列，用於顯示狀態與相關檔案資訊的垂直布局
pub struct MatchDisplayRow {
    /// 處理狀態視覺圖示（✓、🔍、⚠、✗）
    #[tabled(rename = "狀態")]
    pub status: String,

    /// 影片檔案、字幕檔案與新檔名的垂直堆疊資訊
    #[tabled(rename = "檔案名稱")]
    pub filename: String,
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
/// // 多行顯示多個匹配結果
/// let results = vec![
///     MatchDisplayRow { status: "✓".to_string(), filename: "Video 1: Movie.mp4".to_string() },
///     MatchDisplayRow { status: "".to_string(), filename: "├ Subtitle 1: sub123.srt".to_string() },
///     MatchDisplayRow { status: "".to_string(), filename: "└ New name 1: Movie.srt".to_string() },
///     MatchDisplayRow { status: "⚠".to_string(), filename: "Video 2: Episode.mkv".to_string() },
///     MatchDisplayRow { status: "".to_string(), filename: "├ Subtitle 2: unknown.srt".to_string() },
///     MatchDisplayRow { status: "".to_string(), filename: "└ New name 2: Episode.srt".to_string() },
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
