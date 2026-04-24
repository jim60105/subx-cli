//! Utility functions for filesystem operations with CIFS compatibility.
//!
//! Provides helpers to perform file copy operations that avoid POSIX metadata
//! copy which may not be supported on CIFS (SMB) filesystems.

use std::fs::{File, OpenOptions};
use std::io::{self, copy};
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

/// Copies file contents from `source` to `destination` without copying metadata.
///
/// This function opens the source file and creates/truncates the destination file,
/// then copies the data stream. It avoids POSIX permission copy to maintain
/// compatibility with CIFS filesystems where metadata operations may fail.
///
/// # Errors
///
/// Returns an `io::Error` if reading from source or writing to destination fails.
pub fn copy_file_cifs_safe(source: &Path, destination: &Path) -> io::Result<u64> {
    let mut src = File::open(source)?;
    let mut dst = File::create(destination)?;
    copy(&mut src, &mut dst)
}

/// Atomically creates a new file at `path`, failing if it already exists.
///
/// Uses `O_CREAT | O_EXCL` semantics via `create_new(true)` so that file creation
/// is race-free. On Unix systems the file is created with mode `0o644`.
///
/// # Errors
///
/// Returns `io::ErrorKind::AlreadyExists` if the path already exists, or any
/// other I/O error from the underlying `open` call.
pub fn atomic_create_file(path: &Path) -> io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        opts.mode(0o644);
    }
    opts.open(path)
}

/// Validates that `target` resolves within `expected_parent` and is not itself a symlink.
///
/// The validation canonicalizes the parent directory of `target` and compares it to
/// the canonicalized `expected_parent` to ensure no symlink in the parent chain
/// escapes the intended directory. If `target` itself exists and is a symbolic link,
/// an error is returned.
///
/// # Errors
///
/// Returns `io::ErrorKind::PermissionDenied` if the canonicalized parent escapes
/// `expected_parent`, or if `target` is a symbolic link. Returns other I/O errors
/// from canonicalization failures.
pub fn validate_write_target(target: &Path, expected_parent: &Path) -> io::Result<()> {
    let target_parent = target.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "target has no parent directory",
        )
    })?;

    let canon_parent = target_parent.canonicalize()?;
    let canon_expected = expected_parent.canonicalize()?;

    if !canon_parent.starts_with(&canon_expected) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "target parent {} escapes expected parent {}",
                canon_parent.display(),
                canon_expected.display()
            ),
        ));
    }

    match std::fs::symlink_metadata(target) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "refusing to operate on symlink target: {}",
                        target.display()
                    ),
                ));
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }

    Ok(())
}

/// Checks that a file does not exceed the specified size limit.
///
/// Returns `Ok(())` if the file size is within bounds, or an error with a
/// descriptive message including the file label, actual size, and limit.
///
/// # Errors
///
/// Returns an `io::Error` of kind `InvalidInput` if the file exceeds `max_bytes`.
pub fn check_file_size(path: &Path, max_bytes: u64, label: &str) -> io::Result<()> {
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();
    if size > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} file too large: {} bytes (limit: {} bytes): {}",
                label,
                size,
                max_bytes,
                path.display()
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_copy_file_cifs_safe() -> io::Result<()> {
        let temp = TempDir::new()?;
        let src_path = temp.path().join("src.txt");
        let dst_path = temp.path().join("dst.txt");
        let content = b"hello cifs safe copy";
        fs::write(&src_path, content)?;
        let bytes = copy_file_cifs_safe(&src_path, &dst_path)?;
        assert_eq!(bytes as usize, content.len());
        let copied = fs::read(&dst_path)?;
        assert_eq!(copied, content);
        Ok(())
    }

    #[test]
    fn test_atomic_create_file_new() -> io::Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("new.txt");
        let mut f = atomic_create_file(&path)?;
        f.write_all(b"data")?;
        drop(f);
        assert_eq!(fs::read(&path)?, b"data");
        Ok(())
    }

    #[test]
    fn test_atomic_create_file_existing_fails() -> io::Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("exists.txt");
        fs::write(&path, b"x")?;
        let err = atomic_create_file(&path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_create_file_mode() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let temp = TempDir::new()?;
        let path = temp.path().join("mode.txt");
        atomic_create_file(&path)?;
        let meta = fs::metadata(&path)?;
        let mode = meta.permissions().mode() & 0o777;
        // umask may strip some bits, but 0o644 should at minimum be the ceiling
        assert!(mode & !0o644 == 0, "unexpected mode: {:o}", mode);
        Ok(())
    }

    #[test]
    fn test_validate_write_target_ok() -> io::Result<()> {
        let temp = TempDir::new()?;
        let target = temp.path().join("file.txt");
        validate_write_target(&target, temp.path())?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_validate_write_target_rejects_symlink_target() -> io::Result<()> {
        let temp = TempDir::new()?;
        let real = temp.path().join("real.txt");
        fs::write(&real, b"x")?;
        let link = temp.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link)?;
        let err = validate_write_target(&link, temp.path()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        Ok(())
    }

    #[test]
    fn test_check_file_size_under_limit() -> io::Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("small.txt");
        fs::write(&path, b"hello")?;
        check_file_size(&path, 1024, "Test")?;
        Ok(())
    }

    #[test]
    fn test_check_file_size_over_limit() -> io::Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("big.txt");
        fs::write(&path, vec![0u8; 2048])?;
        let err = check_file_size(&path, 1024, "Test").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("Test file too large"));
        Ok(())
    }

    #[test]
    fn test_check_file_size_at_limit() -> io::Result<()> {
        let temp = TempDir::new()?;
        let path = temp.path().join("exact.txt");
        fs::write(&path, vec![0u8; 1024])?;
        check_file_size(&path, 1024, "Test")?;
        Ok(())
    }
}
