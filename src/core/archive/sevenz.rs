//! 7-Zip archive extraction.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use log::{debug, warn};

use super::common::{ExtractionLimits, validate_entry_path};

/// Extracts a 7-Zip archive to `dest_dir`.
///
/// Uses the `sevenz-rust` crate for pure-Rust 7z decompression with
/// entry-by-entry callback extraction. Validates each entry against
/// path traversal, rejects directories and anti-items inline, and
/// enforces decompression bomb limits via [`ExtractionLimits`].
pub(super) fn extract_7z(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut extracted_paths: Vec<PathBuf> = Vec::new();
    let mut limits = ExtractionLimits::new(archive_path);

    sevenz_rust::decompress_file_with_extract_fn(archive_path, dest_dir, |entry, reader, _dest| {
        if entry.is_directory || entry.is_anti_item {
            return Ok(true);
        }

        // Reject reparse points (Windows symlinks)
        if entry.has_windows_attributes && entry.windows_attributes & 0x0400 != 0 {
            warn!(
                "Skipping reparse-point entry in archive {}: {}",
                archive_path.display(),
                entry.name
            );
            return Ok(true);
        }

        limits
            .check_entry(entry.size)
            .map_err(|e| sevenz_rust::Error::Other(std::borrow::Cow::Owned(e.to_string())))?;

        let entry_path = Path::new(&entry.name);
        let target_path = match validate_entry_path(dest_dir, entry_path) {
            Some(p) => p,
            None => {
                warn!(
                    "Skipping path-traversal entry in archive {}: {}",
                    archive_path.display(),
                    entry.name
                );
                return Ok(true);
            }
        };

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sevenz_rust::Error::Io(e, format!("creating parent dir for {}", entry.name).into())
            })?;
        }

        let mut outfile = fs::File::create(&target_path).map_err(|e| {
            sevenz_rust::Error::Io(e, format!("creating {}", target_path.display()).into())
        })?;
        io::copy(reader, &mut outfile).map_err(|e| {
            sevenz_rust::Error::Io(e, format!("writing {}", target_path.display()).into())
        })?;

        debug!("Extracted: {}", target_path.display());
        extracted_paths.push(target_path);
        Ok(true)
    })
    .map_err(|e| match e {
        sevenz_rust::Error::PasswordRequired => io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "7z archive is password-protected: {}",
                archive_path.display()
            ),
        ),
        sevenz_rust::Error::Io(io_err, ctx) => {
            io::Error::new(io_err.kind(), format!("{ctx}: {io_err}"))
        }
        other => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to extract 7z archive {}: {other}",
                archive_path.display()
            ),
        ),
    })?;

    Ok(extracted_paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_7z_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = extract_7z(&tmp.path().join("missing.7z"), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_7z_invalid_data() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_path = tmp.path().join("bad.7z");
        fs::write(&bad_path, b"not a real 7z archive").unwrap();

        let result = extract_7z(&bad_path, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_7z_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let seven_z_path = tmp.path().join("test.7z");

        let mut writer = sevenz_rust::SevenZWriter::create(&seven_z_path).unwrap();
        let entry = sevenz_rust::SevenZArchiveEntry::from_path(
            Path::new("hello.srt"),
            "hello.srt".to_string(),
        );
        writer
            .push_archive_entry(entry, Some(std::io::Cursor::new(b"content")))
            .unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_7z(&seven_z_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("hello.srt").exists());
    }

    #[test]
    fn test_extract_7z_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let seven_z_path = tmp.path().join("empty.7z");

        let writer = sevenz_rust::SevenZWriter::create(&seven_z_path).unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_7z(&seven_z_path, &dest).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_7z_path_traversal_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let seven_z_path = tmp.path().join("malicious.7z");

        let mut writer = sevenz_rust::SevenZWriter::create(&seven_z_path).unwrap();

        // Malicious entry with path traversal
        let mut bad_entry = sevenz_rust::SevenZArchiveEntry::default();
        bad_entry.name = "../../etc/passwd".to_string();
        writer
            .push_archive_entry(bad_entry, Some(std::io::Cursor::new(b"malicious")))
            .unwrap();

        // Valid entry
        let good_entry = sevenz_rust::SevenZArchiveEntry::from_path(
            Path::new("valid.srt"),
            "valid.srt".to_string(),
        );
        writer
            .push_archive_entry(good_entry, Some(std::io::Cursor::new(b"valid")))
            .unwrap();

        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_7z(&seven_z_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_7z_entry_count_exceeded() {
        use super::super::common::MAX_ENTRY_COUNT;

        let tmp = tempfile::tempdir().unwrap();
        let seven_z_path = tmp.path().join("many.7z");

        let mut writer = sevenz_rust::SevenZWriter::create(&seven_z_path).unwrap();
        for i in 0..=MAX_ENTRY_COUNT {
            let entry = sevenz_rust::SevenZArchiveEntry::from_path(
                Path::new(&format!("file_{i}.txt")),
                format!("file_{i}.txt"),
            );
            writer
                .push_archive_entry(entry, Some(std::io::Cursor::new(b"x")))
                .unwrap();
        }
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_7z(&seven_z_path, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("entry count"));
    }
}
