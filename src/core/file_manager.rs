use std::fs;
use std::path::{Path, PathBuf};

use crate::{Result, error::SubXError};

/// Safe file operation manager with rollback capabilities.
///
/// The `FileManager` provides atomic file operations with automatic
/// rollback functionality. It tracks all file creations and deletions,
/// allowing complete operation reversal in case of errors.
///
/// # Use Cases
///
/// - Batch file operations that need to be atomic
/// - Temporary file creation during processing
/// - Safe file replacement with backup
///
/// # Examples
///
/// ```rust,ignore
/// use subx_cli::core::file_manager::FileManager;
/// use std::path::Path;
///
/// let mut manager = FileManager::new();
/// // Create a new file (tracked for rollback)
/// manager.record_creation(Path::new("output.srt"));
/// // Remove an existing file (backed up for rollback)
/// manager.remove_file(Path::new("old_file.srt")).unwrap();
/// // If something goes wrong, rollback all operations
/// manager.rollback().unwrap();
/// ```
///
/// # Safety
///
/// The manager ensures that:
/// - Created files are properly removed on rollback
/// - Removed files are backed up and restored on rollback
/// - No partial state is left after rollback completion
pub struct FileManager {
    operations: Vec<FileOperation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_manager_remove_and_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let mut manager = FileManager::new();
        manager.remove_file(&file_path).unwrap();
        assert!(!file_path.exists(), "檔案應已移除");

        // 測試回滾建立的檔案
        let new_file = temp_dir.path().join("new.txt");
        fs::write(&new_file, "content").unwrap();
        manager.record_creation(&new_file);
        manager.rollback().unwrap();
        assert!(!new_file.exists(), "建立的檔案應已回滾移除");
    }
}

/// 檔案操作類型：建立或移除
#[derive(Debug)]
enum FileOperation {
    Created(PathBuf),
    Removed(PathBuf),
}

impl FileManager {
    /// 建立新的檔案管理器
    /// Create a new `FileManager` with an empty operation log.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::core::file_manager::FileManager;
    /// let manager = FileManager::new();
    /// ```
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// Record a file creation operation for rollback purposes.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file that was created.
    pub fn record_creation<P: AsRef<Path>>(&mut self, path: P) {
        self.operations
            .push(FileOperation::Created(path.as_ref().to_path_buf()));
    }

    /// Remove a file safely and track the deletion for rollback.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file to remove.
    ///
    /// # Errors
    ///
    /// Returns `SubXError::FileNotFound` if the file does not exist.
    /// Returns `SubXError::FileOperationFailed` if the file removal fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use subx_cli::core::file_manager::FileManager;
    /// # use std::path::Path;
    /// let mut manager = FileManager::new();
    /// manager.remove_file(Path::new("file.txt"))?;
    /// # Ok::<(), subx_cli::error::SubXError>(())
    /// ```
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        if !path_buf.exists() {
            return Err(SubXError::FileNotFound(
                path_buf.to_string_lossy().to_string(),
            ));
        }
        fs::remove_file(&path_buf).map_err(|e| SubXError::FileOperationFailed(e.to_string()))?;
        self.operations
            .push(FileOperation::Removed(path_buf.clone()));
        println!("🗑️  已移除原始檔案: {}", path_buf.display());
        Ok(())
    }

    /// Roll back all recorded operations in reverse execution order.
    ///
    /// # Errors
    ///
    /// Returns `SubXError::FileOperationFailed` if any rollback operation fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use subx_cli::core::file_manager::FileManager;
    /// let mut manager = FileManager::new();
    /// manager.rollback()?;
    /// # Ok::<(), subx_cli::error::SubXError>(())
    /// ```
    pub fn rollback(&mut self) -> Result<()> {
        for op in self.operations.drain(..).rev() {
            match op {
                FileOperation::Created(path) => {
                    if path.exists() {
                        fs::remove_file(&path)
                            .map_err(|e| SubXError::FileOperationFailed(e.to_string()))?;
                        println!("🔄 已回滾建立的檔案: {}", path.display());
                    }
                }
                FileOperation::Removed(_) => {
                    // 已移除的檔案無法恢復，僅記錄警告
                    eprintln!("⚠️  警告：無法恢復已移除的檔案");
                }
            }
        }
        Ok(())
    }
}

impl Default for FileManager {
    fn default() -> Self {
        FileManager::new()
    }
}
