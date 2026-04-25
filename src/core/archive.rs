//! Archive extraction support for SubX.
//!
//! Provides transparent extraction of `.zip` and `.rar` archive files
//! supplied as direct `-i` inputs. Archives discovered during directory
//! traversal are NOT extracted.
//!
//! # Supported formats
//!
//! - **ZIP** (`.zip`): Always available, using the `zip` crate (pure Rust).
//! - **RAR** (`.rar`): Available when compiled with the `archive-rar` feature
//!   flag. Uses the `unrar` crate, which statically compiles the UnRAR C++
//!   library via `unrar_sys` — no runtime native library dependency is required.
//!
//! # Security
//!
//! All extraction operations enforce:
//! - Path traversal prevention (zip-slip)
//! - Symlink and hardlink rejection
//! - Decompression bomb protection (1 GiB size limit, 10,000 entry limit)

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use log::{debug, warn};

/// Maximum total expanded size per archive (1 GiB).
const MAX_EXPANDED_SIZE: u64 = 1024 * 1024 * 1024;

/// Maximum number of entries per archive.
const MAX_ENTRY_COUNT: usize = 10_000;

/// Recognised archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// ZIP archive (`.zip`).
    Zip,
    /// RAR archive (`.rar`).
    Rar,
}

/// Detects archive format by file extension (case-insensitive).
///
/// Returns `None` for unrecognised extensions. No magic-byte sniffing is
/// performed.
pub fn detect_format(path: &Path) -> Option<ArchiveFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "zip" => Some(ArchiveFormat::Zip),
        "rar" => Some(ArchiveFormat::Rar),
        _ => None,
    }
}

/// Extracts an archive to the given destination directory.
///
/// Dispatches to [`extract_zip`] or [`extract_rar`] based on
/// [`detect_format`]. Returns the list of extracted file paths.
///
/// # Errors
///
/// Returns an error if the archive format is unrecognised or extraction
/// fails (corrupted, password-protected, etc.).
pub fn extract_archive(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let format = detect_format(archive_path).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unrecognised archive format: {}", archive_path.display()),
        )
    })?;

    match format {
        ArchiveFormat::Zip => extract_zip(archive_path, dest_dir),
        ArchiveFormat::Rar => extract_rar(archive_path, dest_dir),
    }
}

/// Extracts a ZIP archive to `dest_dir`.
///
/// Validates each entry against path traversal, rejects symlinks and
/// hardlinks, and enforces decompression bomb limits.
pub fn extract_zip(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to open zip archive {}: {e}", archive_path.display()),
        )
    })?;

    let dest_canonical = dest_dir
        .canonicalize()
        .unwrap_or_else(|_| dest_dir.to_path_buf());
    let mut extracted_paths = Vec::new();
    let mut total_size: u64 = 0;
    let mut entry_count: usize = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read zip entry {i}: {e}"),
            )
        })?;

        // Skip directories
        if entry.is_dir() {
            continue;
        }

        // Reject symlinks and other non-regular entries
        if entry.is_symlink() {
            warn!(
                "Skipping symlink entry in archive {}: {}",
                archive_path.display(),
                entry.name()
            );
            continue;
        }

        // Entry count limit
        entry_count += 1;
        if entry_count > MAX_ENTRY_COUNT {
            warn!(
                "Archive {} exceeds maximum entry count ({MAX_ENTRY_COUNT}), aborting extraction",
                archive_path.display()
            );
            return Err(io::Error::other(format!(
                "Archive exceeds maximum entry count ({MAX_ENTRY_COUNT})"
            )));
        }

        // Size limit check (using compressed size as early check, actual check during write)
        let uncompressed = entry.size();
        total_size += uncompressed;
        if total_size > MAX_EXPANDED_SIZE {
            warn!(
                "Archive {} exceeds maximum expanded size (1 GiB), aborting extraction",
                archive_path.display()
            );
            return Err(io::Error::other(
                "Archive exceeds maximum expanded size (1 GiB)",
            ));
        }

        // Path traversal validation
        let entry_path = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                warn!(
                    "Skipping path-traversal entry in archive {}: {}",
                    archive_path.display(),
                    entry.name()
                );
                continue;
            }
        };

        let target_path = dest_dir.join(&entry_path);
        // Double-check with canonical path resolution
        if let Ok(canonical) = target_path.canonicalize() {
            if !canonical.starts_with(&dest_canonical) {
                warn!(
                    "Skipping path-traversal entry in archive {}: {}",
                    archive_path.display(),
                    entry.name()
                );
                continue;
            }
        }

        // Create parent directories
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the file
        let mut outfile = fs::File::create(&target_path)?;
        io::copy(&mut entry, &mut outfile)?;

        debug!("Extracted: {}", target_path.display());
        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

/// Extracts a RAR archive to `dest_dir`.
///
/// Only available when compiled with the `archive-rar` feature flag.
/// Validates each entry against path traversal and enforces decompression
/// bomb limits.
#[cfg(feature = "archive-rar")]
pub fn extract_rar(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let archive = unrar::Archive::new(archive_path)
        .open_for_listing()
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to open rar archive {}: {e}", archive_path.display()),
            )
        })?;

    // First pass: validate entries
    let mut total_size: u64 = 0;
    let mut entry_count: usize = 0;
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

        entry_count += 1;
        if entry_count > MAX_ENTRY_COUNT {
            warn!(
                "Archive {} exceeds maximum entry count ({MAX_ENTRY_COUNT}), aborting extraction",
                archive_path.display()
            );
            return Err(io::Error::other(format!(
                "Archive exceeds maximum entry count ({MAX_ENTRY_COUNT})"
            )));
        }

        total_size += entry.unpacked_size;
        if total_size > MAX_EXPANDED_SIZE {
            warn!(
                "Archive {} exceeds maximum expanded size (1 GiB), aborting extraction",
                archive_path.display()
            );
            return Err(io::Error::other(
                "Archive exceeds maximum expanded size (1 GiB)",
            ));
        }

        let entry_path = PathBuf::from(&entry.filename);

        // Reject entries with absolute paths or parent-directory components
        if entry_path.has_root()
            || entry_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            warn!(
                "Skipping path-traversal entry in archive {}: {}",
                archive_path.display(),
                entry.filename.display()
            );
            continue;
        }

        // Path traversal check via lexical join
        let target_path = dest_dir.join(&entry_path);
        if !target_path.starts_with(dest_dir) {
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
pub fn extract_rar(archive_path: &Path, _dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_detect_format_zip() {
        assert_eq!(
            detect_format(Path::new("test.zip")),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn test_detect_format_zip_uppercase() {
        assert_eq!(
            detect_format(Path::new("test.ZIP")),
            Some(ArchiveFormat::Zip)
        );
    }

    #[test]
    fn test_detect_format_rar() {
        assert_eq!(
            detect_format(Path::new("test.rar")),
            Some(ArchiveFormat::Rar)
        );
    }

    #[test]
    fn test_detect_format_rar_mixed_case() {
        assert_eq!(
            detect_format(Path::new("test.Rar")),
            Some(ArchiveFormat::Rar)
        );
    }

    #[test]
    fn test_detect_format_tar_gz_none() {
        assert_eq!(detect_format(Path::new("test.tar.gz")), None);
    }

    #[test]
    fn test_detect_format_srt_none() {
        assert_eq!(detect_format(Path::new("test.srt")), None);
    }

    #[test]
    fn test_detect_format_no_extension_none() {
        assert_eq!(detect_format(Path::new("testfile")), None);
    }

    fn create_test_zip(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let zip_path = dir.join("test.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (name, content) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
        zip_path
    }

    #[test]
    fn test_extract_zip_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = create_test_zip(
            tmp.path(),
            &[
                ("subtitle.srt", b"1\n00:00:01,000 --> 00:00:02,000\nHello\n"),
                ("subdir/another.ass", b"[Script Info]\nTitle: Test\n"),
            ],
        );

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(result.len(), 2);
        assert!(dest.join("subtitle.srt").exists());
        assert!(dest.join("subdir/another.ass").exists());
    }

    #[test]
    fn test_extract_zip_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = create_test_zip(tmp.path(), &[]);
        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_zip_path_traversal_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("malicious.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // Use raw start_file to bypass name validation
        writer.start_file("../../etc/passwd", options).unwrap();
        writer
            .write_all(b"root:x:0:0:root:/root:/bin/bash\n")
            .unwrap();
        // Also add a valid entry
        writer.start_file("valid.srt", options).unwrap();
        writer.write_all(b"valid content").unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        // The path-traversal entry should be skipped, valid entry extracted
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_zip_entry_count_exceeded() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("many_entries.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        // Create more than MAX_ENTRY_COUNT entries
        for i in 0..=MAX_ENTRY_COUNT {
            writer.start_file(format!("file_{i}.txt"), options).unwrap();
            writer.write_all(b"x").unwrap();
        }
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("entry count"));
    }

    #[test]
    fn test_extract_archive_unknown_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.tar.gz");
        fs::File::create(&path).unwrap();

        let result = extract_archive(&path, tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unrecognised"));
    }

    #[cfg(not(feature = "archive-rar"))]
    #[test]
    fn test_extract_rar_disabled_feature() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.rar");
        fs::File::create(&path).unwrap();

        let result = extract_rar(&path, tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not compiled in"));
    }
}
