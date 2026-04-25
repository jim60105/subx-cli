//! RAR archive extraction (feature-gated).

use std::io;
use std::path::{Path, PathBuf};

use log::warn;

/// Extracts a RAR archive to `dest_dir`.
///
/// Only available when compiled with the `archive-rar` feature flag.
/// Validates each entry against path traversal and enforces decompression
/// bomb limits.
#[cfg(feature = "archive-rar")]
pub(super) fn extract_rar(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    use std::fs;

    use log::debug;

    use super::common::{ExtractionLimits, validate_entry_path};

    let archive = unrar::Archive::new(archive_path)
        .open_for_listing()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to open rar archive {}: {e}", archive_path.display()),
            )
        })?;

    // First pass: validate entries
    let mut limits = ExtractionLimits::new(archive_path);
    let mut entries_to_extract = Vec::new();

    for entry_result in archive {
        let entry = entry_result.map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read rar entry: {e}"),
            )
        })?;

        if entry.is_directory() {
            continue;
        }

        limits.check_entry(entry.unpacked_size)?;

        let entry_path = PathBuf::from(&entry.filename);

        if validate_entry_path(dest_dir, &entry_path).is_none() {
            warn!(
                "Skipping path-traversal entry in archive {}: {}",
                archive_path.display(),
                entry.filename.display()
            );
            continue;
        }

        entries_to_extract.push(entry.filename.clone());
    }

    // Second pass: extract
    let mut archive = unrar::Archive::new(archive_path)
        .open_for_processing()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to open rar archive for extraction {}: {e}",
                    archive_path.display()
                ),
            )
        })?;

    let mut extracted_paths = Vec::new();

    while let Some(cursor) = archive.read_header().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to read rar header: {e}"),
        )
    })? {
        let entry_filename = cursor.entry().filename.clone();
        let is_dir = cursor.entry().is_directory();

        if is_dir || !entries_to_extract.contains(&entry_filename) {
            archive = cursor
                .skip()
                .map_err(|e| io::Error::other(format!("Failed to skip rar entry: {e}")))?;
            continue;
        }

        let target_path = dest_dir.join(&entry_filename);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        archive = cursor.extract_to(&target_path).map_err(|e| {
            io::Error::other(format!(
                "Failed to extract rar entry {}: {e}",
                entry_filename.display()
            ))
        })?;

        debug!("Extracted: {}", target_path.display());
        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

/// Stub for RAR extraction when the `archive-rar` feature is disabled.
#[cfg(not(feature = "archive-rar"))]
pub(super) fn extract_rar(archive_path: &Path, _dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    warn!(
        "RAR support is not compiled in. Skipping archive: {}. \
         Rebuild with `--features archive-rar` to enable RAR extraction.",
        archive_path.display()
    );
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "RAR support is not compiled in. Cannot extract: {}",
            archive_path.display()
        ),
    ))
}
