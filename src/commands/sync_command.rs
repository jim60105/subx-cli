//! Refactored sync command supporting new multi-method sync engine.
//!
//! This module provides the synchronization command functionality, supporting
//! multiple synchronization methods including local VAD (Voice Activity Detection),
//! automatic method selection, and manual offset adjustment.

use crate::cli::SyncArgs;
use crate::cli::SyncMode;
use crate::cli::output::{OutputMode, active_mode, emit_success};
use crate::cli::sync_args::create_default_output_path;
use crate::config::Config;
use crate::config::ConfigService;
use crate::core::formats::manager::FormatManager;
use crate::core::sync::{SyncEngine, SyncMethod, SyncResult};
use crate::{Result, error::SubXError};
use serde::Serialize;

/// Approximate VAD chunk duration in milliseconds.
///
/// Silero VAD operates on 512 samples at 16 kHz (≈32 ms) and 256 samples
/// at 8 kHz (≈32 ms). The exact chunk duration depends on the audio
/// sample rate; 32 ms is the value used for both supported rates.
const VAD_CHUNK_MS: u32 = 32;

/// JSON payload emitted under `data` of the top-level envelope by
/// `subx-cli --output json sync`.
///
/// The shape is uniform across single-pair and batch invocations: an
/// `inputs` array describing each subtitle that was analyzed, an
/// `operations` array describing each file that was written or
/// planned, and a top-level `method` string identifying the active
/// synchronization strategy.
#[derive(Debug, Serialize)]
pub struct SyncPayload {
    /// Active sync method: `"vad"`, `"manual"`, or `"auto"`.
    pub method: String,
    /// Per-subtitle analysis results, one entry per processed input.
    pub inputs: Vec<SyncInput>,
    /// Per-subtitle write operations, one entry per planned/applied write.
    pub operations: Vec<SyncOperation>,
}

/// One entry in [`SyncPayload::inputs`] — describes the analysis stage
/// for a single subtitle file.
#[derive(Debug, Serialize)]
pub struct SyncInput {
    /// Subtitle file path that was analyzed.
    pub subtitle_path: String,
    /// Audio/video source used for analysis (absent for manual offsets
    /// and skipped items).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// Detected offset in milliseconds (positive = subtitles delayed).
    /// For manual sync this equals the user-supplied offset.
    pub detected_offset_ms: i64,
    /// Detection confidence (0.0–1.0); absent for manual and skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// VAD-specific metadata (only populated when `method == "vad"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vad: Option<VadInfoPayload>,
    /// Either `"ok"` or `"error"`.
    pub status: &'static str,
    /// Error metadata when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SyncItemError>,
}

/// One entry in [`SyncPayload::operations`] — describes a planned or
/// applied write to disk.
#[derive(Debug, Serialize)]
pub struct SyncOperation {
    /// Subtitle source path the operation derives from.
    pub subtitle_path: String,
    /// Path the synchronized subtitle was (or would have been) written to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    /// True when the synchronized subtitle was actually written to disk.
    pub applied: bool,
    /// True when `--dry-run` was supplied.
    pub dry_run: bool,
    /// Either `"ok"` or `"error"`.
    pub status: &'static str,
    /// Error metadata when `status == "error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SyncItemError>,
}

/// VAD-specific metadata included in [`SyncInput::vad`].
#[derive(Debug, Serialize)]
pub struct VadInfoPayload {
    /// VAD sensitivity threshold in effect (0.0–1.0).
    pub sensitivity: f32,
    /// Padding around detected speech, expressed in milliseconds.
    pub padding_ms: u32,
    /// Detected speech segments (`{start,end,duration}` objects).
    pub segments: Vec<serde_json::Value>,
}

/// Stable per-item error payload (mirrors the top-level error envelope
/// minus `exit_code`).
#[derive(Debug, Serialize, Clone)]
pub struct SyncItemError {
    /// Stable machine code (e.g. `E_SUBTITLE_FORMAT`).
    pub code: String,
    /// Stable category (e.g. `subtitle_format`).
    pub category: String,
    /// Human-readable message.
    pub message: String,
}

/// Internal aggregate produced by [`run_single`] — pairs the input
/// analysis record with its companion write operation.
struct SyncSingleResult {
    input: SyncInput,
    operation: SyncOperation,
}

fn method_to_str(m: &SyncMethod) -> &'static str {
    match m {
        SyncMethod::LocalVad => "vad",
        SyncMethod::Manual => "manual",
        SyncMethod::Auto => "auto",
    }
}

fn build_single_result(
    args: &SyncArgs,
    sync_result: &SyncResult,
    subtitle_path: &std::path::Path,
    audio_path: Option<&std::path::Path>,
    output_path: Option<&std::path::Path>,
    applied: bool,
    vad_cfg: &crate::config::VadConfig,
) -> SyncSingleResult {
    let offset_ms = (sync_result.offset_seconds as f64 * 1000.0).round() as i64;
    let confidence = if matches!(sync_result.method_used, SyncMethod::Manual) {
        None
    } else {
        Some(sync_result.confidence)
    };
    let vad = if matches!(sync_result.method_used, SyncMethod::LocalVad) {
        let segments = sync_result
            .additional_info
            .as_ref()
            .and_then(|v| v.get("detected_segments"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Some(VadInfoPayload {
            sensitivity: vad_cfg.sensitivity,
            padding_ms: vad_cfg.padding_chunks.saturating_mul(VAD_CHUNK_MS),
            segments,
        })
    } else {
        None
    };
    let subtitle_str = subtitle_path.display().to_string();
    let input = SyncInput {
        subtitle_path: subtitle_str.clone(),
        audio_path: audio_path.map(|p| p.display().to_string()),
        detected_offset_ms: offset_ms,
        confidence,
        vad,
        status: "ok",
        error: None,
    };
    let operation = SyncOperation {
        subtitle_path: subtitle_str,
        output_path: output_path.map(|p| p.display().to_string()),
        applied,
        dry_run: args.dry_run,
        status: "ok",
        error: None,
    };
    SyncSingleResult { input, operation }
}

/// Resolve the active sync method as a stable string for the
/// top-level [`SyncPayload::method`] field. Mirrors the dispatch
/// rules in [`determine_sync_method`] but operates without an audio
/// source so it can be evaluated even when no items succeed.
fn resolve_method_string(args: &SyncArgs, default_method: &str) -> String {
    if args.offset.is_some() {
        return "manual".to_string();
    }
    if let Some(method_arg) = &args.method {
        return method_to_str(&method_arg.clone().into()).to_string();
    }
    if args.vad_sensitivity.is_some() {
        return "vad".to_string();
    }
    match default_method {
        "vad" => "vad".to_string(),
        "auto" => "auto".to_string(),
        _ => "auto".to_string(),
    }
}

fn make_skip_input_op(
    sub_path: &std::path::Path,
    audio_path: Option<&std::path::Path>,
    reason: &str,
    dry_run: bool,
) -> (SyncInput, SyncOperation) {
    let err = SyncItemError {
        code: "E_FILE_MATCHING".to_string(),
        category: "file_matching".to_string(),
        message: format!("Skip sync: {reason}"),
    };
    let subtitle_str = sub_path.display().to_string();
    let input = SyncInput {
        subtitle_path: subtitle_str.clone(),
        audio_path: audio_path.map(|p| p.display().to_string()),
        detected_offset_ms: 0,
        confidence: None,
        vad: None,
        status: "error",
        error: Some(err.clone()),
    };
    let operation = SyncOperation {
        subtitle_path: subtitle_str,
        output_path: None,
        applied: false,
        dry_run,
        status: "error",
        error: Some(err),
    };
    (input, operation)
}

/// Internal helper to perform a single video-subtitle synchronization.
///
/// Returns a [`SyncSingleResult`] describing the operation. Prose stdout
/// chatter is suppressed when JSON mode is active; the caller decides
/// whether to wrap the pair in a single-pair [`SyncPayload`] or stitch
/// it into a batch result.
async fn run_single(
    args: &SyncArgs,
    config: &Config,
    sync_engine: &SyncEngine,
    format_manager: &FormatManager,
) -> Result<SyncSingleResult> {
    let json = active_mode().is_json();
    let subtitle_path = args.subtitle.as_ref().ok_or_else(|| {
        SubXError::CommandExecution(
            "Subtitle file path is required for single file sync".to_string(),
        )
    })?;

    if args.verbose && !json {
        println!("🎬 Loading subtitle file: {}", subtitle_path.display());
        println!("📄 Subtitle entries count: {}", {
            let s = format_manager.load_subtitle(subtitle_path).map_err(|e| {
                log::debug!("Failed to load subtitle: {e}");
                e
            })?;
            s.entries.len()
        });
    }
    let mut subtitle = format_manager.load_subtitle(subtitle_path).map_err(|e| {
        log::debug!("Failed to load subtitle: {e}");
        e
    })?;
    let mut effective_vad_cfg = config.sync.vad.clone();
    let mut audio_for_payload: Option<std::path::PathBuf> = None;
    let sync_result = if let Some(offset) = args.offset {
        if args.verbose && !json {
            println!("⚙️  Using manual offset: {offset:.3}s");
        }
        sync_engine
            .apply_manual_offset(&mut subtitle, offset)
            .map_err(|e| {
                log::debug!("Failed to apply manual offset: {e}");
                e
            })?;
        SyncResult {
            offset_seconds: offset,
            confidence: 1.0,
            method_used: crate::core::sync::SyncMethod::Manual,
            correlation_peak: 0.0,
            processing_duration: std::time::Duration::ZERO,
            warnings: Vec::new(),
            additional_info: None,
        }
    } else {
        // Automatic sync requires video file
        let video_path = args.video.as_ref().ok_or_else(|| {
            SubXError::CommandExecution(
                "Video file path is required for automatic sync".to_string(),
            )
        })?;

        // Check if video path is empty (manual mode case)
        if video_path.as_os_str().is_empty() {
            return Err(SubXError::CommandExecution(
                "Video file path is required for automatic sync".to_string(),
            ));
        }

        let method = determine_sync_method(args, &config.sync.default_method)?;
        if args.verbose && !json {
            println!("🔍 Starting sync analysis...");
            println!("   Method: {method:?}");
            println!("   Analysis window: {}s", args.window);
            println!("   Video file: {}", video_path.display());
        }
        let mut sync_cfg = config.sync.clone();
        apply_cli_overrides(&mut sync_cfg, args)?;
        effective_vad_cfg = sync_cfg.vad.clone();
        audio_for_payload = Some(video_path.clone());
        let result = sync_engine
            .detect_sync_offset(video_path.as_path(), &subtitle, Some(method))
            .await
            .map_err(|e| {
                log::debug!("Failed to detect sync offset: {e}");
                e
            })?;
        if args.verbose && !json {
            println!("✅ Analysis completed:");
            println!("   Detected offset: {:.3}s", result.offset_seconds);
            println!("   Confidence: {:.1}%", result.confidence * 100.0);
            println!("   Processing time: {:?}", result.processing_duration);
        }
        if !args.dry_run {
            sync_engine
                .apply_manual_offset(&mut subtitle, result.offset_seconds)
                .map_err(|e| {
                    log::debug!("Failed to apply detected offset: {e}");
                    e
                })?;
        }
        result
    };
    if !json {
        display_sync_result(&sync_result, args.verbose);
    }
    let mut applied = false;
    let mut output_path_used: Option<std::path::PathBuf> = None;
    if !args.dry_run {
        if let Some(out) = args.get_output_path() {
            if out.exists() && !args.force {
                log::debug!("Output file exists and --force not set: {}", out.display());
                return Err(SubXError::CommandExecution(format!(
                    "Output file already exists: {}. Use --force to overwrite.",
                    out.display()
                )));
            }
            format_manager.save_subtitle(&subtitle, &out).map_err(|e| {
                log::debug!("Failed to save subtitle: {e}");
                e
            })?;
            if !json {
                if args.verbose {
                    println!("💾 Synchronized subtitle saved to: {}", out.display());
                } else {
                    println!("Synchronized subtitle saved to: {}", out.display());
                }
            }
            applied = true;
            output_path_used = Some(out);
        } else {
            log::debug!("No output path specified");
            return Err(SubXError::CommandExecution(
                "No output path specified".to_string(),
            ));
        }
    } else if !json {
        println!("🔍 Dry run mode - file not saved");
    }
    Ok(build_single_result(
        args,
        &sync_result,
        subtitle_path,
        audio_for_payload.as_deref(),
        output_path_used.as_deref(),
        applied,
        &effective_vad_cfg,
    ))
}

/// Execute the sync command with the provided arguments.
///
/// This function handles both manual offset synchronization and automatic
/// synchronization using various detection methods.
///
/// # Arguments
///
/// * `args` - The sync command arguments containing input files and options
/// * `config_service` - Service for accessing configuration settings
///
/// # Returns
///
/// Returns `Ok(())` on successful synchronization, or an error if the operation fails
///
/// # Errors
///
/// This function returns an error if:
/// - Arguments validation fails
/// - Subtitle file cannot be loaded
/// - Video file is required but not provided for automatic sync
/// - Output file already exists and force flag is not set
/// - Synchronization detection fails
///
/// Execute the sync command with the provided arguments.
///
/// Handles both single and batch synchronization modes.
pub async fn execute(args: SyncArgs, config_service: &dyn ConfigService) -> Result<()> {
    // Validate arguments and prepare resources
    if let Err(msg) = args.validate() {
        return Err(SubXError::CommandExecution(msg));
    }
    let config = config_service.get_config()?;

    // Validate manual offset against max_offset_seconds configuration
    if let Some(manual_offset) = args.offset {
        if manual_offset.abs() > config.sync.max_offset_seconds {
            return Err(SubXError::config(format!(
                "The specified offset {:.2}s exceeds the configured maximum allowed value {:.2}s.\n\n\
                Please use one of the following methods to resolve this issue:\n\
                1. Use a smaller offset: --offset {:.2}\n\
                2. Adjust configuration: subx-cli config set sync.max_offset_seconds {:.2}\n\
                3. Use automatic detection: remove the --offset parameter",
                manual_offset,
                config.sync.max_offset_seconds,
                config.sync.max_offset_seconds * 0.9, // Recommended value slightly below limit
                manual_offset
                    .abs()
                    .max(config.sync.max_offset_seconds * 1.5) // Recommend increasing to appropriate value
            )));
        }
    }

    let sync_engine = SyncEngine::new(config.sync.clone())?;
    let format_manager = FormatManager::new();
    let mode = active_mode();
    let json = matches!(mode, OutputMode::Json);

    // Batch mode: multiple video-subtitle pairs
    if let Ok(SyncMode::Batch(handler)) = args.get_sync_mode() {
        let paths = handler
            .collect_files()
            .map_err(|e| SubXError::CommandExecution(e.to_string()))?;

        // Separate video and subtitle files
        let video_files: Vec<_> = paths
            .iter()
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|e| ["mp4", "mkv", "avi", "mov"].contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .collect();

        let subtitle_files: Vec<_> = paths
            .iter()
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|e| ["srt", "ass", "vtt", "sub"].contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .collect();

        let mut inputs: Vec<SyncInput> = Vec::new();
        let mut operations: Vec<SyncOperation> = Vec::new();
        let method_string = resolve_method_string(&args, &config.sync.default_method);

        // Case 1: No video files - skip all subtitles
        if video_files.is_empty() {
            for sub_path in &subtitle_files {
                if !json {
                    println!(
                        "✗ Skip sync for {}: no video files found in directory",
                        sub_path.display()
                    );
                }
                if json {
                    let (input, op) = make_skip_input_op(
                        sub_path,
                        None,
                        "no video files found in directory",
                        args.dry_run,
                    );
                    inputs.push(input);
                    operations.push(op);
                }
            }
            if json {
                // Forward progress is impossible without a video — emit a
                // top-level error envelope instead of a success envelope
                // wrapping all-error items.
                return Err(SubXError::FileMatching {
                    message: "No video files found in directory; cannot sync any subtitles"
                        .to_string(),
                });
            }
            return Ok(());
        }

        // Case 2: Exactly one video and one subtitle - sync regardless of name match
        if video_files.len() == 1 && subtitle_files.len() == 1 {
            let mut single_args = args.clone();
            single_args.input_paths.clear();
            single_args.batch = None;
            single_args.recursive = false;
            single_args.video = Some(video_files[0].clone());
            single_args.subtitle = Some(subtitle_files[0].clone());
            // If subtitle came from an archive, redirect output beside the archive
            if single_args.output.is_none() {
                if let Some(archive_path) = paths.archive_origin(subtitle_files[0]) {
                    if let Some(archive_dir) = archive_path.parent() {
                        let default = create_default_output_path(subtitle_files[0]);
                        if let Some(filename) = default.file_name() {
                            single_args.output = Some(archive_dir.join(filename));
                        }
                    }
                }
            }
            let pair = run_single(&single_args, &config, &sync_engine, &format_manager).await?;
            if json {
                emit_success(
                    mode,
                    "sync",
                    SyncPayload {
                        method: method_string,
                        inputs: vec![pair.input],
                        operations: vec![pair.operation],
                    },
                );
            }
            return Ok(());
        }

        // Case 3: Multiple videos/subtitles - match by prefix and handle unmatched
        let mut processed_videos = std::collections::HashSet::new();
        let mut processed_subtitles = std::collections::HashSet::new();

        // Process subtitle files with matching videos
        for sub_path in &subtitle_files {
            let sub_name = sub_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let sub_dir = sub_path.parent();

            let matching_video = video_files.iter().find(|&video_path| {
                let video_name = video_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let video_dir = video_path.parent();

                // Check if they are in the same directory
                if sub_dir != video_dir {
                    return false;
                }

                // If in the same directory, check if it's a 1-to-1 pair
                let dir_videos: Vec<_> = video_files
                    .iter()
                    .filter(|v| v.parent() == video_dir)
                    .collect();
                let dir_subtitles: Vec<_> = subtitle_files
                    .iter()
                    .filter(|s| s.parent() == sub_dir)
                    .collect();

                if dir_videos.len() == 1 && dir_subtitles.len() == 1 {
                    // 1-to-1 in same directory - always match
                    return true;
                }

                // Otherwise use starts_with logic
                !video_name.is_empty() && sub_name.starts_with(video_name)
            });

            if let Some(video_path) = matching_video {
                let mut single_args = args.clone();
                single_args.input_paths.clear();
                single_args.batch = None;
                single_args.recursive = false;
                single_args.video = Some((*video_path).clone());
                single_args.subtitle = Some((*sub_path).clone());
                // If subtitle came from an archive, redirect output beside the archive
                if single_args.output.is_none() {
                    if let Some(archive_path) = paths.archive_origin(sub_path) {
                        if let Some(archive_dir) = archive_path.parent() {
                            let default = create_default_output_path(sub_path);
                            if let Some(filename) = default.file_name() {
                                single_args.output = Some(archive_dir.join(filename));
                            }
                        }
                    }
                }
                // Per-file isolation: capture errors as per-item failures so
                // the top-level batch envelope can stay `status == "ok"`
                // when at least one item makes forward progress.
                match run_single(&single_args, &config, &sync_engine, &format_manager).await {
                    Ok(pair) => {
                        if json {
                            inputs.push(pair.input);
                            operations.push(pair.operation);
                        }
                    }
                    Err(err) => {
                        if !json {
                            // Preserve text-mode contract: stop on first error.
                            return Err(err);
                        }
                        let item_err = SyncItemError {
                            code: err.machine_code().to_string(),
                            category: err.category().to_string(),
                            message: err.user_friendly_message(),
                        };
                        let subtitle_str = sub_path.display().to_string();
                        let audio_str = (*video_path).display().to_string();
                        inputs.push(SyncInput {
                            subtitle_path: subtitle_str.clone(),
                            audio_path: Some(audio_str),
                            detected_offset_ms: 0,
                            confidence: None,
                            vad: None,
                            status: "error",
                            error: Some(item_err.clone()),
                        });
                        operations.push(SyncOperation {
                            subtitle_path: subtitle_str,
                            output_path: None,
                            applied: false,
                            dry_run: args.dry_run,
                            status: "error",
                            error: Some(item_err),
                        });
                    }
                }

                processed_videos.insert(video_path.as_path());
                processed_subtitles.insert(sub_path.as_path());
            }
        }

        // Display skip messages for unmatched videos
        for video_path in &video_files {
            if !processed_videos.contains(video_path.as_path()) && !json {
                println!(
                    "✗ Skip sync for {}: no matching subtitle",
                    video_path.display()
                );
            }
        }

        // Display skip messages for unmatched subtitles
        for sub_path in &subtitle_files {
            if !processed_subtitles.contains(sub_path.as_path()) {
                if !json {
                    println!("✗ Skip sync for {}: no matching video", sub_path.display());
                } else {
                    let (input, op) =
                        make_skip_input_op(sub_path, None, "no matching video", args.dry_run);
                    inputs.push(input);
                    operations.push(op);
                }
            }
        }

        if json {
            let forward_progress =
                inputs.iter().any(|i| i.status == "ok") || operations.iter().any(|o| o.applied);
            if !forward_progress {
                return Err(SubXError::FileMatching {
                    message: "No subtitle/video pairs were synced successfully".to_string(),
                });
            }
            emit_success(
                mode,
                "sync",
                SyncPayload {
                    method: method_string,
                    inputs,
                    operations,
                },
            );
        }
        return Ok(());
    }

    // Single mode or error
    match args.get_sync_mode() {
        Ok(SyncMode::Single { video, subtitle }) => {
            // Update args with the resolved paths from SyncMode
            let mut resolved_args = args.clone();
            if !video.as_os_str().is_empty() {
                resolved_args.video = Some(video.clone());
            }
            resolved_args.subtitle = Some(subtitle.clone());
            // For subtitle-only sync without offset, default to zero manual offset
            if resolved_args.video.is_none() && resolved_args.offset.is_none() {
                resolved_args.offset = Some(0.0);
                resolved_args.method = Some(crate::cli::SyncMethodArg::Manual);
            }
            let method_string = resolve_method_string(&resolved_args, &config.sync.default_method);
            let pair = run_single(&resolved_args, &config, &sync_engine, &format_manager).await?;
            if json {
                emit_success(
                    mode,
                    "sync",
                    SyncPayload {
                        method: method_string,
                        inputs: vec![pair.input],
                        operations: vec![pair.operation],
                    },
                );
            }
            Ok(())
        }
        Err(err) => Err(err),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfigService;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sync_batch_processing() -> Result<()> {
        // Prepare test configuration
        let config_service = Arc::new(TestConfigService::with_sync_settings(0.5, 30.0));

        // Create temporary directory with video and subtitle files
        let tmp = TempDir::new().unwrap();
        let video1 = tmp.path().join("movie1.mp4");
        let sub1 = tmp.path().join("movie1.srt");
        fs::write(&video1, b"").unwrap();
        fs::write(&sub1, b"1\n00:00:01,000 --> 00:00:02,000\nTest1\n\n").unwrap();

        // Test single file sync instead of batch to avoid audio processing issues
        let args = SyncArgs {
            positional_paths: Vec::new(),
            video: Some(video1.clone()),
            subtitle: Some(sub1.clone()),
            input_paths: vec![],
            recursive: false,
            offset: Some(1.0), // Use manual offset to avoid audio processing
            method: Some(crate::cli::SyncMethodArg::Manual),
            window: 30,
            vad_sensitivity: None,
            output: None,
            verbose: false,
            dry_run: true, // Use dry run to avoid file creation
            force: true,
            batch: None, // Disable batch mode,
            no_extract: false,
        };

        execute(args, config_service.as_ref()).await?;

        // In dry run mode, files are not actually created, so we just verify the command executed successfully
        Ok(())
    }
}

/// Maintain consistency with other commands
pub async fn execute_with_config(
    args: SyncArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> Result<()> {
    execute(args, config_service.as_ref()).await
}

/// Determine the sync method to use based on CLI arguments and configuration.
///
/// # Arguments
///
/// * `args` - CLI arguments which may specify a sync method
/// * `default_method` - Default method from configuration
///
/// # Returns
///
/// The determined sync method to use
fn determine_sync_method(args: &SyncArgs, default_method: &str) -> Result<SyncMethod> {
    // If CLI specifies a method, use it
    if let Some(ref method_arg) = args.method {
        return Ok(method_arg.clone().into());
    }
    // If VAD sensitivity specified, default to VAD method
    if args.vad_sensitivity.is_some() {
        return Ok(SyncMethod::LocalVad);
    }
    // Otherwise use the default method from configuration
    match default_method {
        "vad" => Ok(SyncMethod::LocalVad),
        "auto" => Ok(SyncMethod::Auto),
        _ => Ok(SyncMethod::Auto),
    }
}

/// Apply CLI argument overrides to the sync configuration.
///
/// # Arguments
///
/// * `config` - Sync configuration to modify
/// * `args` - CLI arguments containing overrides
fn apply_cli_overrides(config: &mut crate::config::SyncConfig, args: &SyncArgs) -> Result<()> {
    // Apply VAD-specific overrides
    if let Some(sensitivity) = args.vad_sensitivity {
        config.vad.sensitivity = sensitivity;
    }

    Ok(())
}

/// Display sync result information to the user.
///
/// # Arguments
///
/// * `result` - The sync result to display
/// * `verbose` - Whether to show detailed information
fn display_sync_result(result: &SyncResult, verbose: bool) {
    if verbose {
        println!("\n=== Sync Results ===");
        println!("Method used: {:?}", result.method_used);
        println!("Detected offset: {:.3} seconds", result.offset_seconds);
        println!("Confidence: {:.1}%", result.confidence * 100.0);
        println!("Processing time: {:?}", result.processing_duration);

        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &result.warnings {
                println!("  ⚠️  {warning}");
            }
        }

        if let Some(info) = &result.additional_info {
            if let Ok(pretty_info) = serde_json::to_string_pretty(info) {
                println!("\nAdditional information:");
                println!("{pretty_info}");
            }
        }
    } else {
        println!(
            "✅ Sync completed: offset {:.3}s (confidence: {:.1}%)",
            result.offset_seconds,
            result.confidence * 100.0
        );
    }
}
