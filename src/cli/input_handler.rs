use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SubXError;

/// Universal input path processing structure for CLI commands.
///
/// `InputPathHandler` provides a unified interface for processing file and directory
/// inputs across different SubX CLI commands. It supports multiple input sources,
/// recursive directory scanning, and file extension filtering.
///
/// This handler is used by commands like `match`, `convert`, `sync`, and `detect-encoding`
/// to provide consistent `-i` parameter functionality and directory processing behavior.
///
/// # Features
///
/// - **Multiple Input Sources**: Supports multiple files and directories via `-i` parameter
/// - **Recursive Processing**: Optional recursive directory scanning with `--recursive` flag
/// - **File Filtering**: Filter files by extension for command-specific processing
/// - **Path Validation**: Validates all input paths exist before processing
/// - **Cross-Platform**: Handles both absolute and relative paths correctly
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use subx_cli::cli::InputPathHandler;
/// use std::path::PathBuf;
/// # use tempfile::TempDir;
/// # use std::fs;
///
/// # let tmp = TempDir::new().unwrap();
/// # let test_dir = tmp.path();
/// # let file1 = test_dir.join("test1.srt");
/// # let file2 = test_dir.join("test2.ass");
/// # fs::write(&file1, "test content").unwrap();
/// # fs::write(&file2, "test content").unwrap();
///
/// // Create handler from multiple paths
/// let paths = vec![file1, file2];
/// let handler = InputPathHandler::from_args(&paths, false)?
///     .with_extensions(&["srt", "ass"]);
///
/// // Collect all matching files
/// let files = handler.collect_files()?;
/// assert_eq!(files.len(), 2);
/// # Ok::<(), subx_cli::error::SubXError>(())
/// ```
///
/// ## Directory Processing
///
/// ```rust
/// use subx_cli::cli::InputPathHandler;
/// use std::path::PathBuf;
/// # use tempfile::TempDir;
/// # use std::fs;
///
/// # let tmp = TempDir::new().unwrap();
/// # let test_dir = tmp.path();
/// # let nested_dir = test_dir.join("nested");
/// # fs::create_dir(&nested_dir).unwrap();
/// # let file1 = test_dir.join("test1.srt");
/// # let file2 = nested_dir.join("test2.srt");
/// # fs::write(&file1, "test content").unwrap();
/// # fs::write(&file2, "test content").unwrap();
///
/// // Flat directory scanning (non-recursive)
/// let handler_flat = InputPathHandler::from_args(&[test_dir.to_path_buf()], false)?
///     .with_extensions(&["srt"]);
/// let files_flat = handler_flat.collect_files()?;
/// assert_eq!(files_flat.len(), 1); // Only finds file1
///
/// // Recursive directory scanning
/// let handler_recursive = InputPathHandler::from_args(&[test_dir.to_path_buf()], true)?
///     .with_extensions(&["srt"]);
/// let files_recursive = handler_recursive.collect_files()?;
/// assert_eq!(files_recursive.len(), 2); // Finds both file1 and file2
/// # Ok::<(), subx_cli::error::SubXError>(())
/// ```
///
/// ## Command Integration
///
/// ```rust,no_run
/// use subx_cli::cli::{InputPathHandler, MatchArgs};
/// # use std::path::PathBuf;
///
/// // Example of how commands use InputPathHandler
/// # let args = MatchArgs {
/// #     path: Some(PathBuf::from("test")),
/// #     input_paths: vec![],
/// #     recursive: false,
/// #     dry_run: false,
/// #     confidence: 80,
/// #     backup: false,
/// #     copy: false,
/// #     move_files: false,
/// # };
/// let handler = args.get_input_handler()?;
/// let files = handler.collect_files()?;
/// // Process files...
/// # Ok::<(), subx_cli::error::SubXError>(())
/// ```
#[derive(Debug, Clone)]
pub struct InputPathHandler {
    /// List of input paths (files and directories) to process
    pub paths: Vec<PathBuf>,
    /// Whether to recursively scan subdirectories
    pub recursive: bool,
    /// File extension filters (lowercase, without dot)
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
