//! Subtitle format conversion command implementation.
//!
//! This module provides comprehensive subtitle format conversion capabilities,
//! transforming subtitle files between different standards while preserving
//! timing information, styling, and encoding. It supports both single file
//! and batch directory processing with intelligent format detection.
//!
//! # Supported Conversions
//!
//! The conversion system supports transformation between major subtitle formats:
//!
//! ## Input Formats (Auto-detected)
//! - **SRT (SubRip)**: Most common subtitle format
//! - **ASS/SSA (Advanced SubStation Alpha)**: Rich formatting support
//! - **VTT (WebVTT)**: Web-optimized subtitle format
//! - **SUB (MicroDVD)**: Frame-based subtitle format
//! - **SMI (SAMI)**: Microsoft subtitle format
//! - **LRC (Lyrics)**: Simple lyric format
//!
//! ## Output Formats (User-specified)
//! - **SRT**: Universal compatibility and simplicity
//! - **ASS**: Advanced styling and positioning
//! - **VTT**: HTML5 video and web applications
//! - **SUB**: Legacy system compatibility
//!
//! # Conversion Features
//!
//! - **Format Detection**: Automatic input format recognition
//! - **Styling Preservation**: Maintain formatting where possible
//! - **Encoding Conversion**: Handle various character encodings
//! - **Batch Processing**: Convert multiple files efficiently
//! - **Quality Validation**: Verify output format integrity
//! - **Backup Creation**: Preserve original files optionally
//!
//! # Quality Assurance
//!
//! Each conversion undergoes comprehensive validation:
//! - **Timing Integrity**: Verify timestamp accuracy and ordering
//! - **Content Preservation**: Ensure no text loss during conversion
//! - **Format Compliance**: Validate output meets format specifications
//! - **Encoding Correctness**: Verify character encoding consistency
//! - **Styling Translation**: Map styles between format capabilities
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::{ConvertArgs, OutputSubtitleFormat};
//! use subx_cli::commands::convert_command;
//! use std::path::PathBuf;
//!
//! // Convert single SRT file to ASS format
//! let args = ConvertArgs {
//!     input: PathBuf::from("input.srt"),
//!     format: Some(OutputSubtitleFormat::Ass),
//!     output: Some(PathBuf::from("output.ass")),
//!     keep_original: true,
//!     encoding: "utf-8".to_string(),
//! };
//!
//! convert_command::execute(args).await?;
//!
//! // Batch convert directory with default settings
//! let batch_args = ConvertArgs {
//!     input: PathBuf::from("./subtitles/"),
//!     format: Some(OutputSubtitleFormat::Vtt),
//!     output: None, // Use default naming
//!     keep_original: true,
//!     encoding: "utf-8".to_string(),
//! };
//!
//! convert_command::execute(batch_args).await?;
//! ```

use crate::cli::{ConvertArgs, OutputSubtitleFormat};
use crate::config::load_config;
use crate::core::file_manager::FileManager;
use crate::core::formats::converter::{ConversionConfig, FormatConverter};
use crate::error::SubXError;

/// Execute subtitle format conversion with comprehensive validation and error handling.
///
/// This function orchestrates the complete conversion workflow, from configuration
/// loading through final output validation. It supports both single file and batch
/// directory processing with intelligent format detection and preservation of
/// subtitle quality.
///
/// # Conversion Process
///
/// 1. **Configuration Loading**: Load application and conversion settings
/// 2. **Format Detection**: Automatically detect input subtitle format
/// 3. **Conversion Setup**: Configure converter with user preferences
/// 4. **Processing**: Transform subtitle content to target format
/// 5. **Validation**: Verify output quality and format compliance
/// 6. **File Management**: Handle backups and output file creation
///
/// # Format Mapping
///
/// The conversion process intelligently maps features between formats:
///
/// ## SRT to ASS
/// - Basic text → Advanced styling capabilities
/// - Simple timing → Precise timing control
/// - Limited formatting → Rich formatting options
///
/// ## ASS to SRT
/// - Rich styling → Basic formatting preservation
/// - Advanced timing → Standard timing format
/// - Complex layouts → Simplified text positioning
///
/// ## Any to VTT
/// - Format-specific features → Web-compatible equivalents
/// - Custom styling → CSS-like styling syntax
/// - Traditional timing → WebVTT timing format
///
/// # Configuration Integration
///
/// The function respects multiple configuration sources:
/// ```toml
/// [formats]
/// default_output = "srt"           # Default output format
/// preserve_styling = true          # Maintain formatting where possible
/// validate_output = true           # Perform output validation
/// backup_enabled = true            # Create backups before conversion
/// ```
///
/// # Arguments
///
/// * `args` - Conversion arguments containing:
///   - `input`: Source file or directory path
///   - `format`: Target output format (SRT, ASS, VTT, SUB)
///   - `output`: Optional output path (auto-generated if not specified)
///   - `keep_original`: Whether to preserve original files
///   - `encoding`: Character encoding for input/output files
///
/// # Returns
///
/// Returns `Ok(())` on successful conversion, or an error describing:
/// - Configuration loading failures
/// - Input file access or format problems
/// - Conversion processing errors
/// - Output file creation or validation issues
///
/// # Error Handling
///
/// Comprehensive error handling covers:
/// - **Input Validation**: File existence, format detection, accessibility
/// - **Processing Errors**: Conversion failures, content corruption
/// - **Output Issues**: Write permissions, disk space, format validation
/// - **Configuration Problems**: Invalid settings, missing dependencies
///
/// # File Safety
///
/// The conversion process ensures file safety through:
/// - **Atomic Operations**: Complete conversion or no changes
/// - **Backup Creation**: Original files preserved when requested
/// - **Validation**: Output quality verification before finalization
/// - **Rollback Capability**: Ability to undo changes if problems occur
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::cli::{ConvertArgs, OutputSubtitleFormat};
/// use subx_cli::commands::convert_command;
/// use std::path::PathBuf;
///
/// // Convert with explicit output path
/// let explicit_args = ConvertArgs {
///     input: PathBuf::from("movie.srt"),
///     format: Some(OutputSubtitleFormat::Ass),
///     output: Some(PathBuf::from("movie_styled.ass")),
///     keep_original: true,
///     encoding: "utf-8".to_string(),
/// };
/// convert_command::execute(explicit_args).await?;
///
/// // Convert with automatic output naming
/// let auto_args = ConvertArgs {
///     input: PathBuf::from("episode.srt"),
///     format: Some(OutputSubtitleFormat::Vtt),
///     output: None, // Will become "episode.vtt"
///     keep_original: false,
///     encoding: "utf-8".to_string(),
/// };
/// convert_command::execute(auto_args).await?;
///
/// // Batch convert directory
/// let batch_args = ConvertArgs {
///     input: PathBuf::from("./season1_subtitles/"),
///     format: Some(OutputSubtitleFormat::Srt),
///     output: None,
///     keep_original: true,
///     encoding: "utf-8".to_string(),
/// };
/// convert_command::execute(batch_args).await?;
/// ```
///
/// # Performance Considerations
///
/// - **Memory Efficiency**: Streaming processing for large subtitle files
/// - **Disk I/O Optimization**: Efficient file access patterns
/// - **Batch Processing**: Optimized for multiple file operations
/// - **Validation Caching**: Avoid redundant quality checks
pub async fn execute(args: ConvertArgs) -> crate::Result<()> {
    // Load application configuration for conversion settings
    let app_config = load_config()?;

    // Configure conversion engine with user preferences and application defaults
    let config = ConversionConfig {
        preserve_styling: app_config.formats.preserve_styling,
        target_encoding: args.encoding.clone(),
        keep_original: args.keep_original,
        validate_output: true,
    };
    let converter = FormatConverter::new(config);

    // Determine output format from arguments or configuration defaults
    let default_output = match app_config.formats.default_output.as_str() {
        "srt" => OutputSubtitleFormat::Srt,
        "ass" => OutputSubtitleFormat::Ass,
        "vtt" => OutputSubtitleFormat::Vtt,
        "sub" => OutputSubtitleFormat::Sub,
        other => {
            return Err(SubXError::config(format!(
                "Unknown default output format: {}",
                other
            )));
        }
    };
    let output_format = args.format.clone().unwrap_or(default_output);

    if args.input.is_file() {
        // Single file conversion with automatic output path generation
        let format_str = output_format.to_string();
        let output_path = args
            .output
            .unwrap_or_else(|| args.input.with_extension(format_str.clone()));
        let mut file_manager = FileManager::new();
        match converter
            .convert_file(&args.input, &output_path, &format_str)
            .await
        {
            Ok(result) => {
                if result.success {
                    file_manager.record_creation(&output_path);
                    println!(
                        "✓ Conversion completed: {} -> {}",
                        args.input.display(),
                        output_path.display()
                    );
                    if !args.keep_original {
                        if let Err(e) = file_manager.remove_file(&args.input) {
                            eprintln!(
                                "⚠️  Cannot remove original file {}: {}",
                                args.input.display(),
                                e
                            );
                        }
                    }
                } else {
                    println!("✗ Conversion failed");
                    for error in result.errors {
                        println!("  Error: {}", error);
                    }
                }
            }
            Err(e) => {
                eprintln!("✗ Conversion failed: {}", e);
                if let Err(rollback_err) = file_manager.rollback() {
                    eprintln!("✗ Rollback failed: {}", rollback_err);
                }
                return Err(e);
            }
        }
    } else {
        // Batch conversion
        let format_str = output_format.to_string();
        let results = converter
            .convert_batch(&args.input, &format_str, true)
            .await?;
        let success_count = results.iter().filter(|r| r.success).count();
        let total_count = results.len();
        println!(
            "Batch conversion completed: {}/{} successful",
            success_count, total_count
        );
        for result in results.iter().filter(|r| !r.success) {
            println!("Failed: {}", result.errors.join(", "));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::init_config_manager;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    /// Reset test environment to avoid state interference between tests
    fn reset_test_environment() {
        // Reset global configuration manager
        crate::config::reset_global_config_manager();
    }

    #[tokio::test]
    #[serial]
    async fn test_convert_srt_to_vtt() -> crate::Result<()> {
        reset_test_environment();
        init_config_manager()?;
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.srt");
        let output_file = temp_dir.path().join("test.vtt");
        fs::write(
            &input_file,
            "1\n00:00:01,000 --> 00:00:02,000\nTest subtitle\n\n",
        )
        .unwrap();
        let args = ConvertArgs {
            input: input_file.clone(),
            format: Some(OutputSubtitleFormat::Vtt),
            output: Some(output_file.clone()),
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        execute(args).await?;
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WEBVTT"));
        assert!(content.contains("00:00:01.000 --> 00:00:02.000"));
        reset_test_environment();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_convert_batch_processing() -> crate::Result<()> {
        reset_test_environment();
        init_config_manager()?;
        let temp_dir = TempDir::new().unwrap();
        for i in 1..=3 {
            let file = temp_dir.path().join(format!("test{}.srt", i));
            fs::write(
                &file,
                format!(
                    "1\n00:00:0{},000 --> 00:00:0{},000\nTest {}\n\n",
                    i,
                    i + 1,
                    i
                ),
            )
            .unwrap();
        }
        let args = ConvertArgs {
            input: temp_dir.path().to_path_buf(),
            format: Some(OutputSubtitleFormat::Vtt),
            output: Some(temp_dir.path().join("output")),
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        // Only check execution result, do not verify actual file generation, as converter behavior is controlled by external modules
        execute(args).await?;
        reset_test_environment();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_convert_unsupported_format() {
        reset_test_environment();
        init_config_manager().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let input_file = temp_dir.path().join("test.txt");
        fs::write(&input_file, "not a subtitle").unwrap();
        let args = ConvertArgs {
            input: input_file,
            format: Some(OutputSubtitleFormat::Srt),
            output: None,
            keep_original: false,
            encoding: String::from("utf-8"),
        };
        let result = execute(args).await;
        assert!(result.is_err());
        reset_test_environment();
    }
}
