//! ZIP archive extraction.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use log::{debug, warn};

use super::common::ExtractionLimits;

/// Extracts a ZIP archive to `dest_dir`.
///
/// Validates each entry against path traversal, rejects symlinks and
/// hardlinks, and enforces decompression bomb limits.
pub(super) fn extract_zip(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
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
    let mut limits = ExtractionLimits::new(archive_path);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to read zip entry {i}: {e}"),
            )
        })?;

        if entry.is_dir() {
            continue;
        }

        if entry.is_symlink() {
            warn!(
                "Skipping symlink entry in archive {}: {}",
                archive_path.display(),
                entry.name()
            );
            continue;
        }

        limits.check_entry(entry.size())?;

        // Path traversal validation (zip-specific enclosed_name)
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

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut outfile = fs::File::create(&target_path)?;
        io::copy(&mut entry, &mut outfile)?;

        debug!("Extracted: {}", target_path.display());
        extracted_paths.push(target_path);
    }

    Ok(extracted_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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

        writer.start_file("../../etc/passwd", options).unwrap();
        writer
            .write_all(b"root:x:0:0:root:/root:/bin/bash\n")
            .unwrap();
        writer.start_file("valid.srt", options).unwrap();
        writer.write_all(b"valid content").unwrap();
        writer.finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_zip(&zip_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_zip_entry_count_exceeded() {
        use super::super::common::MAX_ENTRY_COUNT;

        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("many_entries.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

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
}
