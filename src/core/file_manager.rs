use std::fs;
use std::path::{Path, PathBuf};

use crate::{error::SubXError, Result};

/// 安全的檔案操作管理器，用於追蹤檔案建立與移除操作，並在需要時進行回滾。
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
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    /// 記錄檔案建立操作
    pub fn record_creation<P: AsRef<Path>>(&mut self, path: P) {
        self.operations
            .push(FileOperation::Created(path.as_ref().to_path_buf()));
    }

    /// 安全地移除檔案
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

    /// 回滾所有已記錄的操作，依逆序執行
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
