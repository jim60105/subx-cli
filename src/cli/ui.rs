//! User interface utilities and display helpers for SubX CLI.
//!
//! This module provides a comprehensive set of utilities for creating
//! consistent and user-friendly command-line interfaces. It handles
//! status messages, progress indicators, result displays, and AI usage
//! statistics with consistent styling and formatting.
//!
//! # Features
//!
//! - **Status Messages**: Success, error, and warning message formatting
//! - **Progress Indicators**: Configurable progress bars for long operations
//! - **Result Display**: Formatted tables and structured output
//! - **AI Statistics**: Usage tracking and cost information display
//! - **Consistent Styling**: Color-coded messages with Unicode symbols
//!
//! # Design Principles
//!
//! - **Accessibility**: Clear visual hierarchy with color and symbols
//! - **Configurability**: Respects user preferences for progress display
//! - **Consistency**: Unified styling across all CLI operations
//! - **Informativeness**: Rich context and actionable information
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::cli::ui;
//!
//! // Display status messages
//! ui::print_success("Subtitle files processed successfully");
//! ui::print_warning("File format might be incompatible");
//! ui::print_error("Unable to read configuration file");
//!
//! // Create progress bar for batch operations
//! let progress = ui::create_progress_bar(100);
//! for i in 0..100 {
//!     progress.inc(1);
//!     // ... processing ...
//! }
//! progress.finish_with_message("Processing completed");
//! ```

// src/cli/ui.rs
use crate::cli::output::{self, OutputMode};
use crate::cli::table::{MatchDisplayRow, create_match_table};
use crate::core::matcher::MatchOperation;
use colored::*;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

/// Display a success message with consistent formatting.
///
/// Prints a success message with a green checkmark symbol and styled text.
/// Used throughout the application to indicate successful completion of
/// operations such as file processing, configuration updates, or command execution.
///
/// # Format
/// ```text
/// ✓ [message]
/// ```
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ui::print_success;
///
/// print_success("Successfully processed 15 subtitle files");
/// print_success("Configuration saved to ~/.config/subx/config.toml");
/// print_success("AI matching completed with 98% confidence");
/// ```
///
/// # Output Examples
/// ```text
/// ✓ Successfully processed 15 subtitle files
/// ✓ Configuration saved to ~/.config/subx/config.toml
/// ✓ AI matching completed with 98% confidence
/// ```
pub fn print_success(message: &str) {
    // In JSON output mode, success/warning helpers are silent — the
    // command's structured payload conveys the same information through
    // the envelope. `--quiet` in text mode also suppresses these lines.
    if output::active_mode().is_json() || output::is_quiet() {
        return;
    }
    println!("{} {}", "✓".green().bold(), message);
}

/// Display an error message with consistent formatting.
///
/// Prints an error message to stderr with a red X symbol and styled text.
/// Used for reporting errors, failures, and critical issues that prevent
/// operation completion. Messages are sent to stderr to separate them
/// from normal program output.
///
/// # Format
/// ```text
/// ✗ [message]
/// ```
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ui::print_error;
///
/// print_error("Failed to load configuration file");
/// print_error("AI API request timed out after 30 seconds");
/// print_error("Invalid subtitle format detected");
/// ```
///
/// # Output Examples
/// ```text
/// ✗ Failed to load configuration file
/// ✗ AI API request timed out after 30 seconds
/// ✗ Invalid subtitle format detected
/// ```
pub fn print_error(message: &str) {
    // In JSON mode `print_error` SHALL still write to stderr but
    // without ANSI styling and without the `✗ ` symbol prefix so logs
    // stay greppable. With `--quiet` in JSON mode all stderr chatter
    // is suppressed; fatal errors are surfaced through the JSON
    // envelope on stdout instead.
    if output::active_mode().is_json() {
        if !output::is_quiet() {
            eprintln!("{}", output::strip_ansi(message));
        }
        return;
    }
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Display a warning message with consistent formatting.
///
/// Prints a warning message with a yellow warning symbol and styled text.
/// Used for non-critical issues, deprecation notices, or situations that
/// may require user attention but don't prevent operation completion.
///
/// # Format
/// ```text
/// ⚠ [message]
/// ```
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ui::print_warning;
///
/// print_warning("Legacy subtitle format detected, consider upgrading");
/// print_warning("AI confidence below 80%, manual review recommended");
/// print_warning("Configuration file not found, using defaults");
/// ```
///
/// # Output Examples
/// ```text
/// ⚠ Legacy subtitle format detected, consider upgrading
/// ⚠ AI confidence below 80%, manual review recommended
/// ⚠ Configuration file not found, using defaults
/// ```
pub fn print_warning(message: &str) {
    if output::active_mode().is_json() || output::is_quiet() {
        return;
    }
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Create a progress bar with consistent styling and configuration.
///
/// Creates a progress bar with customized styling that respects user
/// configuration preferences. The progress bar can be hidden based on
/// the `enable_progress_bar` configuration setting, allowing users to
/// disable progress indicators if desired.
///
/// # Configuration Integration
///
/// The progress bar visibility is controlled by the configuration setting:
/// ```toml
/// [general]
/// enable_progress_bar = true  # Show progress bars
/// # or
/// enable_progress_bar = false # Hide progress bars
/// ```
///
/// # Progress Bar Features
///
/// - **Animated spinner**: Indicates ongoing activity
/// - **Elapsed time**: Shows time since operation started
/// - **Progress bar**: Visual representation of completion percentage
/// - **ETA estimation**: Estimated time to completion
/// - **Current/total counts**: Numeric progress indicator
///
/// # Template Format
/// ```text
/// ⠋ [00:01:23] [████████████████████████████████████████] 75/100 (00:00:17)
/// ```
///
/// # Arguments
///
/// * `total` - The total number of items to be processed
///
/// # Returns
///
/// A configured `ProgressBar` instance ready for use
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ui::create_progress_bar;
///
/// // Create progress bar for 100 items
/// let progress = create_progress_bar(100);
///
/// for i in 0..100 {
///     // ... process item ...
///     progress.inc(1);
///     
///     if i % 10 == 0 {
///         progress.set_message(format!("Processing item {}", i));
///     }
/// }
///
/// progress.finish_with_message("✓ All items processed successfully");
/// ```
///
/// # Error Handling
///
/// If configuration loading fails, the progress bar will default to visible.
/// This ensures that progress indication is available even when configuration
/// is problematic.
pub fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_draw_target(progress_draw_target_for(output::active_mode()));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap(),
    );
    pb
}

/// Resolve the progress-bar draw target for the active output mode.
///
/// Per the `progress-reporting` spec, every `indicatif::ProgressBar`
/// constructed by SubX SHALL obtain its `ProgressDrawTarget` from this
/// helper so JSON mode force-hides progress frames regardless of
/// `general.enable_progress_bar`.
pub fn progress_draw_target_for(mode: OutputMode) -> ProgressDrawTarget {
    if mode.is_json() {
        ProgressDrawTarget::hidden()
    } else {
        ProgressDrawTarget::stderr()
    }
}

/// Display comprehensive AI API usage statistics and cost information.
///
/// Presents detailed information about AI API calls including token usage,
/// model information, and cost implications. This helps users understand
/// their AI service consumption and manage usage costs effectively.
///
/// # Information Displayed
///
/// - **Model Name**: The specific AI model used for processing
/// - **Token Breakdown**: Detailed token usage by category
///   - Prompt tokens: Input text sent to the AI
///   - Completion tokens: AI-generated response text
///   - Total tokens: Sum of prompt and completion tokens
/// - **Cost Implications**: Helps users understand billing impact
///
/// # Format Example
/// ```text
/// 🤖 AI API Call Details:
///    Model: gpt-4-turbo-preview
///    Prompt tokens: 1,247
///    Completion tokens: 892
///    Total tokens: 2,139
/// ```
///
/// # Arguments
///
/// * `usage` - AI usage statistics containing token counts and model information
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::ui::display_ai_usage;
/// use subx_cli::services::ai::AiUsageStats;
///
/// let usage = AiUsageStats {
///     model: "gpt-4-turbo-preview".to_string(),
///     prompt_tokens: 1247,
///     completion_tokens: 892,
///     total_tokens: 2139,
/// };
///
/// display_ai_usage(&usage);
/// ```
///
/// # Use Cases
///
/// - **Cost monitoring**: Track API usage for billing awareness
/// - **Performance analysis**: Understand token efficiency
/// - **Debugging**: Verify expected model usage
/// - **Optimization**: Identify opportunities to reduce token consumption
pub fn display_ai_usage(usage: &crate::services::ai::AiUsageStats) {
    if output::active_mode().is_json() {
        return;
    }
    println!("🤖 AI API Call Details:");
    println!("   Model: {}", usage.model);
    println!("   Prompt tokens: {}", usage.prompt_tokens);
    println!("   Completion tokens: {}", usage.completion_tokens);
    println!("   Total tokens: {}", usage.total_tokens);
    println!();
}

/// Display file matching results with support for dry-run preview mode.
pub fn display_match_results(results: &[MatchOperation], is_dry_run: bool) {
    // The match table is suppressed in JSON mode — the command's
    // structured payload covers the same information.
    if output::active_mode().is_json() {
        return;
    }
    if results.is_empty() {
        println!("{}", "No matching file pairs found".yellow());
        return;
    }

    println!("\n{}", "📋 File Matching Results".bold().blue());
    if is_dry_run {
        println!(
            "{}",
            "🔍 Preview mode (files will not be modified)".yellow()
        );
    }
    println!();

    // Split each match result into multiple lines: video, subtitle, new name, and optionally relocation
    let rows: Vec<MatchDisplayRow> = results
        .iter()
        .enumerate()
        .flat_map(|(i, op)| {
            let idx = i + 1;
            let video = op.video_file.path.to_string_lossy();
            let subtitle = op.subtitle_file.path.to_string_lossy();
            let new_name = &op.new_subtitle_name;

            // Add status symbol and tree structure
            let status_symbol = if is_dry_run { "🔍" } else { "✓" };

            let mut rows = vec![
                MatchDisplayRow {
                    file_type: format!("{status_symbol} Video {idx}"),
                    file_path: video.to_string(),
                },
                MatchDisplayRow {
                    file_type: format!("├ Subtitle {idx}"),
                    file_path: subtitle.to_string(),
                },
                MatchDisplayRow {
                    file_type: format!("├ New name {idx}"),
                    file_path: new_name.clone(),
                },
            ];

            // Add relocation operation row if needed
            if op.requires_relocation {
                let operation_icon = match op.relocation_mode {
                    crate::core::matcher::engine::FileRelocationMode::Copy => "📄",
                    crate::core::matcher::engine::FileRelocationMode::Move => "📁",
                    _ => "",
                };

                let operation_verb = match op.relocation_mode {
                    crate::core::matcher::engine::FileRelocationMode::Copy => "Copy to",
                    crate::core::matcher::engine::FileRelocationMode::Move => "Move to",
                    _ => "",
                };

                if let Some(target_path) = &op.relocation_target_path {
                    rows.push(MatchDisplayRow {
                        file_type: format!("└ {operation_icon} {operation_verb}"),
                        file_path: target_path.to_string_lossy().to_string(),
                    });
                } else {
                    // Update the last row to have the proper tree ending
                    if let Some(last_row) = rows.last_mut() {
                        last_row.file_type = last_row.file_type.replace("├", "└");
                    }
                }
            } else {
                // Update the last row to have the proper tree ending
                if let Some(last_row) = rows.last_mut() {
                    last_row.file_type = last_row.file_type.replace("├", "└");
                }
            }

            rows
        })
        .collect();

    println!("{}", create_match_table(rows));

    println!(
        "\n{}",
        format!("Total processed {} file mappings", results.len()).bold()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_table_display() {
        let rows = vec![
            MatchDisplayRow {
                file_type: "✓ Video 1".to_string(),
                file_path: "movie1.mp4".to_string(),
            },
            MatchDisplayRow {
                file_type: "├ Subtitle 1".to_string(),
                file_path: "subtitle1.srt".to_string(),
            },
            MatchDisplayRow {
                file_type: "└ New name 1".to_string(),
                file_path: "movie1.srt".to_string(),
            },
        ];
        let table = create_match_table(rows);
        assert!(table.contains("✓ Video 1"));
        assert!(table.contains("movie1.mp4"));
        assert!(table.contains("├ Subtitle 1"));
        assert!(table.contains("subtitle1.srt"));
        assert!(table.contains("└ New name 1"));
        assert!(table.contains("movie1.srt"));
    }
}
