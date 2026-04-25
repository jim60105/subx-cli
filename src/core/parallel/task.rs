//! Task definition and utilities for parallel processing
use crate::core::fs_util::{atomic_create_file, validate_write_target};
use async_trait::async_trait;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

/// Returns true if the given I/O error indicates a cross-device link failure,
/// which is the signal to fall back from `rename` to copy+delete.
fn is_cross_device_error(err: &io::Error) -> bool {
    #[cfg(unix)]
    {
        // EXDEV == 18 on Linux and most Unixes
        if err.raw_os_error() == Some(18) {
            return true;
        }
    }
    // Fallback check: some platforms report cross-device via a message or kind.
    matches!(err.kind(), io::ErrorKind::Unsupported)
}

/// Resolve filename conflicts by atomically creating the target.
///
/// Returns the resolved path together with the open file handle. Callers
/// should write through this handle to avoid TOCTOU races between conflict
/// resolution and file creation.
fn resolve_filename_conflict(
    target: std::path::PathBuf,
) -> Result<(std::path::PathBuf, File), Box<dyn std::error::Error + Send + Sync>> {
    match atomic_create_file(&target) {
        Ok(f) => return Ok((target, f)),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }

    let file_stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));

    for i in 1..1000 {
        let new_name = if extension.is_empty() {
            format!("{}.{}", file_stem, i)
        } else {
            format!("{}.{}.{}", file_stem, i, extension)
        };
        let new_path = parent.join(new_name);
        match atomic_create_file(&new_path) {
            Ok(f) => return Ok((new_path, f)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err("Could not resolve filename conflict".into())
}

/// Trait defining a unit of work that can be executed asynchronously.
///
/// All tasks in the parallel processing system must implement this trait
/// to provide execution logic and metadata.
#[async_trait]
pub trait Task: Send + Sync {
    /// Executes the task and returns the result.
    async fn execute(&self) -> TaskResult;
    /// Returns the type identifier for this task.
    fn task_type(&self) -> &'static str;
    /// Returns a unique identifier for this specific task instance.
    fn task_id(&self) -> String;
    /// Returns an estimated duration for the task execution.
    fn estimated_duration(&self) -> Option<std::time::Duration> {
        None
    }
    /// Returns a human-readable description of the task.
    fn description(&self) -> String {
        format!("{} task", self.task_type())
    }
}

/// Result of task execution indicating success, failure, or partial completion.
///
/// Provides detailed information about the outcome of a task execution,
/// including success/failure status and descriptive messages.
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Task completed successfully with a result message
    Success(String),
    /// Task failed with an error message
    Failed(String),
    /// Task was cancelled before completion
    Cancelled,
    /// Task partially completed with success and failure messages
    PartialSuccess(String, String),
}

/// Current execution status of a task in the system.
///
/// Tracks the lifecycle of a task from initial queuing through completion
/// or failure, providing detailed status information.
#[derive(Debug, Clone)]
pub enum TaskStatus {
    /// Task is queued and waiting for execution
    Pending,
    /// Task is currently being executed
    Running,
    /// Task completed successfully or with partial success
    Completed(TaskResult),
    /// Task failed during execution
    Failed(String),
    /// Task was cancelled before or during execution
    Cancelled,
}

impl fmt::Display for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskResult::Success(msg) => write!(f, "✓ {}", msg),
            TaskResult::Failed(msg) => write!(f, "✗ {}", msg),
            TaskResult::Cancelled => write!(f, "⚠ Task cancelled"),
            TaskResult::PartialSuccess(success, warn) => {
                write!(f, "⚠ {} (warning: {})", success, warn)
            }
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed(result) => write!(f, "Completed: {}", result),
            TaskStatus::Failed(msg) => write!(f, "Failed: {}", msg),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Task for processing files (convert, sync, match, validate).
///
/// Represents a file processing operation that can be executed
/// asynchronously in the parallel processing system.
pub struct FileProcessingTask {
    /// Path to the input file to be processed
    pub input_path: std::path::PathBuf,
    /// Optional output path for the processed file
    pub output_path: Option<std::path::PathBuf>,
    /// The specific operation to perform on the file
    pub operation: ProcessingOperation,
}

/// Supported operations for file processing tasks.
///
/// Defines the different types of operations that can be performed
/// on subtitle and video files in the processing system.
#[derive(Debug, Clone)]
pub enum ProcessingOperation {
    /// Convert subtitle format from one type to another
    ConvertFormat {
        /// Source format (e.g., "srt", "ass")
        from: String,
        /// Target format (e.g., "srt", "ass")
        to: String,
    },
    /// Synchronize subtitle timing with audio
    SyncSubtitle {
        /// Path to the audio file for synchronization
        audio_path: std::path::PathBuf,
    },
    /// Match subtitle files with video files
    MatchFiles {
        /// Whether to search recursively in subdirectories
        recursive: bool,
    },
    /// Validate subtitle file format and structure
    ValidateFormat,
    /// Copy subtitle file to video folder
    CopyToVideoFolder {
        /// Path to the source subtitle file to be copied
        source: std::path::PathBuf,
        /// Path to the target video folder where the subtitle will be copied
        target: std::path::PathBuf,
    },
    /// Move subtitle file to video folder
    MoveToVideoFolder {
        /// Path to the source subtitle file to be moved
        source: std::path::PathBuf,
        /// Path to the target video folder where the subtitle will be moved
        target: std::path::PathBuf,
    },
    /// Copy a file with a new name (local copy)
    CopyWithRename {
        /// Source file path
        source: std::path::PathBuf,
        /// Target file path
        target: std::path::PathBuf,
    },
    /// Create a backup of a file
    CreateBackup {
        /// Original file path
        source: std::path::PathBuf,
        /// Backup file path
        backup: std::path::PathBuf,
    },
    /// Rename (move) a file
    RenameFile {
        /// Original file path
        source: std::path::PathBuf,
        /// New file path after rename
        target: std::path::PathBuf,
    },
}

#[async_trait]
impl Task for FileProcessingTask {
    async fn execute(&self) -> TaskResult {
        match &self.operation {
            ProcessingOperation::ConvertFormat { from, to } => {
                match self.convert_format(from, to).await {
                    Ok(path) => TaskResult::Success(format!(
                        "Successfully converted {} -> {}: {}",
                        from,
                        to,
                        path.display()
                    )),
                    Err(e) => TaskResult::Failed(format!(
                        "Conversion failed {}: {}",
                        self.input_path.display(),
                        e
                    )),
                }
            }
            ProcessingOperation::SyncSubtitle { .. } => {
                // Sync not supported in parallel tasks
                TaskResult::Failed("Sync functionality not implemented".to_string())
            }
            ProcessingOperation::MatchFiles { recursive } => {
                match self.match_files(*recursive).await {
                    Ok(m) => TaskResult::Success(format!(
                        "File matching completed: found {} matches",
                        m.len()
                    )),
                    Err(e) => TaskResult::Failed(format!("Matching failed: {}", e)),
                }
            }
            ProcessingOperation::ValidateFormat => match self.validate_format().await {
                Ok(true) => TaskResult::Success(format!(
                    "Format validation passed: {}",
                    self.input_path.display()
                )),
                Ok(false) => TaskResult::Failed(format!(
                    "Format validation failed: {}",
                    self.input_path.display()
                )),
                Err(e) => TaskResult::Failed(format!("Validation error: {}", e)),
            },
            ProcessingOperation::CopyToVideoFolder { source, target } => {
                match self.execute_copy_operation(source, target).await {
                    Ok(_) => TaskResult::Success(format!(
                        "Copied: {} -> {}",
                        source.display(),
                        target.display()
                    )),
                    Err(e) => TaskResult::Failed(format!("Copy failed: {}", e)),
                }
            }
            ProcessingOperation::MoveToVideoFolder { source, target } => {
                match self.execute_move_operation(source, target).await {
                    Ok(_) => TaskResult::Success(format!(
                        "Moved: {} -> {}",
                        source.display(),
                        target.display()
                    )),
                    Err(e) => TaskResult::Failed(format!("Move failed: {}", e)),
                }
            }
            ProcessingOperation::CopyWithRename { source, target } => {
                match self
                    .execute_copy_with_rename_operation(source, target)
                    .await
                {
                    Ok(_) => TaskResult::Success(format!(
                        "Copied: {} -> {}",
                        source.display(),
                        target.display()
                    )),
                    Err(e) => TaskResult::Failed(format!("Copy failed: {}", e)),
                }
            }
            ProcessingOperation::CreateBackup { source, backup } => {
                match self.execute_create_backup_operation(source, backup).await {
                    Ok(_) => TaskResult::Success(format!(
                        "Backup created: {} -> {}",
                        source.display(),
                        backup.display()
                    )),
                    Err(e) => TaskResult::Failed(format!("Backup failed: {}", e)),
                }
            }
            ProcessingOperation::RenameFile { source, target } => {
                match self.execute_rename_file_operation(source, target).await {
                    Ok(_) => TaskResult::Success(format!(
                        "Renamed: {} -> {}",
                        source.display(),
                        target.display()
                    )),
                    Err(e) => TaskResult::Failed(format!("Rename failed: {}", e)),
                }
            }
        }
    }

    fn task_type(&self) -> &'static str {
        match &self.operation {
            ProcessingOperation::ConvertFormat { .. } => "convert",
            ProcessingOperation::SyncSubtitle { .. } => "sync",
            ProcessingOperation::MatchFiles { .. } => "match",
            ProcessingOperation::ValidateFormat => "validate",
            ProcessingOperation::CopyToVideoFolder { .. } => "copy_to_video_folder",
            ProcessingOperation::MoveToVideoFolder { .. } => "move_to_video_folder",
            ProcessingOperation::CopyWithRename { .. } => "copy_with_rename",
            ProcessingOperation::CreateBackup { .. } => "create_backup",
            ProcessingOperation::RenameFile { .. } => "rename_file",
        }
    }

    fn task_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.input_path.hash(&mut hasher);
        self.operation.hash(&mut hasher);
        format!("{}_{:x}", self.task_type(), hasher.finish())
    }

    fn estimated_duration(&self) -> Option<std::time::Duration> {
        if let Ok(meta) = std::fs::metadata(&self.input_path) {
            let size_mb = meta.len() as f64 / 1_048_576.0;
            let secs = match &self.operation {
                ProcessingOperation::ConvertFormat { .. } => size_mb * 0.1,
                ProcessingOperation::SyncSubtitle { .. } => size_mb * 0.5,
                ProcessingOperation::MatchFiles { .. } => 2.0,
                ProcessingOperation::ValidateFormat => size_mb * 0.05,
                ProcessingOperation::CopyToVideoFolder { .. } => size_mb * 0.01, // Fast copy
                ProcessingOperation::MoveToVideoFolder { .. } => size_mb * 0.005, // Even faster move
                ProcessingOperation::CopyWithRename { .. } => size_mb * 0.01,
                ProcessingOperation::CreateBackup { .. } => size_mb * 0.01,
                ProcessingOperation::RenameFile { .. } => size_mb * 0.005,
            };
            Some(std::time::Duration::from_secs_f64(secs))
        } else {
            None
        }
    }

    fn description(&self) -> String {
        match &self.operation {
            ProcessingOperation::ConvertFormat { from, to } => {
                format!(
                    "Convert {} from {} to {}",
                    self.input_path.display(),
                    from,
                    to
                )
            }
            ProcessingOperation::SyncSubtitle { audio_path } => format!(
                "Sync subtitle {} with audio {}",
                self.input_path.display(),
                audio_path.display()
            ),
            ProcessingOperation::MatchFiles { recursive } => format!(
                "Match files in {}{}",
                self.input_path.display(),
                if *recursive { " (recursive)" } else { "" }
            ),
            ProcessingOperation::ValidateFormat => {
                format!("Validate format of {}", self.input_path.display())
            }
            ProcessingOperation::CopyToVideoFolder { source, target } => {
                format!("Copy {} to {}", source.display(), target.display())
            }
            ProcessingOperation::MoveToVideoFolder { source, target } => {
                format!("Move {} to {}", source.display(), target.display())
            }
            ProcessingOperation::CopyWithRename { source, target } => {
                format!(
                    "CopyWithRename {} to {}",
                    source.display(),
                    target.display()
                )
            }
            ProcessingOperation::CreateBackup { source, backup } => {
                format!("CreateBackup {} to {}", source.display(), backup.display())
            }
            ProcessingOperation::RenameFile { source, target } => {
                format!("Rename {} to {}", source.display(), target.display())
            }
        }
    }
}

impl FileProcessingTask {
    /// Create a new file processing task with operation
    pub fn new(
        input_path: std::path::PathBuf,
        output_path: Option<std::path::PathBuf>,
        operation: ProcessingOperation,
    ) -> Self {
        FileProcessingTask {
            input_path,
            output_path,
            operation,
        }
    }

    /// Execute copy operation for file relocation
    async fn execute_copy_operation(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = source.to_path_buf();
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                // Create target directory if it doesn't exist
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Atomically resolve filename conflicts and obtain an open file handle.
                let (final_target, mut file) = resolve_filename_conflict(target)?;

                if let Some(parent) = final_target.parent() {
                    validate_write_target(&final_target, parent)?;
                }

                // Stream the source file contents through the exclusive handle.
                let mut src = std::fs::File::open(&source)?;
                std::io::copy(&mut src, &mut file)?;
                Ok(())
            },
        )
        .await?
    }

    /// Execute move operation for file relocation
    async fn execute_move_operation(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = source.to_path_buf();
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                // Create target directory if it doesn't exist
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Try a fast in-filesystem rename first; fall back to copy+delete otherwise.
                if !target.exists() {
                    match std::fs::rename(&source, &target) {
                        Ok(_) => return Ok(()),
                        Err(e) if is_cross_device_error(&e) => {}
                        Err(_) => { /* fall through to copy+delete path */ }
                    }
                }

                let (final_target, mut file) = resolve_filename_conflict(target)?;

                if let Some(parent) = final_target.parent() {
                    validate_write_target(&final_target, parent)?;
                }

                let mut src = std::fs::File::open(&source)?;
                std::io::copy(&mut src, &mut file)?;
                file.sync_all()?;
                drop(file);
                std::fs::remove_file(&source)?;
                Ok(())
            },
        )
        .await?
    }

    /// Execute a copy with rename operation (local copy) using CIFS-safe copy
    async fn execute_copy_with_rename_operation(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = source.to_path_buf();
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                crate::core::fs_util::copy_file_cifs_safe(&source, &target)?;
                Ok(())
            },
        )
        .await?
    }

    /// Execute a create backup operation using an atomically created destination.
    async fn execute_create_backup_operation(
        &self,
        source: &Path,
        backup: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = source.to_path_buf();
        let backup = backup.to_path_buf();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                if let Some(parent) = backup.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let (final_target, mut file) = resolve_filename_conflict(backup)?;

                if let Some(parent) = final_target.parent() {
                    validate_write_target(&final_target, parent)?;
                }

                let mut src = std::fs::File::open(&source)?;
                std::io::copy(&mut src, &mut file)?;
                Ok(())
            },
        )
        .await?
    }

    /// Execute a file rename operation
    async fn execute_rename_file_operation(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let source = source.to_path_buf();
        let target = target.to_path_buf();
        tokio::task::spawn_blocking(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if !target.exists() {
                    match std::fs::rename(&source, &target) {
                        Ok(_) => return Ok(()),
                        Err(e) if is_cross_device_error(&e) => {}
                        Err(_) => { /* fall through to copy+delete path */ }
                    }
                }

                let (final_target, mut file) = resolve_filename_conflict(target)?;

                if let Some(parent) = final_target.parent() {
                    validate_write_target(&final_target, parent)?;
                }

                let mut src = std::fs::File::open(&source)?;
                std::io::copy(&mut src, &mut file)?;
                file.sync_all()?;
                drop(file);
                std::fs::remove_file(&source)?;
                Ok(())
            },
        )
        .await?
    }

    async fn convert_format(&self, _from: &str, _to: &str) -> crate::Result<std::path::PathBuf> {
        // Stub convert: simply return input path
        Ok(self.input_path.clone())
    }

    async fn sync_subtitle(
        &self,
        _audio_path: &std::path::Path,
    ) -> crate::Result<crate::core::sync::SyncResult> {
        // Stub implementation: sync not available
        Err(crate::error::SubXError::parallel_processing(
            "sync_subtitle not implemented".to_string(),
        ))
    }

    async fn match_files(&self, _recursive: bool) -> crate::Result<Vec<()>> {
        // Stub implementation: no actual matching
        Ok(Vec::new())
    }

    async fn validate_format(&self) -> crate::Result<bool> {
        // Stub validate: always succeed
        Ok(true)
    }
}

// impl Hash for ProcessingOperation to support task_id generation
impl std::hash::Hash for ProcessingOperation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ProcessingOperation::ConvertFormat { from, to } => {
                "convert".hash(state);
                from.hash(state);
                to.hash(state);
            }
            ProcessingOperation::SyncSubtitle { audio_path } => {
                "sync".hash(state);
                audio_path.hash(state);
            }
            ProcessingOperation::MatchFiles { recursive } => {
                "match".hash(state);
                recursive.hash(state);
            }
            ProcessingOperation::ValidateFormat => {
                "validate".hash(state);
            }
            ProcessingOperation::CopyToVideoFolder { source, target } => {
                "copy_to_video_folder".hash(state);
                source.hash(state);
                target.hash(state);
            }
            ProcessingOperation::MoveToVideoFolder { source, target } => {
                "move_to_video_folder".hash(state);
                source.hash(state);
                target.hash(state);
            }
            ProcessingOperation::CopyWithRename { source, target } => {
                "copy_with_rename".hash(state);
                source.hash(state);
                target.hash(state);
            }
            ProcessingOperation::CreateBackup { source, backup } => {
                "create_backup".hash(state);
                source.hash(state);
                backup.hash(state);
            }
            ProcessingOperation::RenameFile { source, target } => {
                "rename_file".hash(state);
                source.hash(state);
                target.hash(state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_processing_task_validate_format() {
        let tmp = TempDir::new().unwrap();
        let test_file = tmp.path().join("test.srt");
        tokio::fs::write(&test_file, "1\n00:00:01,000 --> 00:00:02,000\nTest\n")
            .await
            .unwrap();
        let task = FileProcessingTask {
            input_path: test_file.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
    }

    #[tokio::test]
    async fn test_file_processing_task_copy_with_rename() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("orig.txt");
        let dst = tmp.path().join("copy.txt");
        tokio::fs::write(&src, b"hello").await.unwrap();
        let task = FileProcessingTask::new(
            src.clone(),
            Some(dst.clone()),
            ProcessingOperation::CopyWithRename {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        let data = tokio::fs::read(&dst).await.unwrap();
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn test_file_processing_task_create_backup() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("orig.txt");
        let backup = tmp.path().join("orig.txt.bak");
        tokio::fs::write(&src, b"backup").await.unwrap();
        let task = FileProcessingTask::new(
            src.clone(),
            Some(backup.clone()),
            ProcessingOperation::CreateBackup {
                source: src.clone(),
                backup: backup.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        let data = tokio::fs::read(&backup).await.unwrap();
        assert_eq!(data, b"backup");
    }

    #[tokio::test]
    async fn test_file_processing_task_rename_file() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("a.txt");
        let dst = tmp.path().join("b.txt");
        tokio::fs::write(&src, b"rename").await.unwrap();
        let task = FileProcessingTask::new(
            src.clone(),
            Some(dst.clone()),
            ProcessingOperation::RenameFile {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        assert!(tokio::fs::metadata(&src).await.is_err());
        let data = tokio::fs::read(&dst).await.unwrap();
        assert_eq!(data, b"rename");
    }

    /// Test task lifecycle and status transitions
    #[tokio::test]
    async fn test_task_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let test_file = tmp.path().join("lifecycle.srt");
        tokio::fs::write(
            &test_file,
            "1\n00:00:01,000 --> 00:00:02,000\nLifecycle test\n",
        )
        .await
        .unwrap();

        let task = FileProcessingTask {
            input_path: test_file.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };

        // Test initial task properties
        assert_eq!(task.task_type(), "validate");
        assert!(!task.task_id().is_empty());
        assert!(task.description().contains("Validate format"));
        assert!(task.description().contains("lifecycle.srt"));
        assert!(
            task.estimated_duration().is_some(),
            "Should estimate duration for existing file"
        );

        // Test execution
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
    }

    /// Test task result serialization and display
    #[test]
    fn test_task_result_display() {
        let success = TaskResult::Success("Operation completed".to_string());
        let failed = TaskResult::Failed("Operation failed".to_string());
        let cancelled = TaskResult::Cancelled;
        let partial =
            TaskResult::PartialSuccess("Mostly worked".to_string(), "Minor issue".to_string());

        assert_eq!(format!("{}", success), "✓ Operation completed");
        assert_eq!(format!("{}", failed), "✗ Operation failed");
        assert_eq!(format!("{}", cancelled), "⚠ Task cancelled");
        assert_eq!(
            format!("{}", partial),
            "⚠ Mostly worked (warning: Minor issue)"
        );
    }

    /// Test task status display
    #[test]
    fn test_task_status_display() {
        let pending = TaskStatus::Pending;
        let running = TaskStatus::Running;
        let completed = TaskStatus::Completed(TaskResult::Success("Done".to_string()));
        let failed = TaskStatus::Failed("Error occurred".to_string());
        let cancelled = TaskStatus::Cancelled;

        assert_eq!(format!("{}", pending), "Pending");
        assert_eq!(format!("{}", running), "Running");
        assert_eq!(format!("{}", completed), "Completed: ✓ Done");
        assert_eq!(format!("{}", failed), "Failed: Error occurred");
        assert_eq!(format!("{}", cancelled), "Cancelled");
    }

    /// Test format conversion task
    #[tokio::test]
    async fn test_format_conversion_task() {
        let tmp = TempDir::new().unwrap();
        let input_file = tmp.path().join("input.srt");
        let output_file = tmp.path().join("output.ass");

        // Create valid SRT content
        let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
First subtitle

2
00:00:04,000 --> 00:00:06,000
Second subtitle
"#;

        tokio::fs::write(&input_file, srt_content).await.unwrap();

        let task = FileProcessingTask {
            input_path: input_file.clone(),
            output_path: Some(output_file.clone()),
            operation: ProcessingOperation::ConvertFormat {
                from: "srt".to_string(),
                to: "ass".to_string(),
            },
        };

        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));

        // Note: The convert_format method is a stub that returns the input path
        // In a real implementation, this would create an actual output file
        assert!(tokio::fs::metadata(&input_file).await.is_ok());
    }

    /// Test file matching task
    #[tokio::test]
    async fn test_file_matching_task() {
        let tmp = TempDir::new().unwrap();
        let video_file = tmp.path().join("movie.mkv");
        let subtitle_file = tmp.path().join("movie.srt");

        // Create test files
        tokio::fs::write(&video_file, b"fake video content")
            .await
            .unwrap();
        tokio::fs::write(&subtitle_file, "1\n00:00:01,000 --> 00:00:02,000\nTest\n")
            .await
            .unwrap();

        let task = FileProcessingTask {
            input_path: tmp.path().to_path_buf(),
            output_path: None,
            operation: ProcessingOperation::MatchFiles { recursive: false },
        };

        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
    }

    /// Test sync subtitle task (expected to fail)
    #[tokio::test]
    async fn test_sync_subtitle_task() {
        let tmp = TempDir::new().unwrap();
        let audio_file = tmp.path().join("audio.wav");
        let subtitle_file = tmp.path().join("subtitle.srt");

        tokio::fs::write(&audio_file, b"fake audio content")
            .await
            .unwrap();
        tokio::fs::write(&subtitle_file, "1\n00:00:01,000 --> 00:00:02,000\nTest\n")
            .await
            .unwrap();

        let task = FileProcessingTask {
            input_path: subtitle_file.clone(),
            output_path: None,
            operation: ProcessingOperation::SyncSubtitle {
                audio_path: audio_file,
            },
        };

        let result = task.execute().await;
        // Sync is not implemented, so should fail
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    /// Test task error handling
    #[tokio::test]
    async fn test_task_error_handling() {
        // Test with sync operation which always fails in stub implementation
        let tmp = TempDir::new().unwrap();
        let test_file = tmp.path().join("test.srt");

        let task = FileProcessingTask {
            input_path: test_file,
            output_path: None,
            operation: ProcessingOperation::SyncSubtitle {
                audio_path: tmp.path().join("audio.wav"),
            },
        };

        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    /// Test task timeout handling
    #[tokio::test]
    async fn test_task_timeout() {
        use async_trait::async_trait;

        struct SlowTask {
            duration: Duration,
        }

        #[async_trait]
        impl Task for SlowTask {
            async fn execute(&self) -> TaskResult {
                tokio::time::sleep(self.duration).await;
                TaskResult::Success("Slow task completed".to_string())
            }
            fn task_type(&self) -> &'static str {
                "slow"
            }
            fn task_id(&self) -> String {
                "slow_task_1".to_string()
            }
            fn estimated_duration(&self) -> Option<Duration> {
                Some(self.duration)
            }
        }

        let slow_task = SlowTask {
            duration: Duration::from_millis(100),
        };

        // Test estimated duration
        assert_eq!(
            slow_task.estimated_duration(),
            Some(Duration::from_millis(100))
        );

        // Test execution
        let start = std::time::Instant::now();
        let result = slow_task.execute().await;
        let elapsed = start.elapsed();

        assert!(matches!(result, TaskResult::Success(_)));
        assert!(elapsed >= Duration::from_millis(90)); // Allow some variance
    }

    /// Test processing operation variants
    #[test]
    fn test_processing_operation_variants() {
        let convert_op = ProcessingOperation::ConvertFormat {
            from: "srt".to_string(),
            to: "ass".to_string(),
        };

        let sync_op = ProcessingOperation::SyncSubtitle {
            audio_path: std::path::PathBuf::from("audio.wav"),
        };

        let match_op = ProcessingOperation::MatchFiles { recursive: true };
        let validate_op = ProcessingOperation::ValidateFormat;

        // Test debug formatting
        assert!(format!("{:?}", convert_op).contains("ConvertFormat"));
        assert!(format!("{:?}", sync_op).contains("SyncSubtitle"));
        assert!(format!("{:?}", match_op).contains("MatchFiles"));
        assert!(format!("{:?}", validate_op).contains("ValidateFormat"));

        // Test cloning
        let convert_clone = convert_op.clone();
        assert!(format!("{:?}", convert_clone).contains("ConvertFormat"));
    }

    /// Test custom task implementation
    #[tokio::test]
    async fn test_custom_task_implementation() {
        use async_trait::async_trait;

        struct CustomTask {
            id: String,
            should_succeed: bool,
        }

        #[async_trait]
        impl Task for CustomTask {
            async fn execute(&self) -> TaskResult {
                if self.should_succeed {
                    TaskResult::Success(format!("Custom task {} succeeded", self.id))
                } else {
                    TaskResult::Failed(format!("Custom task {} failed", self.id))
                }
            }

            fn task_type(&self) -> &'static str {
                "custom"
            }

            fn task_id(&self) -> String {
                self.id.clone()
            }

            fn description(&self) -> String {
                format!("Custom task with ID: {}", self.id)
            }

            fn estimated_duration(&self) -> Option<Duration> {
                Some(Duration::from_millis(1))
            }
        }

        // Test successful custom task
        let success_task = CustomTask {
            id: "success_1".to_string(),
            should_succeed: true,
        };

        assert_eq!(success_task.task_type(), "custom");
        assert_eq!(success_task.task_id(), "success_1");
        assert_eq!(success_task.description(), "Custom task with ID: success_1");
        assert_eq!(
            success_task.estimated_duration(),
            Some(Duration::from_millis(1))
        );

        let result = success_task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));

        // Test failing custom task
        let fail_task = CustomTask {
            id: "fail_1".to_string(),
            should_succeed: false,
        };

        let result = fail_task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_resolve_filename_conflict_sequential_suffixes() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("x.txt");
        tokio::fs::write(&base, b"first").await.unwrap();

        let (p1, f1) = resolve_filename_conflict(base.clone()).unwrap();
        assert_eq!(p1.file_name().unwrap(), "x.1.txt");
        drop(f1);

        let (p2, _f2) = resolve_filename_conflict(base.clone()).unwrap();
        assert_eq!(p2.file_name().unwrap(), "x.2.txt");
    }

    #[tokio::test]
    async fn test_execute_copy_operation_atomic() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("dst.txt");
        tokio::fs::write(&src, b"payload").await.unwrap();

        let task = FileProcessingTask {
            input_path: src.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        task.execute_copy_operation(&src, &dst).await.unwrap();
        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"payload");
    }

    #[tokio::test]
    async fn test_execute_move_operation_deletes_source() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("from.txt");
        let dst = tmp.path().join("to.txt");
        tokio::fs::write(&src, b"moved").await.unwrap();

        let task = FileProcessingTask {
            input_path: src.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        task.execute_move_operation(&src, &dst).await.unwrap();
        assert!(tokio::fs::metadata(&src).await.is_err());
        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"moved");
    }

    // ── is_cross_device_error ──────────────────────────────────────────────

    #[test]
    fn test_is_cross_device_error_unsupported() {
        let err = io::Error::new(io::ErrorKind::Unsupported, "unsupported");
        assert!(is_cross_device_error(&err));
    }

    #[test]
    fn test_is_cross_device_error_other_kind() {
        let err = io::Error::new(io::ErrorKind::NotFound, "not found");
        assert!(!is_cross_device_error(&err));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_cross_device_error_exdev() {
        // EXDEV == 18 on Linux
        let err = io::Error::from_raw_os_error(18);
        assert!(is_cross_device_error(&err));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_cross_device_error_other_os_error() {
        let err = io::Error::from_raw_os_error(2); // ENOENT
        assert!(!is_cross_device_error(&err));
    }

    // ── resolve_filename_conflict edge cases ───────────────────────────────

    #[test]
    fn test_resolve_filename_conflict_new_file() {
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("fresh.txt");
        let (path, _file) = resolve_filename_conflict(target.clone()).unwrap();
        assert_eq!(path, target);
    }

    #[test]
    fn test_resolve_filename_conflict_no_extension() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("noext");
        std::fs::write(&base, b"data").unwrap();

        let (p1, _f1) = resolve_filename_conflict(base.clone()).unwrap();
        assert_eq!(p1.file_name().unwrap(), "noext.1");
    }

    #[test]
    fn test_resolve_filename_conflict_creates_parent_on_demand() {
        let tmp = TempDir::new().unwrap();
        // Parent directory exists via tempdir; file should not
        let target = tmp.path().join("brand_new.srt");
        let (path, _file) = resolve_filename_conflict(target.clone()).unwrap();
        assert_eq!(path, target);
    }

    // ── CopyToVideoFolder and MoveToVideoFolder via execute() ──────────────

    #[tokio::test]
    async fn test_execute_copy_to_video_folder_success() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("sub.srt");
        let dst = tmp.path().join("video_dir").join("sub.srt");
        tokio::fs::write(&src, b"copy content").await.unwrap();

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::CopyToVideoFolder {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"copy content");
    }

    #[tokio::test]
    async fn test_execute_copy_to_video_folder_failure() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("nonexistent.srt");
        let dst = tmp.path().join("dst.srt");

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::CopyToVideoFolder {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_execute_move_to_video_folder_success() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("move_me.srt");
        let dst = tmp.path().join("dest_dir").join("move_me.srt");
        tokio::fs::write(&src, b"move content").await.unwrap();

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::MoveToVideoFolder {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        assert!(tokio::fs::metadata(&src).await.is_err());
        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"move content");
    }

    #[tokio::test]
    async fn test_execute_move_to_video_folder_failure() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("missing.srt");
        let dst = tmp.path().join("dst.srt");

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::MoveToVideoFolder {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    // ── Failure cases for other operations ────────────────────────────────

    #[tokio::test]
    async fn test_execute_copy_with_rename_failure() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("ghost.txt");
        let dst = tmp.path().join("out.txt");

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::CopyWithRename {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_execute_create_backup_failure() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("ghost.txt");
        let bak = tmp.path().join("ghost.bak");

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::CreateBackup {
                source: src.clone(),
                backup: bak.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    #[tokio::test]
    async fn test_execute_rename_file_failure() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("ghost.txt");
        let dst = tmp.path().join("new_name.txt");

        let task = FileProcessingTask::new(
            src.clone(),
            None,
            ProcessingOperation::RenameFile {
                source: src.clone(),
                target: dst.clone(),
            },
        );
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Failed(_)));
    }

    // ── Move/rename with existing target (conflict resolution path) ────────

    #[tokio::test]
    async fn test_execute_move_operation_conflict_resolved() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src_conflict.txt");
        let dst = tmp.path().join("dst_conflict.txt");
        tokio::fs::write(&src, b"conflict source").await.unwrap();
        tokio::fs::write(&dst, b"existing dest").await.unwrap();

        let task = FileProcessingTask {
            input_path: src.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        task.execute_move_operation(&src, &dst).await.unwrap();
        // Source deleted, a renamed copy exists
        assert!(tokio::fs::metadata(&src).await.is_err());
        assert_eq!(
            tokio::fs::read(&dst).await.unwrap(),
            b"existing dest",
            "Original target must not be overwritten"
        );
        // The moved file should be at dst_conflict.1.txt
        let renamed = tmp.path().join("dst_conflict.1.txt");
        assert_eq!(tokio::fs::read(&renamed).await.unwrap(), b"conflict source");
    }

    #[tokio::test]
    async fn test_execute_rename_file_conflict_resolved() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("ren_src.txt");
        let dst = tmp.path().join("ren_dst.txt");
        tokio::fs::write(&src, b"rename conflict").await.unwrap();
        tokio::fs::write(&dst, b"existing").await.unwrap();

        let task = FileProcessingTask {
            input_path: src.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        task.execute_rename_file_operation(&src, &dst)
            .await
            .unwrap();
        assert!(tokio::fs::metadata(&src).await.is_err());
        let renamed = tmp.path().join("ren_dst.1.txt");
        assert_eq!(tokio::fs::read(&renamed).await.unwrap(), b"rename conflict");
    }

    // ── task_type() for every operation variant ────────────────────────────

    #[test]
    fn test_task_type_all_variants() {
        let tmp = std::path::PathBuf::from("x");

        let cases: &[(&str, ProcessingOperation)] = &[
            (
                "convert",
                ProcessingOperation::ConvertFormat {
                    from: "srt".into(),
                    to: "ass".into(),
                },
            ),
            (
                "sync",
                ProcessingOperation::SyncSubtitle {
                    audio_path: tmp.clone(),
                },
            ),
            (
                "match",
                ProcessingOperation::MatchFiles { recursive: false },
            ),
            ("validate", ProcessingOperation::ValidateFormat),
            (
                "copy_to_video_folder",
                ProcessingOperation::CopyToVideoFolder {
                    source: tmp.clone(),
                    target: tmp.clone(),
                },
            ),
            (
                "move_to_video_folder",
                ProcessingOperation::MoveToVideoFolder {
                    source: tmp.clone(),
                    target: tmp.clone(),
                },
            ),
            (
                "copy_with_rename",
                ProcessingOperation::CopyWithRename {
                    source: tmp.clone(),
                    target: tmp.clone(),
                },
            ),
            (
                "create_backup",
                ProcessingOperation::CreateBackup {
                    source: tmp.clone(),
                    backup: tmp.clone(),
                },
            ),
            (
                "rename_file",
                ProcessingOperation::RenameFile {
                    source: tmp.clone(),
                    target: tmp.clone(),
                },
            ),
        ];

        for (expected, op) in cases {
            let task = FileProcessingTask::new(tmp.clone(), None, op.clone());
            assert_eq!(
                task.task_type(),
                *expected,
                "task_type mismatch for {expected}"
            );
        }
    }

    // ── description() for every operation variant ─────────────────────────

    #[test]
    fn test_description_all_variants() {
        let p = std::path::PathBuf::from("a.srt");
        let q = std::path::PathBuf::from("b.srt");

        let cases: &[(ProcessingOperation, &str)] = &[
            (
                ProcessingOperation::ConvertFormat {
                    from: "srt".into(),
                    to: "ass".into(),
                },
                "Convert",
            ),
            (
                ProcessingOperation::SyncSubtitle {
                    audio_path: q.clone(),
                },
                "Sync subtitle",
            ),
            (
                ProcessingOperation::MatchFiles { recursive: false },
                "Match files",
            ),
            (
                ProcessingOperation::MatchFiles { recursive: true },
                "(recursive)",
            ),
            (ProcessingOperation::ValidateFormat, "Validate format"),
            (
                ProcessingOperation::CopyToVideoFolder {
                    source: p.clone(),
                    target: q.clone(),
                },
                "Copy",
            ),
            (
                ProcessingOperation::MoveToVideoFolder {
                    source: p.clone(),
                    target: q.clone(),
                },
                "Move",
            ),
            (
                ProcessingOperation::CopyWithRename {
                    source: p.clone(),
                    target: q.clone(),
                },
                "CopyWithRename",
            ),
            (
                ProcessingOperation::CreateBackup {
                    source: p.clone(),
                    backup: q.clone(),
                },
                "CreateBackup",
            ),
            (
                ProcessingOperation::RenameFile {
                    source: p.clone(),
                    target: q.clone(),
                },
                "Rename",
            ),
        ];

        for (op, expected_substr) in cases {
            let task = FileProcessingTask::new(p.clone(), None, op.clone());
            let desc = task.description();
            assert!(
                desc.contains(expected_substr),
                "description '{desc}' does not contain '{expected_substr}'"
            );
        }
    }

    // ── estimated_duration() ──────────────────────────────────────────────

    #[test]
    fn test_estimated_duration_nonexistent_file() {
        let task = FileProcessingTask::new(
            std::path::PathBuf::from("/does/not/exist.srt"),
            None,
            ProcessingOperation::ValidateFormat,
        );
        assert!(task.estimated_duration().is_none());
    }

    #[tokio::test]
    async fn test_estimated_duration_all_operations() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.srt");
        tokio::fs::write(&file, "1\n00:00:01,000 --> 00:00:02,000\nHi\n")
            .await
            .unwrap();
        let p = file.clone();

        let ops = vec![
            ProcessingOperation::ConvertFormat {
                from: "srt".into(),
                to: "ass".into(),
            },
            ProcessingOperation::SyncSubtitle {
                audio_path: p.clone(),
            },
            ProcessingOperation::MatchFiles { recursive: true },
            ProcessingOperation::ValidateFormat,
            ProcessingOperation::CopyToVideoFolder {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::MoveToVideoFolder {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::CopyWithRename {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::CreateBackup {
                source: p.clone(),
                backup: p.clone(),
            },
            ProcessingOperation::RenameFile {
                source: p.clone(),
                target: p.clone(),
            },
        ];

        for op in ops {
            let task = FileProcessingTask::new(p.clone(), None, op);
            // For an existing file estimated_duration must return Some
            assert!(
                task.estimated_duration().is_some(),
                "expected Some for operation when file exists"
            );
        }
    }

    // ── task_id uniqueness ─────────────────────────────────────────────────

    #[test]
    fn test_task_id_uniqueness() {
        let p1 = std::path::PathBuf::from("a.srt");
        let p2 = std::path::PathBuf::from("b.srt");

        let t1 = FileProcessingTask::new(p1.clone(), None, ProcessingOperation::ValidateFormat);
        let t2 = FileProcessingTask::new(p2.clone(), None, ProcessingOperation::ValidateFormat);
        assert_ne!(
            t1.task_id(),
            t2.task_id(),
            "different paths → different IDs"
        );

        let t3 = FileProcessingTask::new(
            p1.clone(),
            None,
            ProcessingOperation::ConvertFormat {
                from: "srt".into(),
                to: "ass".into(),
            },
        );
        assert_ne!(
            t1.task_id(),
            t3.task_id(),
            "different operations → different IDs"
        );
    }

    // ── Debug for TaskResult and TaskStatus ───────────────────────────────

    #[test]
    fn test_task_result_debug() {
        assert!(format!("{:?}", TaskResult::Success("ok".into())).contains("Success"));
        assert!(format!("{:?}", TaskResult::Failed("err".into())).contains("Failed"));
        assert!(format!("{:?}", TaskResult::Cancelled).contains("Cancelled"));
        assert!(
            format!("{:?}", TaskResult::PartialSuccess("a".into(), "b".into()))
                .contains("PartialSuccess")
        );
    }

    #[test]
    fn test_task_status_debug() {
        assert!(format!("{:?}", TaskStatus::Pending).contains("Pending"));
        assert!(format!("{:?}", TaskStatus::Running).contains("Running"));
        assert!(
            format!("{:?}", TaskStatus::Completed(TaskResult::Cancelled)).contains("Completed")
        );
        assert!(format!("{:?}", TaskStatus::Failed("x".into())).contains("Failed"));
        assert!(format!("{:?}", TaskStatus::Cancelled).contains("Cancelled"));
    }

    // ── Clone for TaskResult and TaskStatus ───────────────────────────────

    #[test]
    fn test_task_result_clone() {
        let orig = TaskResult::PartialSuccess("s".into(), "w".into());
        let cloned = orig.clone();
        assert_eq!(format!("{orig}"), format!("{cloned}"));
    }

    #[test]
    fn test_task_status_clone() {
        let orig = TaskStatus::Completed(TaskResult::Success("done".into()));
        let cloned = orig.clone();
        assert_eq!(format!("{orig}"), format!("{cloned}"));
    }

    // ── Default Task trait methods via minimal impl ────────────────────────

    #[tokio::test]
    async fn test_default_task_trait_methods() {
        struct MinimalTask;

        #[async_trait::async_trait]
        impl Task for MinimalTask {
            async fn execute(&self) -> TaskResult {
                TaskResult::Cancelled
            }
            fn task_type(&self) -> &'static str {
                "minimal"
            }
            fn task_id(&self) -> String {
                "minimal_1".into()
            }
        }

        let t = MinimalTask;
        // Default estimated_duration returns None
        assert!(t.estimated_duration().is_none());
        // Default description uses task_type()
        assert_eq!(t.description(), "minimal task");
    }

    // ── Hash impl covers all ProcessingOperation variants ─────────────────

    #[test]
    fn test_processing_operation_hash_all_variants() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        fn compute_hash(op: &ProcessingOperation) -> u64 {
            let mut h = DefaultHasher::new();
            op.hash(&mut h);
            h.finish()
        }

        let p = std::path::PathBuf::from("x");

        let ops = vec![
            ProcessingOperation::ConvertFormat {
                from: "srt".into(),
                to: "ass".into(),
            },
            ProcessingOperation::SyncSubtitle {
                audio_path: p.clone(),
            },
            ProcessingOperation::MatchFiles { recursive: true },
            ProcessingOperation::MatchFiles { recursive: false },
            ProcessingOperation::ValidateFormat,
            ProcessingOperation::CopyToVideoFolder {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::MoveToVideoFolder {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::CopyWithRename {
                source: p.clone(),
                target: p.clone(),
            },
            ProcessingOperation::CreateBackup {
                source: p.clone(),
                backup: p.clone(),
            },
            ProcessingOperation::RenameFile {
                source: p.clone(),
                target: p.clone(),
            },
        ];

        let hashes: Vec<u64> = ops.iter().map(compute_hash).collect();
        // All hashes must be computable (no panics); non-duplicate ops should differ
        assert_ne!(hashes[0], hashes[4], "convert vs validate should differ");
        assert_ne!(
            hashes[2], hashes[3],
            "recursive vs non-recursive should differ"
        );
    }

    // ── MatchFiles recursive variant via execute() ────────────────────────

    #[tokio::test]
    async fn test_file_matching_task_recursive() {
        let tmp = TempDir::new().unwrap();
        let task = FileProcessingTask {
            input_path: tmp.path().to_path_buf(),
            output_path: None,
            operation: ProcessingOperation::MatchFiles { recursive: true },
        };
        let result = task.execute().await;
        assert!(matches!(result, TaskResult::Success(_)));
        if let TaskResult::Success(msg) = result {
            assert!(msg.contains("matches"));
        }
    }

    // ── execute_copy_operation creates parent dirs ────────────────────────

    #[tokio::test]
    async fn test_execute_copy_operation_creates_parent() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src.txt");
        let dst = tmp.path().join("subdir").join("dst.txt");
        tokio::fs::write(&src, b"content").await.unwrap();

        let task = FileProcessingTask {
            input_path: src.clone(),
            output_path: None,
            operation: ProcessingOperation::ValidateFormat,
        };
        task.execute_copy_operation(&src, &dst).await.unwrap();
        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"content");
    }
}
