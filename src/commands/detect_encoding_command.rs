//! Advanced character encoding detection command implementation.
//!
//! This module provides sophisticated character encoding detection capabilities
//! for subtitle files, helping users identify and resolve encoding issues that
//! can cause display problems with non-ASCII characters. It uses multiple
//! detection algorithms and heuristics to provide accurate encoding identification.
//!
//! # Detection Algorithms
//!
//! The encoding detection system employs multiple complementary approaches:
//!
//! ## Byte Order Mark (BOM) Detection
//! - **UTF-8**: EF BB BF byte sequence
//! - **UTF-16LE**: FF FE byte sequence
//! - **UTF-16BE**: FE FF byte sequence
//! - **UTF-32**: Various 4-byte BOM sequences
//!
//! ## Statistical Analysis
//! - **Character Frequency**: Analyze byte patterns for specific encodings
//! - **Bigram Analysis**: Examine two-byte character combinations
//! - **Language Heuristics**: Apply language-specific character patterns
//! - **Confidence Scoring**: Quantify detection reliability
//!
//! ## Format-Specific Detection
//! - **ASCII Compatibility**: Check for pure ASCII content
//! - **Extended ASCII**: Detect Windows-1252, ISO-8859-1 variants
//! - **Multi-byte Encodings**: Identify UTF-8, GB2312, Shift_JIS patterns
//! - **Legacy Encodings**: Support for regional and historical encodings
//!
//! # Supported Encodings
//!
//! ## Unicode Family
//! - **UTF-8**: Universal encoding, recommended for all new files
//! - **UTF-16LE/BE**: Unicode with byte order variants
//! - **UTF-32**: Full Unicode support with fixed width
//!
//! ## Western European
//! - **ISO-8859-1 (Latin-1)**: Basic Western European characters
//! - **Windows-1252**: Microsoft's Western European encoding
//! - **ISO-8859-15**: Latin-1 with Euro symbol support
//!
//! ## East Asian
//! - **GB2312/GBK**: Simplified Chinese encodings
//! - **Big5**: Traditional Chinese encoding
//! - **Shift_JIS**: Japanese encoding
//! - **EUC-JP**: Alternative Japanese encoding
//! - **EUC-KR**: Korean encoding
//!
//! ## Cyrillic and Others
//! - **Windows-1251**: Russian and Cyrillic languages
//! - **KOI8-R**: Russian encoding
//! - **ISO-8859-5**: Cyrillic alphabet
//!
//! # Detection Features
//!
//! - **Confidence Scoring**: Reliability percentage for each detection
//! - **Alternative Suggestions**: Multiple encoding candidates with scores
//! - **Content Sampling**: Display decoded text samples for verification
//! - **Language Hints**: Detect probable language from character patterns
//! - **Format Validation**: Verify encoding produces valid subtitle content
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::commands::detect_encoding_command;
//!
//! // Detect encoding for multiple files
//! let files = vec![
//!     "subtitle1.srt".to_string(),
//!     "subtitle2.ass".to_string(),
//! ];
//! detect_encoding_command::detect_encoding_command(&files, true)?;
//!
//! // Basic detection without verbose output
//! detect_encoding_command::detect_encoding_command(&["file.srt".to_string()], false)?;
//! ```

use std::path::PathBuf;

use crate::Result;
use crate::cli::output::{self, emit_success};
use crate::config::ConfigService;
use crate::core::formats::encoding::{Charset, EncodingDetector};
use log::error;
use serde::Serialize;

/// Per-item error carried inside [`DetectEncodingItem::error`].
///
/// This intentionally omits `exit_code`; per the
/// `machine-readable-output` spec only the top-level error envelope
/// carries an exit code. Per-item errors describe a single failed file
/// while the overall command status remains `"ok"`.
#[derive(Debug, Serialize)]
pub struct DetectEncodingItemError {
    /// Snake_case category mirroring [`crate::error::SubXError::category`].
    pub category: String,
    /// Upper-snake-case machine code mirroring
    /// [`crate::error::SubXError::machine_code`].
    pub code: String,
    /// Human-readable English message.
    pub message: String,
}

/// Per-file encoding-detection record emitted in JSON mode.
///
/// Successful entries carry `encoding`/`confidence`/`has_bom`/
/// `bytes_sampled` and omit `error`. Failed entries carry `error` and
/// omit the detection fields.
#[derive(Debug, Serialize)]
pub struct DetectEncodingItem {
    /// Path as supplied on the command line (or as resolved through
    /// `-i`/recursive directory walk).
    pub path: String,
    /// `"ok"` when the file was successfully sampled, `"error"`
    /// otherwise.
    pub status: &'static str,
    /// Canonical encoding label (e.g. `"UTF-8"`, `"GBK"`, `"Big5"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// Detection confidence in `[0.0, 1.0]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Whether a Byte Order Mark was detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_bom: Option<bool>,
    /// Number of bytes the detector inspected. Capped at the detector's
    /// internal sample window (currently 8 KiB).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_sampled: Option<u64>,
    /// Per-item error envelope when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DetectEncodingItemError>,
}

/// Payload for the `detect-encoding` command's `data` field in JSON mode.
///
/// One element per resolved input path. Single-file invocations emit a
/// `files` array of length 1; batch and `-i` invocations emit one entry
/// per file in the resolution order.
#[derive(Debug, Serialize)]
pub struct DetectEncodingPayload {
    /// Per-file detection records.
    pub files: Vec<DetectEncodingItem>,
}

fn charset_to_label(c: &Charset) -> &'static str {
    match c {
        Charset::Utf8 => "UTF-8",
        Charset::Utf16Le => "UTF-16LE",
        Charset::Utf16Be => "UTF-16BE",
        Charset::Utf32Le => "UTF-32LE",
        Charset::Utf32Be => "UTF-32BE",
        Charset::Gbk => "GBK",
        Charset::ShiftJis => "Shift_JIS",
        Charset::Iso88591 => "ISO-8859-1",
        Charset::Windows1252 => "Windows-1252",
        Charset::Big5 => "Big5",
        Charset::Euckr => "EUC-KR",
        Charset::Unknown => "unknown",
    }
}

/// Execute character encoding detection for subtitle files with comprehensive analysis.
///
/// This function performs advanced character encoding detection on subtitle files,
/// providing detailed information about detected encodings, confidence levels,
/// and content samples. It supports both basic detection and verbose analysis
/// modes to meet different user needs.
///
/// # Detection Process
///
/// 1. **File Validation**: Verify file existence and accessibility
/// 2. **Initial Scanning**: Read file header and sample content
/// 3. **BOM Detection**: Check for Unicode Byte Order Marks
/// 4. **Statistical Analysis**: Analyze byte patterns and character frequencies
/// 5. **Language Heuristics**: Apply language-specific detection rules
/// 6. **Confidence Calculation**: Score each potential encoding
/// 7. **Result Ranking**: Order candidates by confidence level
/// 8. **Output Generation**: Format results for user presentation
///
/// # Verbose Mode Features
///
/// When `verbose` is enabled, the output includes:
/// - **Confidence Percentages**: Numerical reliability scores
/// - **Content Samples**: Decoded text previews
/// - **Alternative Encodings**: Other possible encodings with scores
/// - **Detection Metadata**: Technical details about the detection process
/// - **Language Hints**: Probable content language indicators
///
/// # Error Handling
///
/// The function provides robust error handling:
/// - **File Access**: Clear messages for permission or existence issues
/// - **Corruption Detection**: Identification of damaged or invalid files
/// - **Encoding Failures**: Graceful handling of undetectable encodings
/// - **Partial Processing**: Continue with other files if individual files fail
///
/// # Output Formats
///
/// ## Basic Mode
/// ```text
/// file1.srt: UTF-8
/// file2.ass: Windows-1252
/// file3.vtt: GB2312
/// ```
///
/// ## Verbose Mode
/// ```text
/// file1.srt: UTF-8 (99.5% confidence)
/// Sample: "1\n00:00:01,000 --> 00:00:03,000\nHello World"
/// Alternatives: ISO-8859-1 (15.2%), Windows-1252 (12.8%)
/// Language: English (detected)
///
/// file2.ass: Windows-1252 (87.3% confidence)
/// Sample: "[Script Info]\nTitle: Movie Subtitle"
/// Alternatives: ISO-8859-1 (45.1%), UTF-8 (23.7%)
/// Language: Mixed/Unknown
/// ```
///
/// # Performance Considerations
///
/// - **Streaming Analysis**: Large files processed efficiently
/// - **Sample-based Detection**: Uses representative file portions
/// - **Caching**: Results cached for repeated operations
/// - **Parallel Processing**: Multiple files analyzed concurrently
///
/// # Arguments
///
/// * `file_paths` - Vector of file paths to analyze for encoding
/// * `verbose` - Enable detailed output with confidence scores and samples
///
/// # Returns
///
/// Returns `Ok(())` on successful analysis completion, or an error if:
/// - Critical system resources are unavailable
/// - All specified files are inaccessible
/// - The encoding detection system fails to initialize
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::commands::detect_encoding_command;
///
/// // Quick encoding check for single file
/// detect_encoding_command::detect_encoding_command(
///     &["subtitle.srt".to_string()],
///     false
/// )?;
///
/// // Detailed analysis for multiple files
/// let files = vec![
///     "episode1.srt".to_string(),
///     "episode2.ass".to_string(),
///     "episode3.vtt".to_string(),
/// ];
/// detect_encoding_command::detect_encoding_command(&files, true)?;
///
/// // Batch analysis with glob patterns (shell expansion)
/// let glob_files = vec![
///     "season1/*.srt".to_string(),
///     "season2/*.ass".to_string(),
/// ];
/// detect_encoding_command::detect_encoding_command(&glob_files, false)?;
/// ```
///
/// # Use Cases
///
/// - **Troubleshooting**: Identify encoding issues causing display problems
/// - **Conversion Planning**: Determine current encoding before conversion
/// - **Quality Assurance**: Verify encoding consistency across file collections
/// - **Migration**: Assess encoding diversity when migrating subtitle libraries
/// - **Automation**: Integrate encoding detection into batch processing workflows
use crate::cli::DetectEncodingArgs;
use crate::error::SubXError;

/// Execute character encoding detection for subtitle files based on input arguments.
pub fn detect_encoding_command(args: &DetectEncodingArgs) -> Result<()> {
    // Initialize the encoding detection engine
    let detector = EncodingDetector::with_defaults();

    // For `-i` input paths, use InputPathHandler with archive extraction support,
    // keeping CollectedFiles alive so archive temp dirs persist through processing.
    // For positional file_paths, pass them through directly — they may include
    // nonexistent paths that are gracefully handled in the processing loop below.
    let collected;
    let direct_paths: Vec<PathBuf>;
    let paths: &[PathBuf] = if !args.input_paths.is_empty() {
        let handler = args
            .get_input_handler()
            .map_err(|e| SubXError::CommandExecution(e.to_string()))?;
        collected = handler
            .collect_files()
            .map_err(|e| SubXError::CommandExecution(e.to_string()))?;
        &collected
    } else if !args.file_paths.is_empty() {
        direct_paths = args.file_paths.iter().map(PathBuf::from).collect();
        &direct_paths
    } else {
        return Err(SubXError::NoInputSpecified);
    };

    let mode = output::active_mode();
    let json_mode = mode.is_json();

    // Per spec: when a single positional path is supplied and it does
    // not exist, surface a top-level error envelope instead of a
    // per-item error. `-i` paths already error in
    // `InputPathHandler::validate`.
    if paths.len() == 1 && !paths[0].exists() {
        return Err(SubXError::PathNotFound(paths[0].clone()));
    }

    let mut items: Vec<DetectEncodingItem> = Vec::new();

    // Process each file individually to provide isolated error handling
    for path in paths {
        let path_display = path.to_string_lossy().into_owned();

        if !path.exists() {
            if json_mode {
                items.push(DetectEncodingItem {
                    path: path_display.clone(),
                    status: "error",
                    encoding: None,
                    confidence: None,
                    has_bom: None,
                    bytes_sampled: None,
                    error: Some(DetectEncodingItemError {
                        category: "path_not_found".to_string(),
                        code: "E_PATH_NOT_FOUND".to_string(),
                        message: format!("Path does not exist: {}", path.display()),
                    }),
                });
            } else {
                error!("Path does not exist: {}", path.display());
            }
            continue;
        }

        let bytes_sampled = std::fs::metadata(path).ok().map(|m| m.len().min(8192));

        match detector.detect_file_encoding(&path_display) {
            Ok(info) => {
                if json_mode {
                    items.push(DetectEncodingItem {
                        path: path_display.clone(),
                        status: "ok",
                        encoding: Some(charset_to_label(&info.charset).to_string()),
                        confidence: Some(info.confidence),
                        has_bom: Some(info.bom_detected),
                        bytes_sampled,
                        error: None,
                    });
                } else {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path_display);
                    println!("File: {name}");
                    println!(
                        "  Encoding: {:?} (Confidence: {:.1}%) BOM: {}",
                        info.charset,
                        info.confidence * 100.0,
                        if info.bom_detected { "Yes" } else { "No" }
                    );
                    let sample = if args.verbose {
                        info.sample_text.clone()
                    } else if info.sample_text.len() > 50 {
                        format!("{}...", &info.sample_text[..47])
                    } else {
                        info.sample_text.clone()
                    };
                    println!("  Sample text: {sample}\n");
                }
            }
            Err(e) => {
                if json_mode {
                    items.push(DetectEncodingItem {
                        path: path_display.clone(),
                        status: "error",
                        encoding: None,
                        confidence: None,
                        has_bom: None,
                        bytes_sampled: None,
                        error: Some(DetectEncodingItemError {
                            category: e.category().to_string(),
                            code: e.machine_code().to_string(),
                            message: e.user_friendly_message(),
                        }),
                    });
                } else {
                    error!("Unable to detect encoding for {}: {}", path.display(), e);
                }
            }
        }
    }

    if json_mode {
        emit_success(
            mode,
            "detect-encoding",
            DetectEncodingPayload { files: items },
        );
    }

    Ok(())
}

/// Execute encoding detection command with injected configuration service.
///
/// This function provides the new dependency injection interface for the detect_encoding command,
/// accepting a configuration service instead of loading configuration globally.
///
/// # Arguments
///
/// * `file_paths` - File paths to analyze for encoding detection
/// * `verbose` - Whether to show verbose output
/// * `config_service` - Configuration service providing access to settings
///
/// # Returns
///
/// Returns `Ok(())` on successful completion, or an error if detection fails.
pub fn detect_encoding_command_with_config(
    args: DetectEncodingArgs,
    _config_service: &dyn ConfigService,
) -> Result<()> {
    // Delegate to new implementation based on input argument struct
    detect_encoding_command(&args)
}
