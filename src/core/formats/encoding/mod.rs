//! Character encoding detection and conversion utilities for subtitle files.
//!
//! This module provides comprehensive tools for handling various character encodings
//! commonly found in subtitle files, including automatic detection and conversion
//! between different encoding formats.
//!
//! # Main Components
//!
//! - [`analyzer`] - Statistical analysis of file content for encoding detection
//! - [`charset`] - Character set definitions and encoding information
//! - [`converter`] - Encoding conversion functionality
//! - [`detector`] - High-level encoding detection interface
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::core::formats::encoding::{EncodingDetector, EncodingConverter, Charset};
//!
//! // Detect encoding of a subtitle file
//! let detector = EncodingDetector::new()?;
//! let content = std::fs::read("subtitle.srt")?;
//! let detected = detector.detect_encoding(&content)?;
//!
//! // Convert to UTF-8 if needed
//! if detected.charset != Charset::Utf8 {
//!     let converter = EncodingConverter::new();
//!     let converted = converter.convert_to_utf8(&content, &detected.charset)?;
//!     println!("Converted {} bytes", converted.bytes_processed);
//! }
//! ```

/// Statistical analysis of file content for encoding detection
pub mod analyzer;

/// Character set definitions and encoding information  
pub mod charset;

/// Encoding conversion functionality
pub mod converter;

/// High-level encoding detection interface
pub mod detector;

pub use analyzer::{ByteAnalyzer, StatisticalAnalyzer};
pub use charset::{Charset, EncodingInfo};
pub use converter::{ConversionResult, EncodingConverter};
pub use detector::EncodingDetector;
