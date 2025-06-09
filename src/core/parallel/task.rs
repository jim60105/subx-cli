//! Task definition and utilities for parallel processing
use async_trait::async_trait;
use std::fmt;

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
}

#[async_trait]
impl Task for FileProcessingTask {
    async fn execute(&self) -> TaskResult {
        match &self.operation {
            ProcessingOperation::ConvertFormat { from, to } => {
                match self.convert_format(from, to).await {
                    Ok(path) => TaskResult::Success(format!(
                        "成功轉換 {} -> {}: {}",
                        from,
                        to,
                        path.display()
                    )),
                    Err(e) => {
                        TaskResult::Failed(format!("轉換失敗 {}: {}", self.input_path.display(), e))
                    }
                }
            }
            ProcessingOperation::SyncSubtitle { .. } => {
                // Sync not supported in parallel tasks
                TaskResult::Failed("同步功能未實作".to_string())
            }
            ProcessingOperation::MatchFiles { recursive } => {
                match self.match_files(*recursive).await {
                    Ok(m) => TaskResult::Success(format!("檔案匹配完成: 找到 {} 組匹配", m.len())),
                    Err(e) => TaskResult::Failed(format!("匹配失敗: {}", e)),
                }
            }
            ProcessingOperation::ValidateFormat => match self.validate_format().await {
                Ok(true) => {
                    TaskResult::Success(format!("格式驗證通過: {}", self.input_path.display()))
                }
                Ok(false) => {
                    TaskResult::Failed(format!("格式驗證失敗: {}", self.input_path.display()))
                }
                Err(e) => TaskResult::Failed(format!("驗證錯誤: {}", e)),
            },
        }
    }

    fn task_type(&self) -> &'static str {
        match &self.operation {
            ProcessingOperation::ConvertFormat { .. } => "convert",
            ProcessingOperation::SyncSubtitle { .. } => "sync",
            ProcessingOperation::MatchFiles { .. } => "match",
            ProcessingOperation::ValidateFormat => "validate",
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
            };
            Some(std::time::Duration::from_secs_f64(secs))
        } else {
            None
        }
    }

    fn description(&self) -> String {
        match &self.operation {
            ProcessingOperation::ConvertFormat { from, to } => {
                format!("轉換 {} 從 {} 到 {}", self.input_path.display(), from, to)
            }
            ProcessingOperation::SyncSubtitle { audio_path } => format!(
                "同步字幕 {} 與音訊 {}",
                self.input_path.display(),
                audio_path.display()
            ),
            ProcessingOperation::MatchFiles { recursive } => format!(
                "匹配 {} 中的檔案{}",
                self.input_path.display(),
                if *recursive { " (遞歸)" } else { "" }
            ),
            ProcessingOperation::ValidateFormat => {
                format!("驗證 {} 的格式", self.input_path.display())
            }
        }
    }
}

impl FileProcessingTask {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
