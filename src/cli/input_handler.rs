use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SubXError;

/// 通用輸入路徑處理結構
#[derive(Debug, Clone)]
pub struct InputPathHandler {
    /// 輸入路徑列表
    pub paths: Vec<PathBuf>,
    /// 是否遞迴處理子目錄
    pub recursive: bool,
    /// 檔案類型過濾器 (小寫副檔名，不含點)
    pub file_extensions: Vec<String>,
}

impl InputPathHandler {
    /// 從命令列參數建立 InputPathHandler
    pub fn from_args(input_args: &[PathBuf], recursive: bool) -> Result<Self, SubXError> {
        let handler = Self {
            paths: input_args.to_vec(),
            recursive,
            file_extensions: Vec::new(),
        };
        handler.validate()?;
        Ok(handler)
    }

    /// 設定支援的檔案副檔名 (不含點)
    pub fn with_extensions(mut self, extensions: &[&str]) -> Self {
        self.file_extensions = extensions.iter().map(|s| s.to_lowercase()).collect();
        self
    }

    /// 驗證所有路徑是否存在
    pub fn validate(&self) -> Result<(), SubXError> {
        for path in &self.paths {
            if !path.exists() {
                return Err(SubXError::PathNotFound(path.clone()));
            }
        }
        Ok(())
    }

    /// 展開檔案與目錄，並收集所有符合過濾條件的檔案列表
    pub fn collect_files(&self) -> Result<Vec<PathBuf>, SubXError> {
        let mut files = Vec::new();
        for base in &self.paths {
            if base.is_file() {
                if self.matches_extension(base) {
                    files.push(base.clone());
                }
            } else if base.is_dir() {
                if self.recursive {
                    files.extend(self.scan_directory_recursive(base)?);
                } else {
                    files.extend(self.scan_directory_flat(base)?);
                }
            } else {
                return Err(SubXError::InvalidPath(base.clone()));
            }
        }
        Ok(files)
    }

    fn matches_extension(&self, path: &Path) -> bool {
        if self.file_extensions.is_empty() {
            return true;
        }
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| {
                self.file_extensions
                    .iter()
                    .any(|ext| ext.eq_ignore_ascii_case(s))
            })
            .unwrap_or(false)
    }

    fn scan_directory_flat(&self, dir: &Path) -> Result<Vec<PathBuf>, SubXError> {
        let mut result = Vec::new();
        let rd = fs::read_dir(dir).map_err(|e| SubXError::DirectoryReadError {
            path: dir.to_path_buf(),
            source: e,
        })?;
        for entry in rd {
            let entry = entry.map_err(|e| SubXError::DirectoryReadError {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let p = entry.path();
            if p.is_file() && self.matches_extension(&p) {
                result.push(p);
            }
        }
        Ok(result)
    }

    fn scan_directory_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>, SubXError> {
        let mut result = Vec::new();
        let rd = fs::read_dir(dir).map_err(|e| SubXError::DirectoryReadError {
            path: dir.to_path_buf(),
            source: e,
        })?;
        for entry in rd {
            let entry = entry.map_err(|e| SubXError::DirectoryReadError {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let p = entry.path();
            if p.is_file() {
                if self.matches_extension(&p) {
                    result.push(p.clone());
                }
            } else if p.is_dir() {
                result.extend(self.scan_directory_recursive(&p)?);
            }
        }
        Ok(result)
    }
}
