//! Utility functions for filesystem operations with CIFS compatibility.
//!
//! Provides helpers to perform file copy operations that avoid POSIX metadata
//! copy which may not be supported on CIFS (SMB) filesystems.

use std::fs::File;
use std::io::{self, copy};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
}
