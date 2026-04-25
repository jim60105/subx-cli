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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfigService;
    use crate::core::matcher::cache::{CacheData, SnapshotItem};
    use crate::core::matcher::journal::{JournalEntry, JournalEntryStatus, JournalOperationType};
    use std::path::PathBuf;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Redirect the config directory to an isolated temp directory and return
    /// both the `TempDir` guard (must stay alive) and the subx subdirectory.
    fn isolated_config_dir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }
        let subx_dir = tmp.path().join("subx");
        std::fs::create_dir_all(&subx_dir).expect("create subx dir");
        (tmp, subx_dir)
    }

    /// Build a minimal `JournalEntry` whose destination file already exists
    /// on disk so that integrity checks pass by default.
    fn make_journal_entry(
        op_type: JournalOperationType,
        source: PathBuf,
        destination: PathBuf,
    ) -> JournalEntry {
        let meta = std::fs::metadata(&destination).expect("destination must exist");
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        JournalEntry {
            operation_type: op_type,
            source,
            destination,
            backup_path: None,
            status: JournalEntryStatus::Completed,
            file_size: meta.len(),
            file_mtime: mtime,
        }
    }

    /// Minimal valid `CacheData` with an empty snapshot (legacy-style).
    fn empty_snapshot_cache() -> CacheData {
        CacheData {
            cache_version: "1.0".into(),
            directory: "/tmp".into(),
            file_snapshot: vec![],
            match_operations: vec![],
            created_at: 0,
            ai_model_used: "test-model".into(),
            config_hash: "abc123".into(),
            original_relocation_mode: "None".into(),
            original_backup_enabled: false,
        }
    }

    // -----------------------------------------------------------------------
    // format_size
    // -----------------------------------------------------------------------

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(2048), "2.0 KB");
        // Just below 1 MB
        let just_below_mb = (1024.0 * 1024.0 - 1.0) as u64;
        let result = format_size(just_below_mb);
        assert!(result.ends_with("KB"), "expected KB, got {result}");
    }

    #[test]
    fn format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
        // Just below 1 GB
        let just_below_gb = (1024.0 * 1024.0 * 1024.0 - 1.0) as u64;
        let result = format_size(just_below_gb);
        assert!(result.ends_with("MB"), "expected MB, got {result}");
    }

    #[test]
    fn format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }

    // -----------------------------------------------------------------------
    // format_age
    // -----------------------------------------------------------------------

    #[test]
    fn format_age_seconds() {
        assert_eq!(format_age(0), "0 seconds ago");
        assert_eq!(format_age(30), "30 seconds ago");
        assert_eq!(format_age(59), "59 seconds ago");
    }

    #[test]
    fn format_age_minutes() {
        assert_eq!(format_age(60), "1 minutes ago");
        assert_eq!(format_age(90), "1 minutes ago");
        assert_eq!(format_age(3599), "59 minutes ago");
    }

    #[test]
    fn format_age_hours() {
        assert_eq!(format_age(3600), "1 hours ago");
        assert_eq!(format_age(7200), "2 hours ago");
        assert_eq!(format_age(86399), "23 hours ago");
    }

    #[test]
    fn format_age_days() {
        assert_eq!(format_age(86400), "1 days ago");
        assert_eq!(format_age(172800), "2 days ago");
        assert_eq!(format_age(604800), "7 days ago");
    }

    // -----------------------------------------------------------------------
    // compute_config_hash / current_config_hash
    // -----------------------------------------------------------------------

    #[test]
    fn compute_config_hash_is_deterministic() {
        let h1 = compute_config_hash("None", false);
        let h2 = compute_config_hash("None", false);
        assert_eq!(h1, h2);
    }

    #[test]
    fn compute_config_hash_differs_for_different_modes() {
        let h_none = compute_config_hash("None", false);
        let h_copy = compute_config_hash("Copy", false);
        let h_move = compute_config_hash("Move", false);
        assert_ne!(h_none, h_copy);
        assert_ne!(h_none, h_move);
        assert_ne!(h_copy, h_move);
    }

    #[test]
    fn compute_config_hash_differs_for_backup_flag() {
        let h_off = compute_config_hash("None", false);
        let h_on = compute_config_hash("None", true);
        assert_ne!(h_off, h_on);
    }

    #[test]
    fn compute_config_hash_is_16_hex_chars() {
        let h = compute_config_hash("None", false);
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn current_config_hash_returns_string() {
        let svc = TestConfigService::with_defaults();
        let h = current_config_hash(&svc).expect("should succeed");
        assert_eq!(h.len(), 16);
    }

    // -----------------------------------------------------------------------
    // parse_relocation_mode
    // -----------------------------------------------------------------------

    #[test]
    fn parse_relocation_mode_copy() {
        assert!(matches!(
            parse_relocation_mode("Copy"),
            FileRelocationMode::Copy
        ));
    }

    #[test]
    fn parse_relocation_mode_move() {
        assert!(matches!(
            parse_relocation_mode("Move"),
            FileRelocationMode::Move
        ));
    }

    #[test]
    fn parse_relocation_mode_none_keyword() {
        assert!(matches!(
            parse_relocation_mode("None"),
            FileRelocationMode::None
        ));
    }

    #[test]
    fn parse_relocation_mode_unknown_falls_back_to_none() {
        assert!(matches!(
            parse_relocation_mode("UnknownVariant"),
            FileRelocationMode::None
        ));
    }

    // -----------------------------------------------------------------------
    // describe_snapshot
    // -----------------------------------------------------------------------

    #[test]
    fn describe_snapshot_empty_is_reported_as_legacy() {
        let cache = empty_snapshot_cache();
        let (label, status) = describe_snapshot(&cache);
        assert_eq!(status, "empty");
        assert!(label.contains("legacy"), "label: {label}");
    }

    #[test]
    fn describe_snapshot_valid_when_files_match_on_disk() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("video.srt");
        std::fs::write(&file, "content").unwrap();
        let meta = std::fs::metadata(&file).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut cache = empty_snapshot_cache();
        cache.file_snapshot = vec![SnapshotItem {
            path: file.to_string_lossy().into_owned(),
            name: "video.srt".into(),
            size: meta.len(),
            mtime,
            file_type: "subtitle".into(),
        }];

        let (label, status) = describe_snapshot(&cache);
        assert_eq!(status, "valid", "label: {label}");
        assert_eq!(label, "Valid");
    }

    #[test]
    fn describe_snapshot_stale_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("gone.srt");

        let mut cache = empty_snapshot_cache();
        cache.file_snapshot = vec![SnapshotItem {
            path: missing.to_string_lossy().into_owned(),
            name: "gone.srt".into(),
            size: 100,
            mtime: 999,
            file_type: "subtitle".into(),
        }];

        let (label, status) = describe_snapshot(&cache);
        assert_eq!(status, "stale", "label: {label}");
        assert!(label.starts_with("Stale"), "label: {label}");
    }

    // -----------------------------------------------------------------------
    // clear_file
    // -----------------------------------------------------------------------

    #[test]
    fn clear_file_returns_true_and_removes_existing_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("to_delete.txt");
        std::fs::write(&target, "data").unwrap();
        assert!(target.exists());

        let result = clear_file(&target, "Cache").expect("should succeed");
        assert!(result, "should return true when file existed");
        assert!(!target.exists(), "file should be removed");
    }

    #[test]
    fn clear_file_returns_false_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        let missing = tmp.path().join("nonexistent.txt");
        assert!(!missing.exists());

        let result = clear_file(&missing, "Cache").expect("should succeed");
        assert!(!result, "should return false when file was absent");
    }

    // -----------------------------------------------------------------------
    // get_config_dir / cache_path / journal_path (path resolution)
    // -----------------------------------------------------------------------

    #[test]
    fn get_config_dir_uses_xdg_config_home_when_set() {
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }
        let dir = get_config_dir().expect("should succeed");
        assert_eq!(dir, tmp.path());
    }

    #[test]
    fn cache_path_ends_with_expected_components() {
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }
        let p = cache_path().expect("should succeed");
        assert!(p.ends_with("subx/match_cache.json"));
    }

    #[test]
    fn journal_path_ends_with_expected_components() {
        let tmp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        }
        let p = journal_path().expect("should succeed");
        assert!(p.ends_with("subx/match_journal.json"));
    }

    // -----------------------------------------------------------------------
    // verify_destination_integrity
    // -----------------------------------------------------------------------

    #[test]
    fn verify_destination_integrity_ok_when_metadata_matches() {
        let tmp = TempDir::new().unwrap();
        let dst = tmp.path().join("dest.srt");
        std::fs::write(&dst, "hello").unwrap();

        let entry = make_journal_entry(
            JournalOperationType::Copied,
            tmp.path().join("src.srt"),
            dst,
        );

        verify_destination_integrity(&entry).expect("should pass integrity check");
    }

    #[test]
    fn verify_destination_integrity_errors_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let dst = tmp.path().join("missing.srt");
        // Do not create the file

        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: tmp.path().join("src.srt"),
            destination: dst,
            backup_path: None,
            status: JournalEntryStatus::Completed,
            file_size: 5,
            file_mtime: 1_700_000_000,
        };

        let err = verify_destination_integrity(&entry).expect_err("should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("no longer exists"),
            "error should mention missing file: {msg}"
        );
    }

    #[test]
    fn verify_destination_integrity_errors_on_size_mismatch() {
        let tmp = TempDir::new().unwrap();
        let dst = tmp.path().join("sized.srt");
        std::fs::write(&dst, "hello").unwrap(); // 5 bytes

        let meta = std::fs::metadata(&dst).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: tmp.path().join("src.srt"),
            destination: dst,
            backup_path: None,
            status: JournalEntryStatus::Completed,
            file_size: 999, // deliberately wrong
            file_mtime: mtime,
        };

        let err = verify_destination_integrity(&entry).expect_err("should fail on size mismatch");
        let msg = format!("{err}");
        assert!(
            msg.contains("size differs"),
            "error should mention size: {msg}"
        );
    }

    #[test]
    fn verify_destination_integrity_errors_on_mtime_mismatch() {
        let tmp = TempDir::new().unwrap();
        let dst = tmp.path().join("mtimed.srt");
        std::fs::write(&dst, "hello").unwrap();
        let meta = std::fs::metadata(&dst).unwrap();

        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: tmp.path().join("src.srt"),
            destination: dst,
            backup_path: None,
            status: JournalEntryStatus::Completed,
            file_size: meta.len(),
            file_mtime: 1, // deliberately wrong mtime
        };

        let err = verify_destination_integrity(&entry).expect_err("should fail on mtime mismatch");
        let msg = format!("{err}");
        assert!(
            msg.contains("mtime differs"),
            "error should mention mtime: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // rollback_entry
    // -----------------------------------------------------------------------

    #[test]
    fn rollback_entry_copied_removes_destination() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.srt");
        let dst = tmp.path().join("dst.srt");
        std::fs::write(&src, "original").unwrap();
        std::fs::write(&dst, "copy").unwrap();

        let entry = make_journal_entry(JournalOperationType::Copied, src.clone(), dst.clone());
        rollback_entry(&entry, false).expect("rollback copy");

        assert!(!dst.exists(), "copy destination must be removed");
        assert!(src.exists(), "source must remain");
    }

    #[test]
    fn rollback_entry_moved_restores_source() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("original.srt");
        let dst = tmp.path().join("moved.srt");
        // After a move, only the destination exists on disk.
        std::fs::write(&dst, "payload").unwrap();

        let entry = make_journal_entry(JournalOperationType::Moved, src.clone(), dst.clone());
        rollback_entry(&entry, false).expect("rollback move");

        assert!(src.exists(), "source must be restored");
        assert!(!dst.exists(), "destination must be removed");
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "payload");
    }

    #[test]
    fn rollback_entry_renamed_restores_source() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("old_name.srt");
        let dst = tmp.path().join("new_name.srt");
        std::fs::write(&dst, "content").unwrap();

        let entry = make_journal_entry(JournalOperationType::Renamed, src.clone(), dst.clone());
        rollback_entry(&entry, false).expect("rollback rename");

        assert!(src.exists(), "original name must be restored");
        assert!(!dst.exists(), "new name must be gone");
    }

    #[test]
    fn rollback_entry_moved_errors_when_source_exists_without_force() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("exists.srt");
        let dst = tmp.path().join("dest.srt");
        // Both source and destination exist (conflicting state).
        std::fs::write(&src, "already here").unwrap();
        std::fs::write(&dst, "moved here").unwrap();

        let entry = make_journal_entry(JournalOperationType::Moved, src.clone(), dst.clone());
        let err = rollback_entry(&entry, false).expect_err("should abort when source exists");
        let msg = format!("{err}");
        assert!(
            msg.contains("already exists"),
            "error should mention conflict: {msg}"
        );
    }

    #[test]
    fn rollback_entry_moved_with_force_overwrites_existing_source() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src_force.srt");
        let dst = tmp.path().join("dst_force.srt");
        std::fs::write(&src, "old").unwrap();
        std::fs::write(&dst, "new content").unwrap();

        let entry = make_journal_entry(JournalOperationType::Moved, src.clone(), dst.clone());
        rollback_entry(&entry, true).expect("force rollback should succeed");

        assert!(src.exists(), "source must exist after force rollback");
        assert!(!dst.exists(), "destination must be gone");
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "new content");
    }

    #[test]
    fn rollback_entry_removes_existing_backup() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src_bak.srt");
        let dst = tmp.path().join("dst_bak.srt");
        let backup = tmp.path().join("src_bak.srt.bak");
        std::fs::write(&dst, "copy").unwrap();
        std::fs::write(&backup, "backup content").unwrap();

        let meta = std::fs::metadata(&dst).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: src,
            destination: dst.clone(),
            backup_path: Some(backup.clone()),
            status: JournalEntryStatus::Completed,
            file_size: meta.len(),
            file_mtime: mtime,
        };

        rollback_entry(&entry, false).expect("rollback with backup");
        assert!(!dst.exists(), "copy destination must be removed");
        assert!(!backup.exists(), "backup must be deleted");
    }

    #[test]
    fn rollback_entry_tolerates_missing_backup_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src_nobak.srt");
        let dst = tmp.path().join("dst_nobak.srt");
        let backup = tmp.path().join("missing_backup.srt.bak");
        std::fs::write(&dst, "copy").unwrap();
        // backup file intentionally not created

        let meta = std::fs::metadata(&dst).unwrap();
        let mtime = meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: src,
            destination: dst.clone(),
            backup_path: Some(backup),
            status: JournalEntryStatus::Completed,
            file_size: meta.len(),
            file_mtime: mtime,
        };

        rollback_entry(&entry, false).expect("missing backup should not cause error");
        assert!(!dst.exists());
    }

    // -----------------------------------------------------------------------
    // execute_status (unit-level path through private helpers)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn execute_status_no_cache_json_output_contains_exists_false() {
        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");
        assert!(!cache_file.exists());

        let svc = TestConfigService::with_defaults();
        let args = crate::cli::StatusArgs { json: true };
        execute_status(&args, &svc)
            .await
            .expect("status must succeed without cache");
    }

    #[tokio::test]
    async fn execute_status_no_cache_plain_output_is_ok() {
        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");
        assert!(!cache_file.exists());

        let svc = TestConfigService::with_defaults();
        let args = crate::cli::StatusArgs { json: false };
        execute_status(&args, &svc)
            .await
            .expect("status must succeed without cache (plain)");
    }

    #[tokio::test]
    async fn execute_status_valid_cache_plain_succeeds() {
        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");

        let svc = TestConfigService::with_defaults();
        let config = svc.get_config().unwrap();
        let hash = compute_config_hash("None", config.general.backup_enabled);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = serde_json::json!({
            "cache_version": "1.0",
            "directory": "/some/dir",
            "file_snapshot": [],
            "match_operations": [
                {
                    "video_file": "/some/video.mkv",
                    "subtitle_file": "/some/sub.srt",
                    "new_subtitle_name": "video.srt",
                    "confidence": 0.95,
                    "reasoning": []
                }
            ],
            "created_at": now,
            "ai_model_used": "gpt-4",
            "config_hash": hash,
            "original_relocation_mode": "None",
            "original_backup_enabled": false,
        });
        std::fs::write(&cache_file, serde_json::to_string(&cache).unwrap()).unwrap();

        let args = crate::cli::StatusArgs { json: false };
        execute_status(&args, &svc)
            .await
            .expect("status with matching hash must succeed");
    }

    #[tokio::test]
    async fn execute_status_valid_cache_json_mode_succeeds() {
        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");
        let journal_file = subx_dir.join("match_journal.json");

        let svc = TestConfigService::with_defaults();
        let config = svc.get_config().unwrap();
        let hash = compute_config_hash("None", config.general.backup_enabled);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = serde_json::json!({
            "cache_version": "1.0",
            "directory": "/some/dir",
            "file_snapshot": [],
            "match_operations": [],
            "created_at": now,
            "ai_model_used": "gpt-4",
            "config_hash": hash,
            "original_relocation_mode": "None",
            "original_backup_enabled": false,
        });
        std::fs::write(&cache_file, serde_json::to_string(&cache).unwrap()).unwrap();
        std::fs::write(&journal_file, "{}").unwrap();

        let args = crate::cli::StatusArgs { json: true };
        execute_status(&args, &svc)
            .await
            .expect("JSON status must succeed with matching hash");
    }

    #[tokio::test]
    async fn execute_status_mismatched_hash_shows_in_plain_output() {
        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let cache = serde_json::json!({
            "cache_version": "1.0",
            "directory": "/some/dir",
            "file_snapshot": [],
            "match_operations": [],
            "created_at": now,
            "ai_model_used": "gpt-4",
            "config_hash": "00000000deadbeef",
            "original_relocation_mode": "None",
            "original_backup_enabled": false,
        });
        std::fs::write(&cache_file, serde_json::to_string(&cache).unwrap()).unwrap();

        let svc = TestConfigService::with_defaults();
        let args = crate::cli::StatusArgs { json: false };
        // Should succeed even when hash doesn't match — status is informational only.
        execute_status(&args, &svc)
            .await
            .expect("status succeeds even with mismatched config hash");
    }

    // -----------------------------------------------------------------------
    // execute_rollback edge cases
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn execute_rollback_journal_with_only_pending_entries_is_noop() {
        use crate::core::matcher::journal::JournalData;

        let (_tmp, subx_dir) = isolated_config_dir();
        let journal_file = subx_dir.join("match_journal.json");

        let tmp2 = TempDir::new().unwrap();
        let dst = tmp2.path().join("file.srt");
        std::fs::write(&dst, "data").unwrap();

        let pending_entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: tmp2.path().join("src.srt"),
            destination: dst.clone(),
            backup_path: None,
            status: JournalEntryStatus::Pending,
            file_size: 4,
            file_mtime: 0,
        };

        let journal = JournalData {
            batch_id: "pending-only".into(),
            created_at: 0,
            entries: vec![pending_entry],
        };
        journal.save(&journal_file).await.expect("save journal");

        let args = RollbackArgs { force: false };
        execute_rollback(&args)
            .await
            .expect("should succeed with only pending entries");

        // When there are no completed entries to roll back, the journal is
        // left in place (nothing was reversed) and the destination is untouched.
        assert!(
            journal_file.exists(),
            "journal kept when nothing was rolled back"
        );
        assert!(dst.exists(), "pending entry destination must be untouched");
    }

    #[tokio::test]
    async fn execute_rollback_force_skips_integrity_check() {
        use crate::core::matcher::journal::JournalData;

        let (_tmp, subx_dir) = isolated_config_dir();
        let journal_file = subx_dir.join("match_journal.json");

        let tmp2 = TempDir::new().unwrap();
        let src = tmp2.path().join("orig.srt");
        let dst = tmp2.path().join("copy.srt");
        std::fs::write(&dst, "data").unwrap();

        // Record wrong size so integrity check would normally fail.
        let entry = JournalEntry {
            operation_type: JournalOperationType::Copied,
            source: src.clone(),
            destination: dst.clone(),
            backup_path: None,
            status: JournalEntryStatus::Completed,
            file_size: 9999,  // wrong size
            file_mtime: 9999, // wrong mtime
        };

        let journal = JournalData {
            batch_id: "force-batch".into(),
            created_at: 0,
            entries: vec![entry],
        };
        journal.save(&journal_file).await.expect("save journal");

        let args = RollbackArgs { force: true };
        execute_rollback(&args)
            .await
            .expect("force rollback should succeed despite integrity mismatch");

        assert!(!dst.exists(), "copy destination must be removed");
        assert!(!journal_file.exists(), "journal must be deleted");
    }

    // -----------------------------------------------------------------------
    // execute_clear via execute_with_config
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn execute_with_config_clear_journal_type_works() {
        use std::sync::Arc;
        let (_tmp, subx_dir) = isolated_config_dir();
        let journal_file = subx_dir.join("match_journal.json");
        let cache_file = subx_dir.join("match_cache.json");
        std::fs::write(&journal_file, "{}").unwrap();
        std::fs::write(&cache_file, "{}").unwrap();

        let svc = Arc::new(TestConfigService::with_defaults());
        let args = CacheArgs {
            action: crate::cli::CacheAction::Clear(crate::cli::ClearArgs {
                r#type: crate::cli::ClearType::Journal,
            }),
        };
        execute_with_config(args, svc)
            .await
            .expect("clear journal via execute_with_config");

        assert!(!journal_file.exists(), "journal should be removed");
        assert!(cache_file.exists(), "cache should remain");
    }

    // -----------------------------------------------------------------------
    // execute_apply — confidence filter
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn execute_apply_confidence_filter_removes_low_confidence_ops() {
        use crate::cli::ApplyArgs;

        let (_tmp, subx_dir) = isolated_config_dir();
        let cache_file = subx_dir.join("match_cache.json");

        let svc = TestConfigService::with_defaults();
        let config = svc.get_config().unwrap();
        let hash = compute_config_hash("None", config.general.backup_enabled);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Two operations: one at 90 %, one at 50 %.  Filter at 80 % should
        // leave only the first, reducing to 1 op — which then hits
        // "No operations to apply" because the filtered result has 1 entry
        // but the test uses force=true so it proceeds; with an empty cache
        // we'd get "No operations to apply".
        // Use --force to skip snapshot/hash checks.
        let cache = serde_json::json!({
            "cache_version": "1.0",
            "directory": "/dir",
            "file_snapshot": [],
            "match_operations": [
                {
                    "video_file": "/dir/v1.mkv",
                    "subtitle_file": "/dir/s1.srt",
                    "new_subtitle_name": "v1.srt",
                    "confidence": 0.5,
                    "reasoning": []
                }
            ],
            "created_at": now,
            "ai_model_used": "gpt-4",
            "config_hash": hash,
            "original_relocation_mode": "None",
            "original_backup_enabled": false,
        });
        std::fs::write(&cache_file, serde_json::to_string(&cache).unwrap()).unwrap();

        // confidence threshold = 80, filters out the 50% op → empty → "No operations"
        let result = execute_apply(
            &ApplyArgs {
                yes: true,
                force: true,
                confidence: Some(80),
            },
            &svc,
        )
        .await;
        assert!(
            result.is_ok(),
            "confidence filter to empty ops should be Ok: {result:?}"
        );
    }
}
