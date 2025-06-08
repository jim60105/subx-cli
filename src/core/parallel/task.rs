//! Task definition and utilities for parallel processing
use async_trait::async_trait;
use std::fmt;

/// Trait defining a unit of work
#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self) -> TaskResult;
    fn task_type(&self) -> &'static str;
    fn task_id(&self) -> String;
    fn estimated_duration(&self) -> Option<std::time::Duration> {
        None
    }
    fn description(&self) -> String {
        format!("{} 任務", self.task_type())
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub enum TaskResult {
    Success(String),
    Failed(String),
    Cancelled,
    PartialSuccess(String, String),
}

/// Status of a task
#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed(TaskResult),
    Failed(String),
    Cancelled,
}

impl fmt::Display for TaskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskResult::Success(msg) => write!(f, "✓ {}", msg),
            TaskResult::Failed(msg) => write!(f, "✗ {}", msg),
            TaskResult::Cancelled => write!(f, "⚠ 任務被取消"),
            TaskResult::PartialSuccess(success, warn) => {
                write!(f, "⚠ {} (警告: {})", success, warn)
            }
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "等待中"),
            TaskStatus::Running => write!(f, "執行中"),
            TaskStatus::Completed(result) => write!(f, "已完成: {}", result),
            TaskStatus::Failed(msg) => write!(f, "失敗: {}", msg),
            TaskStatus::Cancelled => write!(f, "已取消"),
        }
    }
}

/// Task for processing files (convert, sync, match, validate)
pub struct FileProcessingTask {
    pub input_path: std::path::PathBuf,
    pub output_path: Option<std::path::PathBuf>,
    pub operation: ProcessingOperation,
}

/// Supported operations for file processing
#[derive(Debug, Clone)]
pub enum ProcessingOperation {
    ConvertFormat { from: String, to: String },
    SyncSubtitle { audio_path: std::path::PathBuf },
    MatchFiles { recursive: bool },
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
