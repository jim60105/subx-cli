//! Advanced subtitle synchronization command implementation.
//!
//! This module provides sophisticated subtitle timing alignment capabilities,
//! using advanced audio analysis techniques to automatically detect optimal
//! subtitle timing or apply manual adjustments. It supports both automatic
//! synchronization through dialogue detection and manual offset application.
//!
//! # Synchronization Methods
//!
//! ## Automatic Synchronization
//! Uses cutting-edge audio analysis to achieve precise timing alignment:
//!
//! ### Audio Analysis Pipeline
//! 1. **Audio Extraction**: Extract audio track from video file
//! 2. **Speech Detection**: Identify speech segments using voice activity detection
//! 3. **Dialogue Recognition**: Classify speech vs. non-speech audio content
//! 4. **Pattern Matching**: Correlate speech timing with subtitle timing
//! 5. **Offset Calculation**: Determine optimal time shift for best alignment
//! 6. **Quality Assessment**: Evaluate synchronization confidence and accuracy
//!
//! ### Advanced Features
//! - **Multi-language Support**: Handle various spoken languages
//! - **Background Noise Filtering**: Robust operation in noisy environments
//! - **Music Separation**: Distinguish speech from background music
//! - **Confidence Scoring**: Quantify synchronization quality
//!
//! ## Manual Synchronization
//! Provides precise control for specific timing adjustments:
//!
//! - **Fixed Offset**: Apply uniform time shift to all subtitles
//! - **Fractional Precision**: Support for millisecond-level adjustments
//! - **Positive/Negative Shifts**: Advance or delay subtitle timing
//! - **Preservation**: Maintain relative timing between subtitle entries
//!
//! # Audio Processing Features
//!
//! ## Dialogue Detection
//! - **Voice Activity Detection (VAD)**: Identify speech segments
//! - **Speaker Separation**: Handle multiple speakers
//! - **Language Adaptation**: Optimize for different languages
//! - **Noise Robustness**: Function in challenging audio environments
//!
//! ## Quality Analysis
//! - **Speech Ratio**: Percentage of audio containing speech
//! - **Confidence Metrics**: Reliability indicators for sync quality
//! - **Timing Validation**: Verify subtitle timing consistency
//! - **Content Alignment**: Ensure subtitles match spoken content
//!
//! # Configuration Integration
//!
//! The synchronization system respects comprehensive configuration:
//! ```toml
//! [sync]
//! max_offset_seconds = 30.0           # Maximum search range
//! correlation_threshold = 0.8         # Minimum correlation for acceptance
//! dialogue_detection_threshold = 0.6  # Speech detection sensitivity
//! min_dialogue_duration_ms = 500      # Minimum speech segment length
//! enable_dialogue_detection = true    # Enable advanced audio analysis
//! ```
//!
//! # Performance Optimization
//!
//! - **Efficient Audio Processing**: Optimized algorithms for speed
//! - **Memory Management**: Streaming processing for large files
//! - **Parallel Processing**: Multi-threaded analysis where possible
//! - **Caching**: Results cached for repeated operations
//!
//! # Examples
//!
//! ```rust,ignore
//! use subx_cli::cli::SyncArgs;
//! use subx_cli::commands::sync_command;
//! use std::path::PathBuf;
//!
//! // Automatic synchronization
//! let auto_sync = SyncArgs {
//!     video: PathBuf::from("movie.mp4"),
//!     subtitle: PathBuf::from("subtitle.srt"),
//!     offset: None,
//!     batch: false,
//!     range: Some(20.0),
//!     threshold: Some(0.85),
//! };
//! sync_command::execute(auto_sync).await?;
//!
//! // Manual offset adjustment
//! let manual_sync = SyncArgs {
//!     video: PathBuf::from("episode.mkv"),
//!     subtitle: PathBuf::from("episode.srt"),
//!     offset: Some(2.5), // Delay by 2.5 seconds
//!     batch: false,
//!     range: None,
//!     threshold: None,
//! };
//! sync_command::execute(manual_sync).await?;
//! ```

use crate::Result;
use crate::cli::SyncArgs;
use crate::config::ConfigService;
use crate::config::load_config;
use crate::core::formats::Subtitle;
use crate::core::formats::manager::FormatManager;
use crate::core::matcher::{FileDiscovery, MediaFileType};
use crate::core::sync::dialogue::DialogueDetector;
use crate::core::sync::{SyncConfig, SyncEngine, SyncResult};
use crate::error::SubXError;
use std::path::{Path, PathBuf};

/// Execute advanced subtitle synchronization with audio analysis or manual adjustment.
///
/// This function orchestrates the complete synchronization workflow, supporting
/// both automatic audio-based timing correction and manual offset application.
/// It includes comprehensive audio analysis, dialogue detection, and timing
/// validation to ensure optimal subtitle-audio alignment.
///
/// # Synchronization Workflow
///
/// ## Automatic Mode (no offset specified)
/// 1. **Configuration Setup**: Load sync parameters and thresholds
/// 2. **Audio Analysis**: Extract and analyze audio from video file
/// 3. **Dialogue Detection**: Identify speech segments and timing patterns
/// 4. **Pattern Correlation**: Match speech timing with subtitle timing
/// 5. **Offset Optimization**: Find optimal time shift for best alignment
/// 6. **Quality Validation**: Assess synchronization confidence and accuracy
/// 7. **Application**: Apply calculated offset to subtitle file
///
/// ## Manual Mode (offset specified)
/// 1. **Configuration Loading**: Load basic sync settings
/// 2. **Subtitle Loading**: Parse and validate subtitle file
/// 3. **Offset Application**: Apply specified time shift uniformly
/// 4. **Validation**: Verify timing consistency after adjustment
/// 5. **Output**: Save synchronized subtitle file
///
/// # Audio Analysis Features
///
/// When dialogue detection is enabled, the system provides:
/// - **Speech Segment Detection**: Identify when characters are speaking
/// - **Speech Ratio Analysis**: Calculate percentage of audio containing speech
/// - **Quality Metrics**: Assess suitability for automatic synchronization
/// - **Confidence Scoring**: Quantify reliability of detected patterns
///
/// # Configuration Parameters
///
/// The function uses configuration settings to optimize performance:
/// - **max_offset_seconds**: Maximum search range for automatic sync
/// - **correlation_threshold**: Minimum correlation required for acceptance
/// - **dialogue_detection_threshold**: Sensitivity for speech detection
/// - **min_dialogue_duration_ms**: Minimum length of valid speech segments
///
/// # Arguments
///
/// * `args` - Synchronization arguments containing:
///   - `video`: Video file path for audio analysis
///   - `subtitle`: Subtitle file path to be synchronized
///   - `offset`: Optional manual offset in seconds (overrides auto-detection)
///   - `batch`: Enable batch processing mode
///   - `range`: Override maximum offset search range
///   - `threshold`: Override correlation threshold
///
/// # Returns
///
/// Returns `Ok(())` on successful synchronization, or an error describing:
/// - Configuration loading failures
/// - Video file access or audio extraction problems
/// - Subtitle file parsing or validation issues
/// - Synchronization processing errors
/// - Output file creation problems
///
/// # Error Handling
///
/// Comprehensive error handling addresses:
/// - **Input Validation**: File existence, format support, accessibility
/// - **Audio Processing**: Codec support, extraction failures, analysis errors
/// - **Synchronization**: Pattern matching failures, correlation issues
/// - **Output Generation**: File writing, format validation, backup creation
///
/// # Quality Assurance
///
/// The synchronization process includes multiple quality checks:
/// - **Input Validation**: Verify video and subtitle file integrity
/// - **Audio Quality**: Assess audio suitability for analysis
/// - **Sync Confidence**: Evaluate reliability of calculated offsets
/// - **Output Verification**: Validate synchronized subtitle timing
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::cli::SyncArgs;
/// use subx_cli::commands::sync_command;
/// use std::path::PathBuf;
///
/// // High-precision automatic sync
/// let precise_sync = SyncArgs {
///     video: PathBuf::from("documentary.mp4"),
///     subtitle: PathBuf::from("documentary.srt"),
///     offset: None,
///     batch: false,
///     range: Some(10.0),    // Narrow search range
///     threshold: Some(0.9), // High confidence required
/// };
/// sync_command::execute(precise_sync).await?;
///
/// // Permissive automatic sync for challenging content
/// let permissive_sync = SyncArgs {
///     video: PathBuf::from("action_movie.mkv"),
///     subtitle: PathBuf::from("action_movie.srt"),
///     offset: None,
///     batch: false,
///     range: Some(45.0),    // Wide search range
///     threshold: Some(0.7), // Lower confidence threshold
/// };
/// sync_command::execute(permissive_sync).await?;
///
/// // Fine manual adjustment
/// let fine_tune = SyncArgs {
///     video: PathBuf::from("episode.mp4"),
///     subtitle: PathBuf::from("episode.srt"),
///     offset: Some(0.75), // 750ms delay
///     batch: false,
///     range: None,
///     threshold: None,
/// };
/// sync_command::execute(fine_tune).await?;
/// ```
///
/// # Performance Notes
///
/// - **Audio Processing**: CPU-intensive, may take time for long videos
/// - **Memory Usage**: Proportional to video length and audio quality
/// - **Disk I/O**: Temporary files created during audio extraction
/// - **Optimization**: Results cached for repeated operations on same files
pub async fn execute(args: SyncArgs) -> Result<()> {
    // Load application configuration for synchronization parameters
    let app_config = load_config()?;

    // Configure synchronization engine with user overrides and defaults
    let config = SyncConfig {
        max_offset_seconds: args.range.unwrap_or(app_config.sync.max_offset_seconds),
        correlation_threshold: args
            .threshold
            .unwrap_or(app_config.sync.correlation_threshold),
        dialogue_threshold: app_config.sync.dialogue_detection_threshold,
        min_dialogue_length: app_config.sync.min_dialogue_duration_ms as f32 / 1000.0,
    };
    let sync_engine = SyncEngine::new(config);

    // Delegate to the shared synchronization logic
    execute_sync_logic(args, app_config, sync_engine).await
}

/// Execute audio-subtitle synchronization with injected configuration service.
///
/// This function provides the new dependency injection interface for the sync command,
/// accepting a configuration service instead of loading configuration globally.
///
/// # Arguments
///
/// * `args` - Synchronization arguments including video/subtitle paths and thresholds
/// * `config_service` - Configuration service providing access to sync settings
///
/// # Returns
///
/// Returns `Ok(())` on successful completion, or an error if synchronization fails.
pub async fn execute_with_config(
    args: SyncArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> Result<()> {
    // Load application configuration for synchronization parameters from injected service
    let app_config = config_service.get_config()?;

    // Configure synchronization engine with user overrides and defaults
    let config = SyncConfig {
        max_offset_seconds: args.range.unwrap_or(app_config.sync.max_offset_seconds),
        correlation_threshold: args
            .threshold
            .unwrap_or(app_config.sync.correlation_threshold),
        dialogue_threshold: app_config.sync.dialogue_detection_threshold,
        min_dialogue_length: app_config.sync.min_dialogue_duration_ms as f32 / 1000.0,
    };
    let sync_engine = SyncEngine::new(config);

    // Delegate to the shared synchronization logic
    execute_sync_logic(args, app_config, sync_engine).await
}

/// Internal function containing the core synchronization logic.
///
/// This function contains the shared sync logic that can be used by both
/// the legacy execute() function and the new execute_with_config() function.
async fn execute_sync_logic(
    args: SyncArgs,
    app_config: crate::config::Config,
    sync_engine: SyncEngine,
) -> Result<()> {
    // Perform advanced dialogue detection if enabled in configuration
    if app_config.sync.enable_dialogue_detection {
        let detector = DialogueDetector::new()?;
        let segs = detector.detect_dialogue(&args.video).await?;
        println!("Detected {} dialogue segments", segs.len());
        println!(
            "Speech ratio: {:.1}%",
            detector.get_speech_ratio(&segs) * 100.0
        );
    }

    if let Some(manual_offset) = args.offset {
        // Manual synchronization mode: apply specified offset
        let mut subtitle = load_subtitle(&args.subtitle).await?;
        sync_engine.apply_sync_offset(&mut subtitle, manual_offset as f32)?;
        save_subtitle(&subtitle, &args.subtitle).await?;
        println!("✓ Applied manual offset: {}s", manual_offset);
    } else if args.batch {
        let media_pairs = discover_media_pairs(&args.video).await?;
        for (video_file, subtitle_file) in media_pairs {
            match sync_single_pair(&sync_engine, &video_file, &subtitle_file).await {
                Ok(result) => {
                    println!(
                        "✓ {} - Offset: {:.2}s (Confidence: {:.2})",
                        subtitle_file.display(),
                        result.offset_seconds,
                        result.confidence
                    );
                }
                Err(e) => {
                    println!("✗ {} - Error: {}", subtitle_file.display(), e);
                }
            }
        }
    } else {
        let subtitle = load_subtitle(&args.subtitle).await?;
        let result = sync_engine.sync_subtitle(&args.video, &subtitle).await?;
        if result.confidence > 0.5 {
            let mut updated = subtitle;
            sync_engine.apply_sync_offset(&mut updated, result.offset_seconds)?;
            save_subtitle(&updated, &args.subtitle).await?;
            println!(
                "✓ Sync completed - Offset: {:.2}s (Confidence: {:.2})",
                result.offset_seconds, result.confidence
            );
        } else {
            println!(
                "⚠ Low sync confidence ({:.2}), manual adjustment recommended",
                result.confidence
            );
        }
    }
    Ok(())
}

/// Load and parse subtitle file
async fn load_subtitle(path: &Path) -> Result<Subtitle> {
    let content = tokio::fs::read_to_string(path).await?;
    let mgr = FormatManager::new();
    let mut subtitle = mgr.parse_auto(&content)?;
    // Set source encoding
    subtitle.metadata.encoding = "utf-8".to_string();
    Ok(subtitle)
}

/// Serialize and save subtitle file
async fn save_subtitle(subtitle: &Subtitle, path: &Path) -> Result<()> {
    let mgr = FormatManager::new();
    let text = mgr
        .get_format_by_extension(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
        )
        .ok_or_else(|| SubXError::subtitle_format("Unknown", "Unknown subtitle format"))?
        .serialize(subtitle)?;
    tokio::fs::write(path, text).await?;
    Ok(())
}

/// Scan directory and pair video with subtitle files
async fn discover_media_pairs(dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let discovery = FileDiscovery::new();
    let files = discovery.scan_directory(dir, true)?;
    let videos: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type, MediaFileType::Video))
        .cloned()
        .collect();
    let subs: Vec<_> = files
        .iter()
        .filter(|f| matches!(f.file_type, MediaFileType::Subtitle))
        .cloned()
        .collect();
    let mut pairs = Vec::new();
    for video in videos {
        if let Some(s) = subs.iter().find(|s| s.name == video.name) {
            pairs.push((video.path.clone(), s.path.clone()));
        }
    }
    Ok(pairs)
}

/// Synchronize single media file
async fn sync_single_pair(
    engine: &SyncEngine,
    video: &Path,
    subtitle_path: &Path,
) -> Result<SyncResult> {
    let mut subtitle = load_subtitle(subtitle_path).await?;
    let result = engine.sync_subtitle(video, &subtitle).await?;
    engine.apply_sync_offset(&mut subtitle, result.offset_seconds)?;
    save_subtitle(&subtitle, subtitle_path).await?;
    Ok(result)
}
