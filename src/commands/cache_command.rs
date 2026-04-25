//! Cache management command implementation.
//!
//! This module provides cache management functionality through the `cache`
//! subcommand, enabling users to inspect, apply, rollback, and clear cached
//! data from SubX operations.
//!
//! # Subcommands
//!
//! - **`cache status`** — display cache metadata (path, size, age, AI model,
//!   operation count, config hash validity, snapshot freshness, journal presence).
//!   Supports `--json` for machine-readable output.
//! - **`cache apply`** — replay cached dry-run results without calling the AI
//!   provider. Validates file snapshot and target paths, prompts for
//!   confirmation, and writes a journal for rollback.
//! - **`cache rollback`** — undo the most recent batch of file operations by
//!   reading the journal and reversing entries in LIFO order.
//! - **`cache clear`** — remove cached data. `--type cache` clears only the
//!   match cache, `--type journal` clears only the journal, `--type all`
//!   (default) clears both.
//!
//! All mutating operations acquire an exclusive file lock before proceeding.

use crate::Result;
use crate::cli::{ApplyArgs, CacheArgs, ClearArgs, ClearType, RollbackArgs, StatusArgs};
use crate::config::ConfigService;
use crate::core::lock::acquire_subx_lock;
use crate::core::matcher::cache::CacheData;
use crate::core::matcher::engine::{FileRelocationMode, MatchConfig, apply_cached_operations};
use crate::core::matcher::journal::{
    JournalData, JournalEntry, JournalEntryStatus, JournalOperationType,
};
use crate::error::SubXError;
use serde_json::json;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Resolve the configuration directory, preferring `XDG_CONFIG_HOME` when set.
///
/// This mirrors the path resolution used by the journal module so that cache
/// and journal files live under the same parent directory across commands and
/// tests (which typically override `XDG_CONFIG_HOME`).
fn get_config_dir() -> Result<PathBuf> {
    if let Some(xdg_config) = std::env::var_os("XDG_CONFIG_HOME") {
        Ok(PathBuf::from(xdg_config))
    } else {
        dirs::config_dir().ok_or_else(|| SubXError::config("Unable to determine config directory"))
    }
}

/// Resolve the canonical path to the match cache file.
fn cache_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("subx").join("match_cache.json"))
}

/// Resolve the canonical path to the match journal file.
fn journal_path() -> Result<PathBuf> {
    Ok(get_config_dir()?.join("subx").join("match_journal.json"))
}

/// Delete `path` if it exists, printing a per-file confirmation message.
///
/// Returns `Ok(true)` when a file was removed, `Ok(false)` when no file was
/// present, and propagates any I/O error encountered during deletion.
fn clear_file(path: &Path, label: &str) -> Result<bool> {
    if path.exists() {
        std::fs::remove_file(path)?;
        println!("{} cleared: {}", label, path.display());
        Ok(true)
    } else {
        println!("{} not found: {}", label, path.display());
        Ok(false)
    }
}

/// Handle the `cache clear` subcommand, honoring the `--type` selector.
async fn execute_clear(args: &ClearArgs) -> Result<()> {
    let _lock = acquire_subx_lock().await?;
    let config_dir = get_config_dir()?;
    let cache_file = config_dir.join("subx").join("match_cache.json");
    let journal_file = config_dir.join("subx").join("match_journal.json");

    let mut cleared_any = false;

    match args.r#type {
        ClearType::Cache => {
            cleared_any |= clear_file(&cache_file, "Cache")?;
        }
        ClearType::Journal => {
            cleared_any |= clear_file(&journal_file, "Journal")?;
        }
        ClearType::All => {
            cleared_any |= clear_file(&cache_file, "Cache")?;
            cleared_any |= clear_file(&journal_file, "Journal")?;
        }
    }

    if !cleared_any {
        println!("No cache files found to clear.");
    }
    Ok(())
}

/// Compute a config validity hash for a given relocation mode and backup setting.
///
/// This mirrors `MatchEngine::calculate_config_hash`. For `cache status`, pass
/// the default relocation mode (`"None"`) since the CLI flag is unavailable.
/// For `cache apply`, pass the cache's recorded `original_relocation_mode` to
/// get a correct comparison.
fn compute_config_hash(relocation_mode_debug: &str, backup_enabled: bool) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    relocation_mode_debug.hash(&mut hasher);
    backup_enabled.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Compute the config hash assuming the default relocation mode.
///
/// Used by `cache status` where the CLI relocation flag is not available.
fn current_config_hash(config_service: &dyn ConfigService) -> Result<String> {
    let config = config_service.get_config()?;
    Ok(compute_config_hash("None", config.general.backup_enabled))
}

/// Format a byte count as a short human-readable string (e.g. `2.4 KB`).
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Format an age (in seconds) as a short human-readable phrase.
fn format_age(age_secs: u64) -> String {
    const MIN: u64 = 60;
    const HOUR: u64 = 60 * MIN;
    const DAY: u64 = 24 * HOUR;
    if age_secs < MIN {
        format!("{} seconds ago", age_secs)
    } else if age_secs < HOUR {
        format!("{} minutes ago", age_secs / MIN)
    } else if age_secs < DAY {
        format!("{} hours ago", age_secs / HOUR)
    } else {
        format!("{} days ago", age_secs / DAY)
    }
}

/// Describe the snapshot state of a cache for human-readable reporting.
///
/// Returns a tuple `(label, machine_status)` where `label` is a user-facing
/// string and `machine_status` is the JSON-friendly status identifier
/// (`"valid"`, `"stale"`, or `"empty"`).
fn describe_snapshot(cache: &CacheData) -> (String, &'static str) {
    if cache.has_empty_snapshot() {
        ("Empty (legacy cache)".to_string(), "empty")
    } else {
        let stale = cache.validate_snapshot();
        if stale.is_empty() {
            ("Valid".to_string(), "valid")
        } else {
            (format!("Stale ({} files changed)", stale.len()), "stale")
        }
    }
}

/// Handle the `cache status` subcommand.
///
/// Loads cache metadata from disk and prints a summary of its location,
/// size, age, AI model, operation count, configuration fingerprint,
/// snapshot freshness, and whether a journal exists. Supports a
/// machine-readable `--json` output mode for scripting.
///
/// When no cache file is present, a friendly message is printed and the
/// function returns `Ok(())` without error.
///
/// # Arguments
///
/// * `args` - Parsed `cache status` arguments controlling output format.
/// * `config_service` - Active configuration service, used to recompute
///   the configuration hash for comparison against the cached value.
pub async fn execute_status(args: &StatusArgs, config_service: &dyn ConfigService) -> Result<()> {
    let cache_file = cache_path()?;
    let journal_file = journal_path()?;

    if !cache_file.exists() {
        if args.json {
            let payload = json!({
                "path": cache_file.to_string_lossy(),
                "exists": false,
                "journal_present": journal_file.exists(),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!("No cache found at {}", cache_file.display());
        }
        return Ok(());
    }

    let cache = CacheData::load(&cache_file).map_err(|e| {
        SubXError::config(format!(
            "Failed to load cache at {}: {}",
            cache_file.display(),
            e
        ))
    })?;

    let metadata = std::fs::metadata(&cache_file)?;
    let size_bytes = metadata.len();

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let age_secs = now_secs.saturating_sub(cache.created_at);

    let current_hash = current_config_hash(config_service)?;
    let hash_match = current_hash == cache.config_hash;

    let (snapshot_label, snapshot_status) = describe_snapshot(&cache);
    let stale_entries = if snapshot_status == "stale" {
        cache.validate_snapshot()
    } else {
        Vec::new()
    };
    let journal_present = journal_file.exists();

    if args.json {
        let stale_files: Vec<serde_json::Value> = stale_entries
            .iter()
            .map(|s| json!({ "path": s.path, "reason": s.reason }))
            .collect();
        let payload = json!({
            "path": cache_file.to_string_lossy(),
            "exists": true,
            "size_bytes": size_bytes,
            "created_at": cache.created_at,
            "age_seconds": age_secs,
            "cache_version": cache.cache_version,
            "ai_model": cache.ai_model_used,
            "operation_count": cache.match_operations.len(),
            "config_hash": cache.config_hash,
            "config_hash_match": hash_match,
            "current_config_hash": current_hash,
            "snapshot_status": snapshot_status,
            "stale_files": stale_files,
            "journal_present": journal_present,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        let config_line = if hash_match {
            "✓ (matches current)".to_string()
        } else {
            format!("✗ (differs from current: {})", current_hash)
        };
        let journal_line = if journal_present {
            "Present"
        } else {
            "Not found"
        };

        println!("Cache Status");
        println!("============");
        println!("Path:             {}", cache_file.display());
        println!("Size:             {}", format_size(size_bytes));
        println!("Age:              {}", format_age(age_secs));
        println!("Cache version:    {}", cache.cache_version);
        println!("AI model:         {}", cache.ai_model_used);
        println!("Operations:       {}", cache.match_operations.len());
        println!("Config hash:      {}", cache.config_hash);
        println!("Config match:     {}", config_line);
        println!("Snapshot:         {}", snapshot_label);
        println!("Journal:          {}", journal_line);
    }

    Ok(())
}

/// Handle the `cache apply` subcommand.
///
/// Loads the cached dry-run results and replays the file operations without
/// calling the AI provider. Validates the file snapshot and target paths
/// before proceeding, prompts for confirmation unless `--yes` is supplied,
/// and aborts on non-TTY stdin without `--yes`.
///
/// # Arguments
///
/// * `args` - Parsed `cache apply` arguments controlling validation bypass,
///   confirmation, and confidence filtering.
/// * `config_service` - Active configuration service for rebuilding the
///   `MatchConfig` needed by the engine replay path.
pub async fn execute_apply(args: &ApplyArgs, config_service: &dyn ConfigService) -> Result<()> {
    let _lock = acquire_subx_lock().await?;

    let cache_file = cache_path()?;
    if !cache_file.exists() {
        println!(
            "No cache found at {}. Run a dry-run match first.",
            cache_file.display()
        );
        return Ok(());
    }

    let mut cache = CacheData::load(&cache_file).map_err(|e| {
        SubXError::config(format!(
            "Failed to load cache at {}: {}",
            cache_file.display(),
            e
        ))
    })?;

    // Config hash mismatch detection — use the cache's recorded relocation mode
    let config = config_service.get_config()?;
    let apply_hash = compute_config_hash(
        &cache.original_relocation_mode,
        config.general.backup_enabled,
    );
    if apply_hash != cache.config_hash && !args.force {
        return Err(SubXError::config(format!(
            "Configuration has changed since the cache was created.\n\
             Cache hash:   {}\n\
             Current hash: {}\n\
             Use --force to bypass this check.",
            cache.config_hash, apply_hash
        )));
    }

    // Legacy cache with empty snapshot requires --force
    if cache.has_empty_snapshot() && !args.force {
        return Err(SubXError::config(
            "Cache was created without file snapshot data (legacy format).\n\
             Cannot verify file integrity. Use --force to apply anyway."
                .to_string(),
        ));
    }

    // Snapshot validation
    if !args.force && !cache.has_empty_snapshot() {
        let stale = cache.validate_snapshot();
        if !stale.is_empty() {
            let mut msg = format!(
                "{} source file(s) have changed since the cache was created:\n",
                stale.len()
            );
            for s in &stale {
                msg.push_str(&format!("  - {} ({})\n", s.path, s.reason));
            }
            msg.push_str("Use --force to apply anyway.");
            return Err(SubXError::config(msg));
        }
    }

    // Target path conflict detection
    if !args.force {
        let conflicts = cache.validate_target_paths();
        if !conflicts.is_empty() {
            let mut msg = format!("{} target path(s) already exist:\n", conflicts.len());
            for p in &conflicts {
                msg.push_str(&format!("  - {}\n", p.display()));
            }
            msg.push_str("Use --force to apply anyway.");
            return Err(SubXError::config(msg));
        }
    }

    // Apply confidence filter
    if let Some(min_conf) = args.confidence {
        let threshold = f32::from(min_conf) / 100.0;
        let before = cache.match_operations.len();
        cache
            .match_operations
            .retain(|op| op.confidence >= threshold);
        let after = cache.match_operations.len();
        if before != after {
            println!(
                "Filtered {} operation(s) below {}% confidence.",
                before - after,
                min_conf
            );
        }
    }

    if cache.match_operations.is_empty() {
        println!("No operations to apply.");
        return Ok(());
    }

    // Display summary
    println!("Cache Apply Summary");
    println!("===================");
    println!("Operations:       {}", cache.match_operations.len());
    println!("AI model:         {}", cache.ai_model_used);
    println!("Relocation mode:  {}", cache.original_relocation_mode);
    println!();
    for (i, op) in cache.match_operations.iter().enumerate() {
        println!(
            "  {}. {} → {} (confidence: {:.0}%)",
            i + 1,
            op.subtitle_file,
            op.new_subtitle_name,
            op.confidence * 100.0
        );
    }
    println!();

    // Non-TTY check and interactive confirmation
    if !args.yes {
        if !std::io::stdin().is_terminal() {
            return Err(SubXError::config(
                "Non-interactive terminal detected. Use --yes to skip confirmation.".to_string(),
            ));
        }
        print!("Proceed with apply? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Apply cancelled.");
            return Ok(());
        }
    }

    // Build MatchConfig from config service
    let config = config_service.get_config()?;
    let relocation_mode = parse_relocation_mode(&cache.original_relocation_mode);
    let match_config = MatchConfig {
        confidence_threshold: 0.0,
        max_sample_length: 2000,
        enable_content_analysis: true,
        backup_enabled: cache.original_backup_enabled,
        relocation_mode,
        conflict_resolution: crate::core::matcher::engine::ConflictResolution::Skip,
        ai_model: cache.ai_model_used.clone(),
        max_subtitle_bytes: config.general.max_subtitle_bytes,
    };

    apply_cached_operations(&cache, &match_config).await?;
    println!("Apply complete.");
    Ok(())
}

/// Parse a relocation mode string from cache metadata back into an enum value.
fn parse_relocation_mode(s: &str) -> FileRelocationMode {
    match s {
        "Copy" => FileRelocationMode::Copy,
        "Move" => FileRelocationMode::Move,
        _ => FileRelocationMode::None,
    }
}

/// Verify that a destination file still matches the metadata recorded in
/// the journal entry at the time of the original operation.
///
/// The check compares file size and modification time (seconds since the
/// Unix epoch). A mismatch or a missing destination aborts the rollback
/// and returns a descriptive error so the user can investigate or opt in
/// to force rollback via the `--force` flag.
fn verify_destination_integrity(entry: &JournalEntry) -> Result<()> {
    let metadata = match std::fs::metadata(&entry.destination) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(SubXError::config(format!(
                "Destination file {} no longer exists. Use --force to override.",
                entry.destination.display()
            )));
        }
        Err(e) => return Err(SubXError::Io(e)),
    };

    if metadata.len() != entry.file_size {
        return Err(SubXError::config(format!(
            "Destination file {} has been modified since the operation (size differs). \
             Use --force to override.",
            entry.destination.display()
        )));
    }

    let mtime_secs = metadata
        .modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    if let Some(actual) = mtime_secs {
        if actual != entry.file_mtime {
            return Err(SubXError::config(format!(
                "Destination file {} has been modified since the operation (mtime differs). \
                 Use --force to override.",
                entry.destination.display()
            )));
        }
    }

    Ok(())
}

/// Reverse the effect of a single completed journal entry.
///
/// The reversal depends on the original operation:
/// - `Copied`: the destination copy is deleted, leaving the source intact.
/// - `Moved` / `Renamed`: the destination is moved back to the original
///   source path via `std::fs::rename`.
///
/// If the entry recorded a backup file, that backup is deleted after the
/// primary reversal succeeds.
///
/// For `Moved`/`Renamed` operations the function checks that the original
/// source path is vacant before renaming back. If the source already exists
/// and `force` is false, an error is returned.
fn rollback_entry(entry: &JournalEntry, force: bool) -> Result<()> {
    match entry.operation_type {
        JournalOperationType::Copied => {
            std::fs::remove_file(&entry.destination)?;
            println!("Removed copy: {}", entry.destination.display());
        }
        JournalOperationType::Moved | JournalOperationType::Renamed => {
            if entry.source.exists() && !force {
                return Err(SubXError::config(format!(
                    "Original source path {} already exists. \
                     Rollback would overwrite it. Use --force to override.",
                    entry.source.display()
                )));
            }
            if let Some(parent) = entry.source.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::rename(&entry.destination, &entry.source)?;
            println!(
                "Rolled back: {} \u{2190} {}",
                entry.source.display(),
                entry.destination.display()
            );
        }
    }

    if let Some(backup) = &entry.backup_path {
        if backup.exists() {
            std::fs::remove_file(backup)?;
            println!("Removed backup: {}", backup.display());
        }
    }

    Ok(())
}

/// Handle the `cache rollback` subcommand.
///
/// Acquires the process-wide SubX lock, loads the journal, and replays
/// completed entries in last-in-first-out order — undoing each file
/// operation. When the rollback finishes successfully the journal file
/// is removed so subsequent commands start from a clean state.
///
/// A missing journal is not an error; it yields an informational message
/// and returns `Ok(())`. When `--force` is not supplied, the command
/// aborts before touching any file if any destination's size or mtime no
/// longer matches the journal record.
pub async fn execute_rollback(args: &RollbackArgs) -> Result<()> {
    let _lock = acquire_subx_lock().await?;

    let journal_file = journal_path()?;
    if !journal_file.exists() {
        println!("No operation journal found. Nothing to rollback.");
        return Ok(());
    }

    let journal = JournalData::load(&journal_file).await?;

    let reversed: Vec<&JournalEntry> = journal
        .entries
        .iter()
        .filter(|e| e.status == JournalEntryStatus::Completed)
        .rev()
        .collect();

    if reversed.is_empty() {
        println!("Journal has no completed operations to rollback.");
        return Ok(());
    }

    println!(
        "Rolling back {} operations from batch {}...",
        reversed.len(),
        journal.batch_id
    );

    for entry in &reversed {
        if !args.force {
            verify_destination_integrity(entry)?;
        }
        rollback_entry(entry, args.force)?;
    }

    std::fs::remove_file(&journal_file)?;
    println!("Rollback complete. Journal deleted.");
    Ok(())
}

/// Dispatch the cache subcommand using the production configuration service.
///
/// For testable code paths, prefer [`execute_with_config`] which accepts an
/// injected [`ConfigService`].
pub async fn execute(args: CacheArgs) -> Result<()> {
    match args.action {
        crate::cli::CacheAction::Clear(clear_args) => {
            execute_clear(&clear_args).await?;
        }
        crate::cli::CacheAction::Status(status_args) => {
            // Fall back to the production configuration service when no service
            // was injected by the caller. This keeps the legacy `execute` entry
            // point functional for users invoking it directly.
            let config_service = crate::config::ProductionConfigService::new()?;
            execute_status(&status_args, &config_service).await?;
        }
        crate::cli::CacheAction::Apply(ref apply_args) => {
            let config_service = crate::config::ProductionConfigService::new()?;
            execute_apply(apply_args, &config_service).await?;
        }
        crate::cli::CacheAction::Rollback(rollback_args) => {
            execute_rollback(&rollback_args).await?;
        }
    }
    Ok(())
}

/// Execute cache management command with injected configuration service.
///
/// This function provides the new dependency injection interface for the cache command,
/// accepting a configuration service instead of loading configuration globally.
///
/// # Arguments
///
/// * `args` - Cache command arguments
/// * `config_service` - Configuration service providing access to cache settings
///
/// # Returns
///
/// Returns `Ok(())` on successful completion, or an error if the operation fails.
pub async fn execute_with_config(
    args: CacheArgs,
    config_service: std::sync::Arc<dyn ConfigService>,
) -> Result<()> {
    match args.action {
        crate::cli::CacheAction::Status(status_args) => {
            execute_status(&status_args, config_service.as_ref()).await
        }
        crate::cli::CacheAction::Apply(apply_args) => {
            execute_apply(&apply_args, config_service.as_ref()).await
        }
        other => execute(CacheArgs { action: other }).await,
    }
}
