//! Refactored sync command supporting new multi-method sync engine.
//!
//! This module provides the synchronization command functionality, supporting
//! multiple synchronization methods including local VAD (Voice Activity Detection),
//! automatic method selection, and manual offset adjustment.

use crate::cli::SyncArgs;
use crate::config::ConfigService;
use crate::core::formats::manager::FormatManager;
use crate::core::sync::{SyncEngine, SyncMethod, SyncResult};
use crate::{Result, error::SubXError};

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
pub async fn execute(args: SyncArgs, config_service: &dyn ConfigService) -> Result<()> {
    // Use built-in validation method
    if let Err(msg) = args.validate() {
        return Err(SubXError::CommandExecution(msg));
    }

    let config = config_service.get_config()?;

    // Create sync engine
    let sync_engine = SyncEngine::new(config.sync.clone())?;

    // Load subtitle file
    let format_manager = FormatManager::new();
    let mut subtitle = format_manager.load_subtitle(&args.subtitle)?;

    if args.verbose {
        println!("🎬 Loading subtitle file: {}", args.subtitle.display());
        println!("📄 Subtitle entries count: {}", subtitle.entries.len());
    }

    let sync_result = if let Some(manual_offset) = args.offset {
        // Manual offset mode
        if args.verbose {
            println!("⚙️  Using manual offset: {:.3}s", manual_offset);
        }

        sync_engine.apply_manual_offset(&mut subtitle, manual_offset)?;

        // Create manual offset result
        SyncResult {
            offset_seconds: manual_offset,
            confidence: 1.0,
            method_used: crate::core::sync::SyncMethod::Manual,
            correlation_peak: 0.0,
            processing_duration: std::time::Duration::ZERO,
            warnings: Vec::new(),
            additional_info: None,
        }
    } else {
        // Automatic sync mode
        let method = determine_sync_method(&args, &config.sync.default_method)?;
        let video_path = args
            .video
            .as_ref()
            .ok_or_else(|| SubXError::config("Video path required for automatic sync"))?;

        if args.verbose {
            println!("🔍 Starting sync analysis...");
            println!("   Method: {:?}", method);
            println!("   Analysis window: {}s", args.window);
            println!("   Video file: {}", video_path.display());
        }

        // Apply CLI configuration overrides
        let mut sync_config = config.sync.clone();
        apply_cli_overrides(&mut sync_config, &args)?;

        let result = sync_engine
            .detect_sync_offset(video_path.as_path(), &subtitle, Some(method))
            .await?;

        if args.verbose {
            println!("✅ Analysis completed:");
            println!("   Detected offset: {:.3}s", result.offset_seconds);
            println!("   Confidence: {:.1}%", result.confidence * 100.0);
            println!("   Processing time: {:?}", result.processing_duration);
        }

        // Apply detected offset
        if !args.dry_run {
            sync_engine.apply_manual_offset(&mut subtitle, result.offset_seconds)?;
        }

        result
    };

    // Display results
    display_sync_result(&sync_result, args.verbose);

    // Save results (unless dry run)
    if !args.dry_run {
        let output_path = args.get_output_path();

        // Check if file exists and no force flag
        if output_path.exists() && !args.force {
            return Err(SubXError::CommandExecution(format!(
                "Output file already exists: {}. Use --force to overwrite.",
                output_path.display()
            )));
        }

        format_manager.save_subtitle(&subtitle, &output_path)?;

        if args.verbose {
            println!(
                "💾 Synchronized subtitle saved to: {}",
                output_path.display()
            );
        } else {
            println!("Synchronized subtitle saved to: {}", output_path.display());
        }
    } else {
        println!("🔍 Dry run mode - file not saved");
    }

    Ok(())
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
    if let Some(chunk_size) = args.vad_chunk_size {
        config.vad.chunk_size = chunk_size;
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
                println!("  ⚠️  {}", warning);
            }
        }

        if let Some(info) = &result.additional_info {
            if let Ok(pretty_info) = serde_json::to_string_pretty(info) {
                println!("\nAdditional information:");
                println!("{}", pretty_info);
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
