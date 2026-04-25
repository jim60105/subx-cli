//! Archive extraction support for SubX.
//!
//! Provides transparent extraction of archive files supplied as direct `-i`
//! inputs. Archives discovered during directory traversal are NOT extracted.
//!
//! # Module Structure
//!
//! Each supported format lives in its own sub-module, while shared security
//! validation (path-traversal checks, decompression-bomb limits) is
//! centralised in [`common`].
//!
//! - [`common`] — Shared validation: [`common::validate_entry_path`],
//!   [`common::ExtractionLimits`], size/count constants.
//! - [`zip`] — ZIP extraction (always available, pure Rust).
//! - [`rar`] — RAR extraction (feature-gated `archive-rar`).
//! - [`sevenz`] — 7-Zip extraction (always available, pure Rust).
//! - [`targz`] — Tar-gzip extraction (always available, pure Rust).
//!
//! # Supported Formats
//!
//! | Extension(s)         | Module   | Crate(s)                | Feature gate |
//! |----------------------|----------|-------------------------|--------------|
//! | `.zip`               | `zip`    | `zip`                   | always-on    |
//! | `.rar`               | `rar`    | `unrar` / `unrar_sys`   | `archive-rar`|
//! | `.7z`                | `sevenz` | `sevenz-rust`           | always-on    |
//! | `.tar.gz` / `.tgz`   | `targz`  | `tar` + `flate2`        | always-on    |
//!
//! # Security
//!
//! All extraction operations enforce:
//! - Path traversal prevention (zip-slip)
//! - Symlink and hardlink rejection
//! - Decompression bomb protection (1 GiB size limit, 10,000 entry limit)

mod common;
mod rar;
mod sevenz;
mod targz;
mod zip;

use std::io;
use std::path::{Path, PathBuf};

/// Recognised archive formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// ZIP archive (`.zip`).
    Zip,
    /// RAR archive (`.rar`).
    Rar,
    /// 7-Zip archive (`.7z`).
    SevenZip,
    /// Tar-gzip archive (`.tar.gz` or `.tgz`).
    TarGz,
}

/// Detects archive format by file extension (case-insensitive).
///
/// For `.tar.gz`, the function checks whether the filename ends with
/// `.tar.gz` (case-insensitive) before falling through to single-extension
/// matching. Returns `None` for unrecognised extensions. No magic-byte
/// sniffing is performed.
pub fn detect_format(path: &Path) -> Option<ArchiveFormat> {
    // Compound extension check: .tar.gz
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".tar.gz") {
            return Some(ArchiveFormat::TarGz);
        }
    }

    // Single extension check
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "zip" => Some(ArchiveFormat::Zip),
        "rar" => Some(ArchiveFormat::Rar),
        "7z" => Some(ArchiveFormat::SevenZip),
        "tgz" => Some(ArchiveFormat::TarGz),
        _ => None,
    }
}

/// Extracts an archive to the given destination directory.
///
/// Dispatches to the appropriate format-specific extractor based on
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
        ArchiveFormat::Zip => zip::extract_zip(archive_path, dest_dir),
        ArchiveFormat::Rar => rar::extract_rar(archive_path, dest_dir),
        ArchiveFormat::SevenZip => sevenz::extract_7z(archive_path, dest_dir),
        ArchiveFormat::TarGz => targz::extract_tar_gz(archive_path, dest_dir),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_detect_format_7z() {
        assert_eq!(
            detect_format(Path::new("test.7z")),
            Some(ArchiveFormat::SevenZip)
        );
    }

    #[test]
    fn test_detect_format_7z_uppercase() {
        assert_eq!(
            detect_format(Path::new("test.7Z")),
            Some(ArchiveFormat::SevenZip)
        );
    }

    #[test]
    fn test_detect_format_tar_gz() {
        assert_eq!(
            detect_format(Path::new("test.tar.gz")),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn test_detect_format_tar_gz_uppercase() {
        assert_eq!(
            detect_format(Path::new("test.TAR.GZ")),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn test_detect_format_tgz() {
        assert_eq!(
            detect_format(Path::new("test.tgz")),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn test_detect_format_tgz_uppercase() {
        assert_eq!(
            detect_format(Path::new("test.TGZ")),
            Some(ArchiveFormat::TarGz)
        );
    }

    #[test]
    fn test_detect_format_tar_bz2_none() {
        assert_eq!(detect_format(Path::new("test.tar.bz2")), None);
    }

    #[test]
    fn test_detect_format_plain_gz_none() {
        assert_eq!(detect_format(Path::new("test.gz")), None);
    }

    #[test]
    fn test_detect_format_srt_none() {
        assert_eq!(detect_format(Path::new("test.srt")), None);
    }

    #[test]
    fn test_detect_format_no_extension_none() {
        assert_eq!(detect_format(Path::new("testfile")), None);
    }

    #[test]
    fn test_extract_archive_unknown_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.tar.bz2");
        std::fs::File::create(&path).unwrap();

        let result = extract_archive(&path, tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unrecognised"));
    }

    #[cfg(not(feature = "archive-rar"))]
    #[test]
    fn test_extract_rar_disabled_feature() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.rar");
        std::fs::File::create(&path).unwrap();

        let result = extract_archive(&path, tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not compiled in"));
    }
}
