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
pub(crate) mod line_endings;
pub mod manager;
/// SubRip Text (.srt) subtitle format support
pub mod srt;
pub mod styling;
pub mod sub;
pub mod transformers;
pub mod vtt;

mod types;
pub use types::*;

#[cfg(all(test, feature = "slow-tests"))]
pub(crate) mod tests_support;
