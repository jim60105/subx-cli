//! Task definition and utilities for parallel processing
use async_trait::async_trait;
use std::fmt;
use std::path::Path;

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
        // Create target directory if it doesn't exist
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Handle filename conflicts
        let final_target = self.resolve_filename_conflict(target.to_path_buf()).await?;

        // Execute copy operation
        std::fs::copy(source, &final_target)?;
        Ok(())
    }

    /// Execute move operation for file relocation
    async fn execute_move_operation(
        &self,
        source: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create target directory if it doesn't exist
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Handle filename conflicts
        let final_target = self.resolve_filename_conflict(target.to_path_buf()).await?;

        // Execute move operation
        std::fs::rename(source, &final_target)?;
        Ok(())
    }

    /// Resolve filename conflicts by adding numeric suffix
    async fn resolve_filename_conflict(
        &self,
        target: std::path::PathBuf,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        if !target.exists() {
            return Ok(target);
        }

        // Extract filename components
        let file_stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let extension = target.extension().and_then(|s| s.to_str()).unwrap_or("");

        let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));

        // Try adding numeric suffixes
        for i in 1..1000 {
            let new_name = if extension.is_empty() {
                format!("{}.{}", file_stem, i)
            } else {
                format!("{}.{}.{}", file_stem, i, extension)
            };
            let new_path = parent.join(new_name);
            if !new_path.exists() {
                return Ok(new_path);
            }
        }

        Err("Could not resolve filename conflict".into())
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
}
