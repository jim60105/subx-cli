//! Tar-gzip archive extraction.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use log::{debug, warn};
use tar::EntryType;

use super::common::{ExtractionLimits, validate_entry_path};

/// Extracts a tar-gzip (`.tar.gz` / `.tgz`) archive to `dest_dir`.
///
/// Uses the `tar` and `flate2` crates with manual per-entry iteration.
/// Only `Regular` (and `Continuous`) entries are extracted; directories are
/// created but not tracked. Symlinks, hardlinks, and all other entry types
/// are rejected. Each entry is validated against path traversal and
/// decompression bomb limits via [`ExtractionLimits`].
pub(super) fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)?;
    let gz = GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    let mut extracted_paths = Vec::new();
    let mut limits = ExtractionLimits::new(archive_path);

    for entry_result in archive.entries().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to read tar.gz archive {}: {e}",
                archive_path.display()
            ),
        )
    })? {
        let mut entry = entry_result.map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed to read tar entry in {}: {e}",
                    archive_path.display()
                ),
            )
        })?;

        let entry_type = entry.header().entry_type();

        match entry_type {
            EntryType::Regular | EntryType::Continuous => { /* extract below */ }
            EntryType::Directory => {
                let path = entry.path()?.into_owned();
                if let Some(target) = validate_entry_path(dest_dir, &path) {
                    fs::create_dir_all(target)?;
                }
                continue;
            }
            EntryType::Symlink | EntryType::Link => {
                warn!(
                    "Skipping symlink/hardlink entry in archive {}: {}",
                    archive_path.display(),
                    entry.path().unwrap_or_default().display()
                );
                continue;
            }
            _ => {
                warn!(
                    "Skipping unsupported entry type in archive {}: {}",
                    archive_path.display(),
                    entry.path().unwrap_or_default().display()
                );
                continue;
            }
        }

        let entry_path = entry.path()?.into_owned();
        let size = entry.size();

        limits.check_entry(size)?;

        let target_path = match validate_entry_path(dest_dir, &entry_path) {
            Some(p) => p,
            None => {
                warn!(
                    "Skipping path-traversal entry in archive {}: {}",
                    archive_path.display(),
                    entry_path.display()
                );
                continue;
            }
        };

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

    fn create_test_tar_gz(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let tar_gz_path = dir.join("test.tar.gz");
        let file = fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);

        for (name, content) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_path(name).unwrap();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, &content[..]).unwrap();
        }

        builder.into_inner().unwrap().finish().unwrap();
        tar_gz_path
    }

    #[test]
    fn test_extract_tar_gz_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = create_test_tar_gz(
            tmp.path(),
            &[
                ("subtitle.srt", b"1\n00:00:01,000 --> 00:00:02,000\nHello\n"),
                ("subdir/another.ass", b"[Script Info]\nTitle: Test\n"),
            ],
        );

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&archive_path, &dest).unwrap();
        assert_eq!(result.len(), 2);
        assert!(dest.join("subtitle.srt").exists());
        assert!(dest.join("subdir/another.ass").exists());
    }

    #[test]
    fn test_extract_tar_gz_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let archive_path = create_test_tar_gz(tmp.path(), &[]);

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&archive_path, &dest).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_tar_gz_path_traversal_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_gz_path = tmp.path().join("malicious.tar.gz");
        let file = fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);

        // Malicious entry — set_path() rejects ".." so write raw bytes
        let content = b"root:x:0:0:root:/root:/bin/bash\n";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(EntryType::Regular);
        // Write the path directly into the raw header name field
        {
            let raw_name = b"../../etc/passwd";
            let dst = &mut header.as_old_mut().name;
            dst[..raw_name.len()].copy_from_slice(raw_name);
        }
        header.set_cksum();
        builder.append(&header, &content[..]).unwrap();

        // Valid entry
        let valid_content = b"valid content";
        let mut header2 = tar::Header::new_gnu();
        header2.set_path("valid.srt").unwrap();
        header2.set_size(valid_content.len() as u64);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder.append(&header2, &valid_content[..]).unwrap();

        builder.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&tar_gz_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_tar_gz_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = extract_tar_gz(&tmp.path().join("missing.tar.gz"), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_tar_gz_invalid_data() {
        let tmp = tempfile::tempdir().unwrap();
        let bad_path = tmp.path().join("bad.tar.gz");
        fs::write(&bad_path, b"not a tar.gz").unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&bad_path, &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_tar_gz_entry_count_exceeded() {
        use super::super::common::MAX_ENTRY_COUNT;

        let tmp = tempfile::tempdir().unwrap();
        let tar_gz_path = tmp.path().join("many_entries.tar.gz");
        let file = fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);

        for i in 0..=MAX_ENTRY_COUNT {
            let mut header = tar::Header::new_gnu();
            header.set_path(format!("file_{i}.txt")).unwrap();
            header.set_size(1);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, &b"x"[..]).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&tar_gz_path, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("entry count"));
    }

    #[test]
    fn test_extract_tar_gz_symlink_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_gz_path = tmp.path().join("symlink.tar.gz");
        let file = fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);

        // Symlink entry
        let mut header = tar::Header::new_gnu();
        header.set_path("link.srt").unwrap();
        header.set_size(0);
        header.set_entry_type(EntryType::Symlink);
        header.set_link_name("/etc/passwd").unwrap();
        header.set_mode(0o777);
        header.set_cksum();
        builder.append(&header, &b""[..]).unwrap();

        // Valid entry
        let valid_content = b"valid subtitle content";
        let mut header2 = tar::Header::new_gnu();
        header2.set_path("valid.srt").unwrap();
        header2.set_size(valid_content.len() as u64);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder.append(&header2, &valid_content[..]).unwrap();

        builder.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&tar_gz_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }

    #[test]
    fn test_extract_tar_gz_hardlink_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let tar_gz_path = tmp.path().join("hardlink.tar.gz");
        let file = fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        let mut builder = tar::Builder::new(gz);

        // Hardlink entry
        let mut header = tar::Header::new_gnu();
        header.set_path("link.srt").unwrap();
        header.set_size(0);
        header.set_entry_type(EntryType::Link);
        header.set_link_name("/etc/passwd").unwrap();
        header.set_mode(0o644);
        header.set_cksum();
        builder.append(&header, &b""[..]).unwrap();

        // Valid entry
        let valid_content = b"valid subtitle content";
        let mut header2 = tar::Header::new_gnu();
        header2.set_path("valid.srt").unwrap();
        header2.set_size(valid_content.len() as u64);
        header2.set_mode(0o644);
        header2.set_cksum();
        builder.append(&header2, &valid_content[..]).unwrap();

        builder.into_inner().unwrap().finish().unwrap();

        let dest = tmp.path().join("extracted");
        fs::create_dir_all(&dest).unwrap();

        let result = extract_tar_gz(&tar_gz_path, &dest).unwrap();
        assert_eq!(result.len(), 1);
        assert!(dest.join("valid.srt").exists());
    }
}
