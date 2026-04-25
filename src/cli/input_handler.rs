use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use log::warn;
use tempfile::TempDir;

use crate::core::archive;
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
/// - **Archive Extraction**: Transparently extracts `.zip` (and `.rar` when built
///   with the `archive-rar` feature) archives passed directly as inputs into a
///   temporary directory and processes the extracted files as if they had been
///   supplied directly. Archives discovered during recursive directory traversal
///   are NOT extracted. The behaviour can be disabled per command via
///   `--no-extract` (see [`with_no_extract`](Self::with_no_extract)), in which
///   case archive files are treated as regular files and filtered by the
///   command's extension list.
///
/// # Return Value
///
/// [`collect_files`](Self::collect_files) returns a [`CollectedFiles`] handle
/// that dereferences to `&[PathBuf]`. When archive extraction is performed,
/// `CollectedFiles` owns the underlying [`tempfile::TempDir`] handles; the
/// extracted directories are removed automatically (RAII) when the
/// `CollectedFiles` value is dropped. Callers must therefore keep the
/// `CollectedFiles` value alive for as long as the extracted file paths are
/// in use. `CollectedFiles` also exposes
/// [`archive_origin`](CollectedFiles::archive_origin) so callers can map an
/// extracted file back to the original archive that produced it.
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
/// #     no_extract: false,
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
    /// Whether to skip archive extraction for archive file inputs
    pub no_extract: bool,
}

impl InputPathHandler {
    /// Merge paths from multiple sources to create a unified path list
    ///
    /// This method provides a unified interface for CLI commands to merge
    /// different types of path parameters into a single PathBuf vector.
    ///
    /// # Arguments
    ///
    /// * `optional_paths` - Optional path list (e.g., `path`, `input`, `video`, `subtitle`, etc.)
    /// * `multiple_paths` - Multiple path list (e.g., `input_paths`)
    /// * `string_paths` - String format path list (e.g., `file_paths`)
    ///
    /// # Returns
    ///
    /// Returns the merged PathBuf vector, or an error if all inputs are empty
    ///
    /// # Examples
    ///
    /// ```rust
    /// use subx_cli::cli::InputPathHandler;
    /// use std::path::PathBuf;
    ///
    /// // Merge paths from different sources
    /// let optional = vec![Some(PathBuf::from("single.srt"))];
    /// let multiple = vec![PathBuf::from("dir1"), PathBuf::from("dir2")];
    /// let strings = vec!["file1.srt".to_string(), "file2.ass".to_string()];
    ///
    /// let merged = InputPathHandler::merge_paths_from_multiple_sources(
    ///     &optional,
    ///     &multiple,
    ///     &strings
    /// )?;
    ///
    /// // merged now contains all paths
    /// assert_eq!(merged.len(), 5);
    /// # Ok::<(), subx_cli::error::SubXError>(())
    /// ```
    pub fn merge_paths_from_multiple_sources(
        optional_paths: &[Option<PathBuf>],
        multiple_paths: &[PathBuf],
        string_paths: &[String],
    ) -> Result<Vec<PathBuf>, SubXError> {
        let mut all_paths = Vec::new();

        // Add optional paths (filter out None values)
        for p in optional_paths.iter().flatten() {
            all_paths.push(p.clone());
        }

        // Add multiple paths
        all_paths.extend(multiple_paths.iter().cloned());

        // Add string paths (convert to PathBuf)
        for path_str in string_paths {
            all_paths.push(PathBuf::from(path_str));
        }

        // Check if any paths were specified
        if all_paths.is_empty() {
            return Err(SubXError::NoInputSpecified);
        }

        Ok(all_paths)
    }

    /// Create InputPathHandler from command line arguments
    pub fn from_args(input_args: &[PathBuf], recursive: bool) -> Result<Self, SubXError> {
        let handler = Self {
            paths: input_args.to_vec(),
            recursive,
            file_extensions: Vec::new(),
            no_extract: false,
        };
        handler.validate()?;
        Ok(handler)
    }

    /// Set supported file extensions (without dot)
    pub fn with_extensions(mut self, extensions: &[&str]) -> Self {
        self.file_extensions = extensions.iter().map(|s| s.to_lowercase()).collect();
        self
    }

    /// Set whether to skip archive extraction.
    ///
    /// When `true`, archive files (`.zip`, `.rar`) are treated as regular
    /// files and subject to the normal extension filter instead of being
    /// extracted.
    pub fn with_no_extract(mut self, no_extract: bool) -> Self {
        self.no_extract = no_extract;
        self
    }

    /// Validate that all paths exist
    pub fn validate(&self) -> Result<(), SubXError> {
        for path in &self.paths {
            if !path.exists() {
                return Err(SubXError::PathNotFound(path.clone()));
            }
        }
        Ok(())
    }

    /// Get all specified directory paths
    ///
    /// This method returns all specified directory paths for commands
    /// that need to process directories one by one. If the specified path
    /// contains files, it will return the directory containing that file.
    ///
    /// # Returns
    ///
    /// Deduplicated list of directory paths
    ///
    /// # Examples
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
    /// # fs::write(&file1, "test content").unwrap();
    ///
    /// let paths = vec![file1.clone(), test_dir.to_path_buf()];
    /// let handler = InputPathHandler::from_args(&paths, false)?;
    /// let directories = handler.get_directories();
    ///
    /// // Should contain test_dir (after deduplication)
    /// assert_eq!(directories.len(), 1);
    /// assert_eq!(directories[0], test_dir);
    /// # Ok::<(), subx_cli::error::SubXError>(())
    /// ```
    pub fn get_directories(&self) -> Vec<PathBuf> {
        let mut directories = std::collections::HashSet::new();

        for path in &self.paths {
            if path.is_dir() {
                directories.insert(path.clone());
            } else if path.is_file() {
                if let Some(parent) = path.parent() {
                    directories.insert(parent.to_path_buf());
                }
            }
        }

        directories.into_iter().collect()
    }

    /// Expand files and directories, collecting all files that match the filter conditions.
    ///
    /// When archive extraction is enabled (the default), directly-specified
    /// archive files (`.zip`, `.rar`) are transparently extracted to temporary
    /// directories and their contents are included in the result instead of
    /// the archive path itself. Archives found during directory traversal
    /// are **not** extracted.
    pub fn collect_files(&self) -> Result<CollectedFiles, SubXError> {
        let mut files = Vec::new();
        let mut temp_dirs = Vec::new();
        let mut archive_origins: HashMap<PathBuf, PathBuf> = HashMap::new();

        for base in &self.paths {
            if base.is_file() {
                // Check if this is an archive that should be extracted
                if !self.no_extract {
                    if let Some(_format) = archive::detect_format(base) {
                        match self.extract_and_collect(base) {
                            Ok((extracted, temp_dir)) => {
                                let temp_root = temp_dir.path().to_path_buf();
                                archive_origins.insert(temp_root, base.clone());
                                files.extend(extracted);
                                temp_dirs.push(temp_dir);
                                continue;
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to extract archive {}, skipping: {e}",
                                    base.display()
                                );
                                continue;
                            }
                        }
                    }
                }
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

        if temp_dirs.is_empty() {
            Ok(CollectedFiles::new(files))
        } else {
            Ok(CollectedFiles::with_archives(
                files,
                temp_dirs,
                archive_origins,
            ))
        }
    }

    /// Extracts an archive to a temp directory and returns paths matching
    /// the configured extension filter.
    fn extract_and_collect(
        &self,
        archive_path: &Path,
    ) -> Result<(Vec<PathBuf>, TempDir), SubXError> {
        let temp_dir = TempDir::new().map_err(|e| {
            SubXError::CommandExecution(format!("Failed to create temp directory: {e}"))
        })?;
        let extracted = archive::extract_archive(archive_path, temp_dir.path()).map_err(|e| {
            SubXError::CommandExecution(format!(
                "Failed to extract {}: {e}",
                archive_path.display()
            ))
        })?;

        let filtered: Vec<PathBuf> = extracted
            .into_iter()
            .filter(|p| self.matches_extension(p))
            .collect();

        Ok((filtered, temp_dir))
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
            let ft = entry
                .file_type()
                .map_err(|e| SubXError::DirectoryReadError {
                    path: dir.to_path_buf(),
                    source: e,
                })?;
            if ft.is_symlink() {
                log::debug!("Skipping symlink: {}", entry.path().display());
                continue;
            }
            let p = entry.path();
            if ft.is_file() && self.matches_extension(&p) {
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
            let ft = entry
                .file_type()
                .map_err(|e| SubXError::DirectoryReadError {
                    path: dir.to_path_buf(),
                    source: e,
                })?;
            if ft.is_symlink() {
                log::debug!("Skipping symlink: {}", entry.path().display());
                continue;
            }
            let p = entry.path();
            if ft.is_file() {
                if self.matches_extension(&p) {
                    result.push(p.clone());
                }
            } else if ft.is_dir() {
                result.extend(self.scan_directory_recursive(&p)?);
            }
        }
        Ok(result)
    }
}

/// Result of collecting files from input paths, including any temporary
/// directories created during archive extraction.
///
/// This struct owns any `TempDir` handles created during archive extraction.
/// The temporary directories are automatically cleaned up when this value
/// is dropped.
#[derive(Debug)]
pub struct CollectedFiles {
    /// Collected file paths
    paths: Vec<PathBuf>,
    /// Temporary directories from archive extraction (kept alive by ownership)
    _temp_dirs: Vec<TempDir>,
    /// Mapping from temp-directory root to original archive file path
    archive_origins: HashMap<PathBuf, PathBuf>,
}

impl CollectedFiles {
    /// Creates a new `CollectedFiles` with no archive origins.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            _temp_dirs: Vec::new(),
            archive_origins: HashMap::new(),
        }
    }

    /// Creates a new `CollectedFiles` with archive context.
    pub fn with_archives(
        paths: Vec<PathBuf>,
        temp_dirs: Vec<TempDir>,
        archive_origins: HashMap<PathBuf, PathBuf>,
    ) -> Self {
        Self {
            paths,
            _temp_dirs: temp_dirs,
            archive_origins,
        }
    }

    /// Returns the archive origin path for a file extracted from an archive.
    ///
    /// If the given path starts with a known temp-directory root, returns
    /// the original archive file path. Returns `None` for non-archive paths.
    pub fn archive_origin(&self, path: &Path) -> Option<&Path> {
        for (temp_root, archive_path) in &self.archive_origins {
            if path.starts_with(temp_root) {
                return Some(archive_path.as_path());
            }
        }
        None
    }

    /// Consumes self and returns the collected paths.
    ///
    /// **Warning:** This drops the `TempDir` handles, so any paths pointing
    /// to temporary extraction directories will become invalid.
    pub fn into_paths(self) -> Vec<PathBuf> {
        self.paths
    }
}

impl std::ops::Deref for CollectedFiles {
    type Target = Vec<PathBuf>;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl AsRef<[PathBuf]> for CollectedFiles {
    fn as_ref(&self) -> &[PathBuf] {
        &self.paths
    }
}

#[cfg(test)]
mod symlink_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn test_scan_directory_recursive_skips_symlinks() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real.txt");
        fs::write(&real, b"x").unwrap();
        let link = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let handler = InputPathHandler::from_args(&[tmp.path().to_path_buf()], true).unwrap();
        let results = handler.scan_directory_recursive(tmp.path()).unwrap();

        assert!(results.iter().any(|p| p == &real));
        assert!(
            !results.iter().any(|p| p == &link),
            "symlinked file should have been skipped"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_scan_directory_flat_skips_symlinks() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real.txt");
        fs::write(&real, b"x").unwrap();
        let link = tmp.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let handler = InputPathHandler::from_args(&[tmp.path().to_path_buf()], false).unwrap();
        let results = handler.scan_directory_flat(tmp.path()).unwrap();

        assert!(results.iter().any(|p| p == &real));
        assert!(!results.iter().any(|p| p == &link));
    }
}
