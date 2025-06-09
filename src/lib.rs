//! SubX: Intelligent Subtitle Processing Library
//!
//! SubX is a comprehensive Rust library for intelligent subtitle file processing,
//! featuring AI-powered matching, format conversion, audio synchronization,
//! and advanced encoding detection capabilities.
//!
//! # Key Features
//!
//! - **AI-Powered Matching**: Intelligent subtitle file matching and renaming
//! - **Format Conversion**: Support for multiple subtitle formats (SRT, ASS, VTT, etc.)
//! - **Audio Synchronization**: Advanced audio-subtitle timing adjustment
//! - **Encoding Detection**: Automatic character encoding detection and conversion
//! - **Parallel Processing**: High-performance batch operations
//! - **Configuration Management**: Flexible multi-source configuration system
//!
//! # Architecture Overview
//!
//! The library is organized into several key modules:
//!
//! - [`cli`] - Command-line interface and argument parsing
//! - [`commands`] - Implementation of all SubX commands
//! - [`config`] - Configuration management and validation
//! - [`core`] - Core processing engines (formats, matching, sync)
//! - [`error`] - Comprehensive error handling system
//! - [`services`] - External service integrations (AI, audio processing)
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use subx_cli::config::load_config;
//! 
//! // Load configuration
//! let config = load_config().expect("Failed to load configuration");
//! 
//! // Use the configuration for processing...
//! ```
//!
//! # Error Handling
//!
//! All operations return a [`Result<T>`] type that wraps [`error::SubXError`]:
//!
//! ```rust
//! use subx_cli::{Result, error::SubXError};
//!
//! fn example_operation() -> Result<String> {
//!     // This could fail with various error types
//!     Err(SubXError::config("Missing configuration"))
//! }
//! ```
//!
//! # Configuration
//!
//! SubX supports multi-source configuration loading:
//!
//! ```rust,no_run
//! use subx_cli::config::source::FileSource;
//! use std::path::PathBuf;
//!
//! // Create a file source
//! let file_source = FileSource::new(PathBuf::from("config.toml"));
//! 
//! // Use with configuration manager...
//! ```
//!     .add_source(Box::new(FileSource::new(
//!         PathBuf::from("config.toml")
//!     )));
//!
//! // Load and access configuration
//! manager.load().expect("Failed to load config");
//! let config = manager.config();
//! ```
//!
//! # Performance Considerations
//!
//! - Use [`core::parallel`] for batch operations on large file sets
//! - Configure appropriate cache settings for repeated operations
//! - Consider memory usage when processing large audio files
//!
//! # Thread Safety
//!
//! The library is designed to be thread-safe where appropriate:
//! - Configuration manager uses `Arc<RwLock<T>>` for shared state
//! - File operations include rollback capabilities for atomicity
//! - Parallel processing uses safe concurrency patterns
//!
//! # Feature Flags
//!
//! SubX supports several optional features:
//! - `ai` - AI service integrations (default)
//! - `audio` - Audio processing capabilities (default)
//! - `parallel` - Parallel processing support (default)
#![allow(
    clippy::new_without_default,
    clippy::manual_clamp,
    clippy::useless_vec,
    clippy::items_after_test_module,
    clippy::needless_borrow
)]

/// Library version string.
///
/// This constant provides the current version of the SubX library,
/// automatically populated from `Cargo.toml` at compile time.
///
/// # Examples
///
/// ```rust
/// use subx_cli::VERSION;
///
/// println!("SubX version: {}", VERSION);
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod cli;
pub mod commands;
pub mod config;
pub use config::{init_config_manager, load_config};
pub mod core;
pub mod error;
/// Convenient type alias for `Result<T, SubXError>`.
///
/// This type alias simplifies error handling throughout the SubX library
/// by providing a default error type for all fallible operations.
pub type Result<T> = error::SubXResult<T>;

pub mod services;
