use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use subx_cli::core::parallel::task::{FileProcessingTask, ProcessingOperation, Task, TaskResult};

/// Create a series of generic file processing tasks for testing
pub async fn create_test_processing_tasks(count: usize) -> Vec<Box<dyn Task + Send + Sync>> {
    let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
    for i in 0..count {
        let input = PathBuf::from(format!("test_file_{}.srt", i));
        let task = FileProcessingTask {
            input_path: input.clone(),
            output_path: None,
            operation: ProcessingOperation::ConvertFormat {
                from: "srt".into(),
                to: "ass".into(),
            },
        };
        tasks.push(Box::new(task));
    }
    tasks
}

/// Create CPU-intensive tasks (simulate workload via sleep)
#[allow(dead_code)]
pub async fn create_cpu_intensive_tasks(count: usize) -> Vec<Box<dyn Task + Send + Sync>> {
    let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
    for _ in 0..count {
        struct SleepTask(Duration);
        #[async_trait]
        impl Task for SleepTask {
            async fn execute(&self) -> TaskResult {
                tokio::time::sleep(self.0).await;
                TaskResult::Success("done".into())
            }
            fn task_type(&self) -> &'static str {
                "sleep"
            }
            fn task_id(&self) -> String {
                format!("sleep_{:?}", self.0)
            }
        }
        tasks.push(Box::new(SleepTask(Duration::from_millis(10))));
    }
    tasks
}

/// Create a simple successful task for testing error recovery
#[allow(dead_code)]
pub fn create_success_task() -> Box<dyn Task + Send + Sync> {
    struct SuccessTask;
    #[async_trait]
    impl Task for SuccessTask {
        async fn execute(&self) -> TaskResult {
            TaskResult::Success("ok".into())
        }
        fn task_type(&self) -> &'static str {
            "success"
        }
        fn task_id(&self) -> String {
            "success".into()
        }
    }
    Box::new(SuccessTask)
}

/// Create a task that fails for testing error recovery
#[allow(dead_code)]
pub fn create_failure_task() -> Box<dyn Task + Send + Sync> {
    struct FailTask;
    #[async_trait]
    impl Task for FailTask {
        async fn execute(&self) -> TaskResult {
            TaskResult::Failed("err".into())
        }
        fn task_type(&self) -> &'static str {
            "fail"
        }
        fn task_id(&self) -> String {
            "fail".into()
        }
    }
    Box::new(FailTask)
}

/// Create tasks with implicit priority for scheduler testing
#[allow(dead_code)]
pub async fn create_prioritized_tasks() -> Vec<Box<dyn Task + Send + Sync>> {
    // Use same generic tasks; priority simulation is done in tests
    create_test_processing_tasks(4).await
}

/// Verify basic task execution results
#[allow(dead_code)]
pub fn verify_task_results(results: &[TaskResult]) -> bool {
    results.iter().all(|r| matches!(r, TaskResult::Success(_)))
}
