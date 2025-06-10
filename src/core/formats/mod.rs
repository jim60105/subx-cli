//! Comprehensive subtitle format handling and conversion system.
//!
//! This module provides a unified interface for parsing, converting, and managing
//! multiple subtitle formats including SRT, ASS/SSA, VTT (WebVTT), and SUB formats.
//! It handles format detection, parsing, conversion between formats, and preservation
//! of styling information where supported.
//!
//! # Supported Formats
//!
//! - **SRT (SubRip)**: The most common subtitle format with simple timing and text
//! - **ASS/SSA (Advanced SubStation Alpha)**: Advanced format with rich styling support
//! - **VTT (WebVTT)**: Web-based format with positioning and styling capabilities
//! - **SUB (MicroDVD)**: Frame-based timing format
//!
//! # Architecture
//!
//! The format system is built around several key components:
//!
//! - **Format Detection**: Automatic detection based on file extension and content analysis
//! - **Unified Data Model**: Common `Subtitle` and `SubtitleEntry` structures for all formats
//! - **Format-Specific Parsers**: Dedicated parsing logic for each format
//! - **Conversion Engine**: Intelligent conversion between formats with feature mapping
//! - **Styling Preservation**: Maintains formatting information during conversions
//! - **Encoding Handling**: Automatic encoding detection and conversion
//!
//! # Usage Examples
//!
//! ## Basic Format Detection and Parsing
//!
//! ```rust,ignore
//! use subx_cli::core::formats::{manager::FormatManager, SubtitleFormatType};
//! use std::path::Path;
//!
//! // Create format manager
//! let manager = FormatManager::new();
//!
//! // Detect format from file
//! let format = manager.detect_format(Path::new("movie.srt"))?;
//! println!("Detected format: {}", format);
//!
//! // Parse subtitle file
//! let subtitle = manager.parse_file(Path::new("movie.srt"))?;
//! println!("Loaded {} entries", subtitle.entries.len());
//! ```
//!
//! ## Format Conversion
//!
//! ```rust,ignore
//! use subx_cli::core::formats::converter::FormatConverter;
//!
//! let converter = FormatConverter::new();
//!
//! // Convert SRT to ASS format
//! let ass_content = converter.convert(
//!     &srt_subtitle,
//!     SubtitleFormatType::Ass,
//!     None // Use default conversion options
//! )?;
//!
//! // Save converted content
//! std::fs::write("movie.ass", ass_content)?;
//! ```
//!
//! ## Working with Styling Information
//!
//! ```rust,ignore
//! use subx_cli::core::formats::{StylingInfo, SubtitleEntry};
//! use std::time::Duration;
//!
//! // Create a styled subtitle entry
//! let styled_entry = SubtitleEntry {
//!     index: 1,
//!     start_time: Duration::from_secs(10),
//!     end_time: Duration::from_secs(13),
//!     text: "Styled subtitle text".to_string(),
//!     styling: Some(StylingInfo {
//!         font_name: Some("Arial".to_string()),
//!         font_size: Some(20),
//!         color: Some("#FFFFFF".to_string()),
//!         bold: true,
//!         italic: false,
//!         underline: false,
//!     }),
//! };
//! ```
//!
//! # Format-Specific Features
//!
//! ## SRT Format
//! - Simple timing format (hours:minutes:seconds,milliseconds)
//! - Basic text formatting with HTML-like tags
//! - Wide compatibility across media players
//!
//! ## ASS/SSA Format
//! - Advanced styling with fonts, colors, positioning
//! - Animation and transition effects
//! - Karaoke timing support
//! - Multiple style definitions
//!
//! ## VTT Format
//! - Web-optimized format for HTML5 video
//! - CSS-based styling support
//! - Positioning and region definitions
//! - Metadata and chapter support
//!
//! ## SUB Format
//! - Frame-based timing (requires frame rate)
//! - Simple text format
//! - Legacy format support
//!
//! # Error Handling
//!
//! The format system provides comprehensive error handling for:
//! - Invalid file formats or corrupted content
//! - Encoding detection and conversion failures
//! - Timing inconsistencies and overlaps
//! - Missing or invalid styling information
//! - File I/O errors during parsing or saving
//!
//! # Performance Considerations
//!
//! - **Streaming Parsing**: Large files are processed incrementally
//! - **Memory Efficiency**: Minimal memory footprint for subtitle data
//! - **Caching**: Format detection results are cached for performance
//! - **Parallel Processing**: Multiple files can be processed concurrently
//!
//! # Thread Safety
//!
//! All format operations are thread-safe and can be used in concurrent environments.
//! The format manager and converters can be safely shared across threads.

#![allow(dead_code)]

pub mod ass;
pub mod converter;
pub mod encoding;
pub mod manager;
/// SubRip Text (.srt) subtitle format support
pub mod srt;
pub mod styling;
pub mod sub;
pub mod transformers;
pub mod vtt;

use std::time::Duration;

/// Supported subtitle format types with their characteristics and use cases.
///
/// This enum represents the different subtitle formats that SubX can process,
/// each with distinct features, compatibility, and use cases.
///
/// # Format Characteristics
///
/// - **SRT**: Universal compatibility, simple timing, basic formatting
/// - **ASS**: Advanced styling, animations, precise positioning
/// - **VTT**: Web-optimized, CSS styling, HTML5 video integration
/// - **SUB**: Frame-based timing, legacy format support
///
/// # Examples
///
/// ```rust
/// use subx_cli::core::formats::SubtitleFormatType;
///
/// let format = SubtitleFormatType::Srt;
/// assert_eq!(format.as_str(), "srt");
/// assert_eq!(format.to_string(), "srt");
///
/// // Check format capabilities
/// assert!(format.supports_basic_timing());
/// assert!(!format.supports_advanced_styling());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubtitleFormatType {
    /// SubRip Text format (.srt) - Most common subtitle format.
    ///
    /// Features:
    /// - Simple timing format (HH:MM:SS,mmm)
    /// - Basic HTML-like formatting tags
    /// - Universal player compatibility
    /// - Lightweight and fast parsing
    Srt,

    /// Advanced SubStation Alpha format (.ass/.ssa) - Professional subtitling format.
    ///
    /// Features:
    /// - Rich styling with fonts, colors, effects
    /// - Precise positioning and alignment
    /// - Animation and transition support
    /// - Karaoke timing capabilities
    /// - Multiple style definitions
    Ass,

    /// WebVTT format (.vtt) - Web-optimized subtitle format.
    ///
    /// Features:
    /// - CSS-based styling
    /// - Positioning and region support
    /// - Metadata and chapter markers
    /// - HTML5 video integration
    /// - Web accessibility features
    Vtt,

    /// MicroDVD format (.sub) - Frame-based subtitle format.
    ///
    /// Features:
    /// - Frame-based timing (requires FPS)
    /// - Simple text format
    /// - Legacy format support
    /// - Compact file size
    Sub,
}

impl SubtitleFormatType {
    /// Get the format as a lowercase string slice (e.g., "srt").
    ///
    /// This method returns the standard file extension for the format,
    /// which can be used for file naming and format identification.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::core::formats::SubtitleFormatType;
    ///
    /// assert_eq!(SubtitleFormatType::Srt.as_str(), "srt");
    /// assert_eq!(SubtitleFormatType::Ass.as_str(), "ass");
    /// assert_eq!(SubtitleFormatType::Vtt.as_str(), "vtt");
    /// assert_eq!(SubtitleFormatType::Sub.as_str(), "sub");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            SubtitleFormatType::Srt => "srt",
            SubtitleFormatType::Ass => "ass",
            SubtitleFormatType::Vtt => "vtt",
            SubtitleFormatType::Sub => "sub",
        }
    }

    /// Check if the format supports basic timing information.
    ///
    /// All supported formats have basic timing capabilities.
    ///
    /// # Returns
    ///
    /// Always returns `true` for all current formats.
    pub fn supports_basic_timing(&self) -> bool {
        true
    }

    /// Check if the format supports advanced styling features.
    ///
    /// Advanced styling includes fonts, colors, positioning, and effects.
    ///
    /// # Returns
    ///
    /// - `true` for ASS and VTT formats
    /// - `false` for SRT and SUB formats
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::core::formats::SubtitleFormatType;
    ///
    /// assert!(SubtitleFormatType::Ass.supports_advanced_styling());
    /// assert!(SubtitleFormatType::Vtt.supports_advanced_styling());
    /// assert!(!SubtitleFormatType::Srt.supports_advanced_styling());
    /// assert!(!SubtitleFormatType::Sub.supports_advanced_styling());
    /// ```
    pub fn supports_advanced_styling(&self) -> bool {
        matches!(self, SubtitleFormatType::Ass | SubtitleFormatType::Vtt)
    }

    /// Check if the format uses frame-based timing.
    ///
    /// Frame-based timing requires knowledge of the video frame rate
    /// for accurate time calculations.
    ///
    /// # Returns
    ///
    /// - `true` for SUB format
    /// - `false` for all other formats
    pub fn uses_frame_timing(&self) -> bool {
        matches!(self, SubtitleFormatType::Sub)
    }
}

impl std::fmt::Display for SubtitleFormatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unified subtitle data structure containing entries, metadata, and format information.
///
/// This structure represents a complete subtitle file in memory, providing a
/// format-agnostic representation that can be converted between different
/// subtitle formats while preserving as much information as possible.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::formats::{Subtitle, SubtitleEntry, SubtitleMetadata, SubtitleFormatType};
/// use std::time::Duration;
///
/// let subtitle = Subtitle {
///     entries: vec![
///         SubtitleEntry {
///             index: 1,
///             start_time: Duration::from_secs(10),
///             end_time: Duration::from_secs(13),
///             text: "Hello, world!".to_string(),
///             styling: None,
///         }
///     ],
///     metadata: SubtitleMetadata {
///         title: Some("Movie Title".to_string()),
///         language: Some("en".to_string()),
///         encoding: "UTF-8".to_string(),
///         frame_rate: Some(23.976),
///         original_format: SubtitleFormatType::Srt,
///     },
///     format: SubtitleFormatType::Srt,
/// };
///
/// println!("Subtitle has {} entries", subtitle.entries.len());
/// ```
#[derive(Debug, Clone)]
pub struct Subtitle {
    /// Collection of subtitle entries with timing and text content.
    pub entries: Vec<SubtitleEntry>,

    /// Metadata information about the subtitle file.
    pub metadata: SubtitleMetadata,

    /// Current format type of the subtitle data.
    pub format: SubtitleFormatType,
}

impl Subtitle {
    /// Create a new subtitle with the given format and metadata.
    ///
    /// # Arguments
    ///
    /// * `format` - The subtitle format type
    /// * `metadata` - Metadata associated with the subtitle
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use subx_cli::core::formats::{Subtitle, SubtitleMetadata, SubtitleFormatType};
    ///
    /// let metadata = SubtitleMetadata::default();
    /// let subtitle = Subtitle::new(SubtitleFormatType::Srt, metadata);
    /// assert_eq!(subtitle.entries.len(), 0);
    /// ```
    pub fn new(format: SubtitleFormatType, metadata: SubtitleMetadata) -> Self {
        Self {
            entries: Vec::new(),
            metadata,
            format,
        }
    }

    /// Get the total duration of the subtitle file.
    ///
    /// Returns the time span from the first entry's start time to the
    /// last entry's end time, or zero if there are no entries.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let duration = subtitle.total_duration();
    /// println!("Subtitle duration: {:.2} seconds", duration.as_secs_f32());
    /// ```
    pub fn total_duration(&self) -> Duration {
        if self.entries.is_empty() {
            return Duration::ZERO;
        }

        let first_start = self.entries.first().unwrap().start_time;
        let last_end = self.entries.last().unwrap().end_time;
        last_end.saturating_sub(first_start)
    }

    /// Check if subtitle entries have any timing overlaps.
    ///
    /// Returns `true` if any entry's start time is before the previous
    /// entry's end time, indicating overlapping subtitles.
    pub fn has_overlaps(&self) -> bool {
        for window in self.entries.windows(2) {
            if window[1].start_time < window[0].end_time {
                return true;
            }
        }
        false
    }

    /// Sort entries by start time to ensure chronological order.
    ///
    /// This method is useful after manual manipulation of entries
    /// or when merging subtitles from multiple sources.
    pub fn sort_entries(&mut self) {
        self.entries.sort_by_key(|entry| entry.start_time);

        // Re-index entries after sorting
        for (index, entry) in self.entries.iter_mut().enumerate() {
            entry.index = index + 1;
        }
    }
}

/// Single subtitle entry containing timing, index, and text information.
///
/// This structure represents an individual subtitle entry with its timing,
/// content, and optional styling information. It provides the basic building
/// block for all subtitle formats.
///
/// # Timing Constraints
///
/// - `start_time` must be less than `end_time`
/// - Times are represented as `Duration` from the beginning of the media
/// - Minimum recommended duration is 1 second for readability
/// - Maximum recommended duration is 7 seconds for standard subtitles
///
/// # Text Content
///
/// - Supports Unicode text for international character sets
/// - May contain format-specific markup (HTML tags for SRT, ASS tags for ASS format)
/// - Line breaks are preserved and format-dependent (\n, \N, or <br>)
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::formats::{SubtitleEntry, StylingInfo};
/// use std::time::Duration;
///
/// // Basic subtitle entry
/// let entry = SubtitleEntry {
///     index: 1,
///     start_time: Duration::from_millis(10500), // 10.5 seconds
///     end_time: Duration::from_millis(13750),   // 13.75 seconds
///     text: "Hello, world!".to_string(),
///     styling: None,
/// };
///
/// // Entry with styling
/// let styled_entry = SubtitleEntry {
///     index: 2,
///     start_time: Duration::from_secs(15),
///     end_time: Duration::from_secs(18),
///     text: "<b>Bold text</b>".to_string(),
///     styling: Some(StylingInfo {
///         bold: true,
///         ..Default::default()
///     }),
/// };
///
/// assert_eq!(entry.duration(), Duration::from_millis(3250));
/// assert!(entry.is_valid_timing());
/// ```
#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    /// Sequential number of the subtitle entry (1-based indexing).
    ///
    /// This index is used for ordering and reference purposes.
    /// Most formats expect continuous numbering starting from 1.
    pub index: usize,

    /// Start timestamp of the subtitle entry.
    ///
    /// Represents when the subtitle should first appear on screen,
    /// measured from the beginning of the media file.
    pub start_time: Duration,

    /// End timestamp of the subtitle entry.
    ///
    /// Represents when the subtitle should disappear from screen.
    /// Must be greater than `start_time`.
    pub end_time: Duration,

    /// Text content of the subtitle entry.
    ///
    /// May contain format-specific markup for styling and line breaks.
    /// Unicode content is fully supported for international subtitles.
    pub text: String,

    /// Optional styling information for the subtitle entry.
    ///
    /// Contains font, color, and formatting information. Not all formats
    /// support styling, and some styling may be lost during conversion.
    pub styling: Option<StylingInfo>,
}

impl SubtitleEntry {
    /// Create a new subtitle entry with basic timing and text.
    ///
    /// # Arguments
    ///
    /// * `index` - Sequential number of the entry (1-based)
    /// * `start_time` - When the subtitle should appear
    /// * `end_time` - When the subtitle should disappear
    /// * `text` - The subtitle text content
    ///
    /// # Panics
    ///
    /// Panics if `start_time >= end_time`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use subx_cli::core::formats::SubtitleEntry;
    /// use std::time::Duration;
    ///
    /// let entry = SubtitleEntry::new(
    ///     1,
    ///     Duration::from_secs(10),
    ///     Duration::from_secs(13),
    ///     "Hello!".to_string()
    /// );
    /// ```
    pub fn new(index: usize, start_time: Duration, end_time: Duration, text: String) -> Self {
        assert!(start_time < end_time, "Start time must be before end time");

        Self {
            index,
            start_time,
            end_time,
            text,
            styling: None,
        }
    }

    /// Calculate the duration of this subtitle entry.
    ///
    /// Returns the time span from start to end of the subtitle.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(entry.duration(), Duration::from_secs(3));
    /// ```
    pub fn duration(&self) -> Duration {
        self.end_time.saturating_sub(self.start_time)
    }

    /// Check if the timing of this entry is valid.
    ///
    /// Returns `true` if start_time < end_time and both are valid durations.
    pub fn is_valid_timing(&self) -> bool {
        self.start_time < self.end_time
    }

    /// Check if this entry overlaps with another entry.
    ///
    /// # Arguments
    ///
    /// * `other` - Another subtitle entry to check against
    ///
    /// # Returns
    ///
    /// Returns `true` if the time ranges overlap.
    pub fn overlaps_with(&self, other: &SubtitleEntry) -> bool {
        self.start_time < other.end_time && other.start_time < self.end_time
    }

    /// Get the text content without any format-specific markup.
    ///
    /// Removes common formatting tags like HTML tags for SRT format.
    /// This is useful for text analysis and search operations.
    pub fn plain_text(&self) -> String {
        // Basic HTML tag removal for SRT format
        let mut text = self.text.clone();

        // Remove common HTML tags
        let tags = [
            "<b>",
            "</b>",
            "<i>",
            "</i>",
            "<u>",
            "</u>",
            "<font[^>]*>",
            "</font>",
            "<br>",
            "<br/>",
        ];

        for tag in &tags {
            if tag.contains('[') {
                // Use regex for complex patterns (simplified for example)
                text = text.replace(tag, "");
            } else {
                text = text.replace(tag, " ");
            }
        }

        // Clean up extra whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// Metadata associated with a subtitle file, containing format and content information.
///
/// This structure holds descriptive information about the subtitle file that
/// may be embedded in the file format or derived during processing. It helps
/// maintain context during format conversions and provides useful information
/// for subtitle management.
///
/// # Fields Description
///
/// - `title`: Optional title of the media or subtitle content
/// - `language`: Language code (ISO 639-1/639-3) for the subtitle content
/// - `encoding`: Character encoding used in the original file
/// - `frame_rate`: Video frame rate for frame-based timing formats
/// - `original_format`: The source format before any conversions
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::formats::{SubtitleMetadata, SubtitleFormatType};
///
/// let metadata = SubtitleMetadata {
///     title: Some("Episode 1".to_string()),
///     language: Some("en".to_string()),
///     encoding: "UTF-8".to_string(),
///     frame_rate: Some(23.976),
///     original_format: SubtitleFormatType::Srt,
/// };
///
/// assert!(metadata.is_frame_based());
/// assert_eq!(metadata.display_name(), "Episode 1 (English)");
/// ```
#[derive(Debug, Clone)]
pub struct SubtitleMetadata {
    /// Optional title of the subtitle content or associated media.
    ///
    /// This may be extracted from the subtitle file header or derived
    /// from the filename. Used for display and organization purposes.
    pub title: Option<String>,

    /// Language code for the subtitle content.
    ///
    /// Uses ISO 639-1 (2-letter) or ISO 639-3 (3-letter) codes.
    /// Examples: "en", "zh", "ja", "chi", "eng"
    pub language: Option<String>,

    /// Character encoding of the original subtitle file.
    ///
    /// Common values: "UTF-8", "UTF-16", "GB2312", "BIG5", "Shift_JIS"
    /// This information is crucial for proper text decoding.
    pub encoding: String,

    /// Video frame rate for frame-based timing calculations.
    ///
    /// Required for SUB format and useful for timing validation.
    /// Common values: 23.976, 24.0, 25.0, 29.97, 30.0
    pub frame_rate: Option<f32>,

    /// Original format type before any conversions.
    ///
    /// Tracks the source format to maintain conversion history
    /// and format-specific feature compatibility.
    pub original_format: SubtitleFormatType,
}

impl SubtitleMetadata {
    /// Create new metadata with default values and specified format.
    ///
    /// # Arguments
    ///
    /// * `format` - The original format type
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let metadata = SubtitleMetadata::new(SubtitleFormatType::Srt);
    /// assert_eq!(metadata.encoding, "UTF-8");
    /// ```
    pub fn new(format: SubtitleFormatType) -> Self {
        Self {
            title: None,
            language: None,
            encoding: "UTF-8".to_string(),
            frame_rate: None,
            original_format: format,
        }
    }

    /// Check if the subtitle uses frame-based timing.
    ///
    /// Returns `true` if the format requires frame rate information.
    pub fn is_frame_based(&self) -> bool {
        self.original_format.uses_frame_timing()
    }

    /// Generate a display-friendly name for the subtitle.
    ///
    /// Combines title and language information for user presentation.
    ///
    /// # Returns
    ///
    /// A formatted string like "Title (Language)" or just "Language" if no title.
    pub fn display_name(&self) -> String {
        match (&self.title, &self.language) {
            (Some(title), Some(lang)) => format!("{} ({})", title, lang.to_uppercase()),
            (Some(title), None) => title.clone(),
            (None, Some(lang)) => lang.to_uppercase(),
            (None, None) => "Unknown".to_string(),
        }
    }

    /// Check if the metadata contains complete information.
    ///
    /// Returns `true` if title, language, and frame rate (when needed) are set.
    pub fn is_complete(&self) -> bool {
        self.title.is_some()
            && self.language.is_some()
            && (!self.is_frame_based() || self.frame_rate.is_some())
    }
}

impl Default for SubtitleMetadata {
    fn default() -> Self {
        Self::new(SubtitleFormatType::Srt)
    }
}

/// Optional styling information for subtitle entries with comprehensive formatting support.
///
/// This structure contains visual formatting information that can be applied to
/// subtitle text. Not all formats support all styling options, and some styling
/// may be lost during format conversions.
///
/// # Format Support
///
/// - **SRT**: Limited support (bold, italic, underline via HTML tags)
/// - **ASS**: Full support for all styling options plus advanced features
/// - **VTT**: Good support via CSS-style declarations
/// - **SUB**: No styling support (ignored)
///
/// # Color Format
///
/// Colors can be specified in various formats:
/// - Hex: "#FF0000", "#ff0000"
/// - RGB: "rgb(255, 0, 0)"
/// - Named: "red", "blue", "white"
/// - ASS format: "&H0000FF&" (BGR order)
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::formats::StylingInfo;
///
/// // Basic text styling
/// let basic_style = StylingInfo {
///     bold: true,
///     italic: false,
///     underline: false,
///     ..Default::default()
/// };
///
/// // Complete styling with font and color
/// let full_style = StylingInfo {
///     font_name: Some("Arial".to_string()),
///     font_size: Some(20),
///     color: Some("#FFFFFF".to_string()),
///     bold: true,
///     italic: false,
///     underline: false,
/// };
///
/// assert!(full_style.has_font_styling());
/// assert!(full_style.has_text_decoration());
/// ```
#[derive(Debug, Clone, Default)]
pub struct StylingInfo {
    /// Font family name for the subtitle text.
    ///
    /// Common fonts: "Arial", "Times New Roman", "Helvetica", "SimHei"
    /// Some formats may fall back to default fonts if not available.
    pub font_name: Option<String>,

    /// Font size in points or pixels (format-dependent).
    ///
    /// Typical ranges: 12-24 for normal subtitles, larger for accessibility.
    /// The exact interpretation depends on the target format.
    pub font_size: Option<u32>,

    /// Text color specification.
    ///
    /// Supports multiple formats: hex (#FF0000), RGB, named colors.
    /// Default is usually white (#FFFFFF) for video subtitles.
    pub color: Option<String>,

    /// Whether the text should be rendered in bold weight.
    pub bold: bool,

    /// Whether the text should be rendered in italic style.
    pub italic: bool,

    /// Whether the text should have underline decoration.
    pub underline: bool,
}

impl StylingInfo {
    /// Create new styling with only text decorations.
    ///
    /// # Arguments
    ///
    /// * `bold` - Apply bold weight
    /// * `italic` - Apply italic style
    /// * `underline` - Apply underline decoration
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let style = StylingInfo::new(true, false, false); // Bold only
    /// ```
    pub fn new(bold: bool, italic: bool, underline: bool) -> Self {
        Self {
            font_name: None,
            font_size: None,
            color: None,
            bold,
            italic,
            underline,
        }
    }

    /// Check if any font-related styling is applied.
    ///
    /// Returns `true` if font name, size, or color is specified.
    pub fn has_font_styling(&self) -> bool {
        self.font_name.is_some() || self.font_size.is_some() || self.color.is_some()
    }

    /// Check if any text decoration is applied.
    ///
    /// Returns `true` if bold, italic, or underline is enabled.
    pub fn has_text_decoration(&self) -> bool {
        self.bold || self.italic || self.underline
    }

    /// Check if any styling is applied at all.
    ///
    /// Returns `true` if either font styling or text decoration is present.
    pub fn has_any_styling(&self) -> bool {
        self.has_font_styling() || self.has_text_decoration()
    }

    /// Convert color to hex format if possible.
    ///
    /// Attempts to normalize the color specification to #RRGGBB format.
    /// Returns the original color string if conversion is not possible.
    pub fn normalized_color(&self) -> Option<String> {
        self.color.as_ref().map(|color| {
            if color.starts_with('#') && color.len() == 7 {
                color.to_uppercase()
            } else if color.starts_with("rgb(") {
                // Basic RGB parsing (simplified)
                color.clone() // Would need proper RGB parsing
            } else {
                // Named colors - would need color name mapping
                color.clone()
            }
        })
    }

    /// Generate CSS-style representation of the styling.
    ///
    /// Creates a CSS declaration block that can be used for VTT format
    /// or web-based subtitle rendering.
    ///
    /// # Returns
    ///
    /// CSS string like "font-family: Arial; font-weight: bold; color: #FF0000;"
    pub fn to_css(&self) -> String {
        let mut css = Vec::new();

        if let Some(font) = &self.font_name {
            css.push(format!("font-family: {}", font));
        }

        if let Some(size) = self.font_size {
            css.push(format!("font-size: {}pt", size));
        }

        if let Some(color) = &self.color {
            css.push(format!("color: {}", color));
        }

        if self.bold {
            css.push("font-weight: bold".to_string());
        }

        if self.italic {
            css.push("font-style: italic".to_string());
        }

        if self.underline {
            css.push("text-decoration: underline".to_string());
        }

        css.join("; ")
    }
}

/// Trait defining the interface for subtitle format parsing, serialization, and detection.
///
/// This trait provides a unified interface for working with different subtitle formats.
/// Each format implementation provides specific parsing and serialization logic while
/// maintaining a consistent API for format detection and conversion.
///
/// # Implementation Requirements
///
/// Implementors must provide:
/// - **Parsing**: Convert raw text content to structured `Subtitle` data
/// - **Serialization**: Convert structured data back to format-specific text
/// - **Detection**: Identify if content belongs to this format
/// - **Metadata**: Format name and supported file extensions
///
/// # Format Detection Priority
///
/// When multiple formats claim to support content, detection should be:
/// 1. **Strict**: Prefer specific format markers over generic patterns
/// 2. **Fast**: Use lightweight checks before expensive parsing
/// 3. **Reliable**: Minimize false positives for robust format identification
///
/// # Error Handling
///
/// All parsing and serialization methods should return `crate::Result<T>` to
/// provide detailed error information about format-specific failures.
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::formats::{SubtitleFormat, Subtitle};
///
/// struct MyFormat;
///
/// impl SubtitleFormat for MyFormat {
///     fn parse(&self, content: &str) -> crate::Result<Subtitle> {
///         // Format-specific parsing logic
///         todo!()
///     }
///     
///     fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String> {
///         // Format-specific serialization logic
///         todo!()
///     }
///     
///     fn detect(&self, content: &str) -> bool {
///         // Check for format-specific markers
///         content.contains("my_format_marker")
///     }
///     
///     fn format_name(&self) -> &'static str {
///         "My Format"
///     }
///     
///     fn file_extensions(&self) -> &'static [&'static str] {
///         &["myf"]
///     }
/// }
///
/// // Usage
/// let format = MyFormat;
/// let content = "...subtitle content...";
///
/// if format.detect(content) {
///     let subtitle = format.parse(content)?;
///     println!("Parsed {} entries", subtitle.entries.len());
/// }
/// ```
pub trait SubtitleFormat {
    /// Parse subtitle content into a structured `Subtitle` data structure.
    ///
    /// This method converts raw subtitle file content into the unified
    /// `Subtitle` representation, handling format-specific timing,
    /// text content, and styling information.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw subtitle file content as UTF-8 string
    ///
    /// # Returns
    ///
    /// Returns a `Subtitle` struct containing:
    /// - Parsed subtitle entries with timing and text
    /// - Metadata extracted from the file content
    /// - Format type information
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Content is not valid for this format
    /// - Timing information is malformed or invalid
    /// - Text encoding issues are encountered
    /// - Required format elements are missing
    ///
    /// # Implementation Notes
    ///
    /// - Should be tolerant of minor formatting variations
    /// - Must validate timing consistency (start < end)
    /// - Should preserve as much styling information as possible
    /// - May apply format-specific text normalization
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let srt_content = "1\n00:00:10,500 --> 00:00:13,000\nHello World!\n\n";
    /// let subtitle = format.parse(srt_content)?;
    /// assert_eq!(subtitle.entries.len(), 1);
    /// assert_eq!(subtitle.entries[0].text, "Hello World!");
    /// ```
    fn parse(&self, content: &str) -> crate::Result<Subtitle>;

    /// Serialize a `Subtitle` structure into format-specific text representation.
    ///
    /// This method converts the unified subtitle data structure back into
    /// the raw text format, applying format-specific timing, styling,
    /// and text formatting rules.
    ///
    /// # Arguments
    ///
    /// * `subtitle` - Structured subtitle data to serialize
    ///
    /// # Returns
    ///
    /// Returns a formatted string that can be written to a subtitle file.
    /// The output should be valid for the target format and compatible
    /// with standard media players.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Subtitle data contains invalid timing information
    /// - Styling information cannot be represented in the target format
    /// - Text content contains unsupported characters or formatting
    /// - Required metadata is missing for the format
    ///
    /// # Implementation Notes
    ///
    /// - Should generate clean, standards-compliant output
    /// - Must handle timing precision appropriate for the format
    /// - Should gracefully degrade unsupported styling features
    /// - May need to validate or adjust entry ordering
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let output = format.serialize(&subtitle)?;
    /// std::fs::write("output.srt", output)?;
    /// ```
    fn serialize(&self, subtitle: &Subtitle) -> crate::Result<String>;

    /// Detect whether the provided content matches this subtitle format.
    ///
    /// This method performs lightweight content analysis to determine if
    /// the raw text content belongs to this subtitle format. It should
    /// be fast and reliable for format identification.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw subtitle file content to analyze
    ///
    /// # Returns
    ///
    /// Returns `true` if the content appears to be in this format.
    /// Should minimize false positives while catching valid content.
    ///
    /// # Implementation Guidelines
    ///
    /// - Look for format-specific markers or patterns
    /// - Check timing format conventions
    /// - Validate structural elements (headers, separators)
    /// - Avoid expensive parsing in detection
    /// - Be conservative to prevent false matches
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let srt_content = "1\n00:00:10,500 --> 00:00:13,000\nText\n\n";
    /// assert!(srt_format.detect(srt_content));
    ///
    /// let ass_content = "[Script Info]\nTitle: Test\n[V4+ Styles]\n";
    /// assert!(ass_format.detect(ass_content));
    /// ```
    fn detect(&self, content: &str) -> bool;

    /// Returns the human-readable name of this subtitle format.
    ///
    /// This name is used for user interfaces, error messages, and
    /// format selection dialogs. It should be clear and descriptive.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(srt_format.format_name(), "SubRip Text");
    /// assert_eq!(ass_format.format_name(), "Advanced SubStation Alpha");
    /// assert_eq!(vtt_format.format_name(), "WebVTT");
    /// ```
    fn format_name(&self) -> &'static str;

    /// Returns the list of supported file extensions for this format.
    ///
    /// Extensions should be lowercase without the leading dot.
    /// The primary extension should be listed first.
    ///
    /// # Returns
    ///
    /// Array of extension strings, with primary extension first.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(srt_format.file_extensions(), &["srt"]);
    /// assert_eq!(ass_format.file_extensions(), &["ass", "ssa"]);
    /// assert_eq!(vtt_format.file_extensions(), &["vtt"]);
    /// ```
    fn file_extensions(&self) -> &'static [&'static str];

    /// Check if this format supports advanced styling features.
    ///
    /// Returns `true` if the format can handle fonts, colors, positioning,
    /// and other advanced subtitle styling.
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns `false`. Formats with styling
    /// support should override this method.
    fn supports_styling(&self) -> bool {
        false
    }

    /// Check if this format uses frame-based timing.
    ///
    /// Returns `true` if timing is based on frame numbers rather than
    /// absolute time, requiring frame rate information for conversion.
    ///
    /// # Default Implementation
    ///
    /// The default implementation returns `false`. Frame-based formats
    /// should override this method.
    fn uses_frame_timing(&self) -> bool {
        false
    }
}
