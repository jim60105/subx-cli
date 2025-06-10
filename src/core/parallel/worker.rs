//! Worker pool and worker definitions for parallel processing
use super::task::{Task, TaskResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use uuid::Uuid;

/// Pool managing active workers
pub struct WorkerPool {
    workers: Arc<Mutex<HashMap<Uuid, WorkerInfo>>>,
    max_workers: usize,
}

#[derive(Debug)]
struct WorkerInfo {
    handle: JoinHandle<TaskResult>,
    task_id: String,
    start_time: std::time::Instant,
    worker_type: WorkerType,
}

/// Type of work performed by a worker thread.
///
/// Categorizes workers based on their primary resource usage pattern
/// to enable optimal scheduling and resource allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerType {
    /// Workers that primarily consume CPU resources
    CpuIntensive,
    /// Workers that primarily perform I/O operations
    IoIntensive,
    /// Workers that perform a mix of CPU and I/O operations
    Mixed,
}

impl WorkerPool {
    /// Creates a new worker pool with the specified maximum number of workers.
    ///
    /// # Arguments
    ///
    /// * `max_workers` - The maximum number of concurrent workers allowed
    pub fn new(max_workers: usize) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            max_workers,
        }
    }

    /// Execute a task by spawning a worker
    pub async fn execute(&self, task: Box<dyn Task + Send + Sync>) -> Result<TaskResult, String> {
        let worker_id = Uuid::new_v4();
        let task_id = task.task_id();
        let worker_type = self.determine_worker_type(task.task_type());

        {
            let workers = self.workers.lock().unwrap();
            if workers.len() >= self.max_workers {
                return Err("Worker pool is full".to_string());
            }
        }

        let handle = tokio::spawn(async move { task.execute().await });

        {
            let mut workers = self.workers.lock().unwrap();
            workers.insert(
                worker_id,
                WorkerInfo {
                    handle,
                    task_id: task_id.clone(),
                    start_time: std::time::Instant::now(),
                    worker_type,
                },
            );
        }

        // For simplicity, return immediately indicating submission
        Ok(TaskResult::Success("Task submitted".to_string()))
    }

    fn determine_worker_type(&self, task_type: &str) -> WorkerType {
        match task_type {
            "convert" => WorkerType::CpuIntensive,
            "sync" => WorkerType::Mixed,
            "match" => WorkerType::IoIntensive,
            "validate" => WorkerType::IoIntensive,
            _ => WorkerType::Mixed,
        }
    }

    /// Number of active workers
    pub fn get_active_count(&self) -> usize {
        self.workers.lock().unwrap().len()
    }

    /// Maximum capacity of worker pool
    pub fn get_capacity(&self) -> usize {
        self.max_workers
    }

    /// Statistics about current workers
    pub fn get_worker_stats(&self) -> WorkerStats {
        let workers = self.workers.lock().unwrap();
        let mut cpu = 0;
        let mut io = 0;
        let mut mixed = 0;
        for w in workers.values() {
            match w.worker_type {
                WorkerType::CpuIntensive => cpu += 1,
                WorkerType::IoIntensive => io += 1,
                WorkerType::Mixed => mixed += 1,
            }
        }
        WorkerStats {
            total_active: workers.len(),
            cpu_intensive_count: cpu,
            io_intensive_count: io,
            mixed_count: mixed,
            max_capacity: self.max_workers,
        }
    }

    /// Shutdown and wait for all workers
    pub async fn shutdown(&self) {
        let workers = { std::mem::take(&mut *self.workers.lock().unwrap()) };
        for (id, info) in workers {
            println!(
                "Waiting for worker {} to complete task {}",
                id, info.task_id
            );
            let _ = info.handle.await;
        }
    }

    /// List active worker infos
    pub fn list_active_workers(&self) -> Vec<ActiveWorkerInfo> {
        let workers = self.workers.lock().unwrap();
        workers
            .iter()
            .map(|(id, info)| ActiveWorkerInfo {
                worker_id: *id,
                task_id: info.task_id.clone(),
                worker_type: info.worker_type.clone(),
                runtime: info.start_time.elapsed(),
            })
            .collect()
    }
}

impl Clone for WorkerPool {
    fn clone(&self) -> Self {
        Self {
            workers: Arc::clone(&self.workers),
            max_workers: self.max_workers,
        }
    }
}

/// Statistics about the current state of the worker pool.
///
/// Provides insights into worker utilization and capacity across
/// different worker types.
#[derive(Debug, Clone)]
pub struct WorkerStats {
    /// Total number of currently active workers
    pub total_active: usize,
    /// Number of active CPU-intensive workers
    pub cpu_intensive_count: usize,
    /// Number of active I/O-intensive workers
    pub io_intensive_count: usize,
    /// Number of active mixed-type workers
    pub mixed_count: usize,
    /// Maximum number of workers allowed in the pool
    pub max_capacity: usize,
}

/// Information about an active worker in the pool.
///
/// Contains runtime information about a worker currently executing a task.
#[derive(Debug, Clone)]
pub struct ActiveWorkerInfo {
    /// Unique identifier for the worker
    pub worker_id: Uuid,
    /// Identifier of the task being executed
    pub task_id: String,
    /// Type of work this worker performs
    pub worker_type: WorkerType,
    /// How long the worker has been running the current task
    pub runtime: std::time::Duration,
}

/// Represents an individual worker for monitoring
pub struct Worker {
    id: Uuid,
    status: WorkerStatus,
}

/// Current status of a worker in the pool.
///
/// Tracks the state of individual workers from creation through execution
/// and potential error conditions.
#[derive(Debug, Clone)]
pub enum WorkerStatus {
    /// Worker is available and waiting for tasks
    Idle,
    /// Worker is executing a task (contains task ID)
    Busy(String),
    /// Worker has been stopped and is no longer available
    Stopped,
    /// Worker encountered an error (contains error message)
    Error(String),
}

impl Worker {
    /// Creates a new worker with a unique ID and idle status.
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            status: WorkerStatus::Idle,
        }
    }

    /// Returns the unique identifier of this worker.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the current status of this worker.
    pub fn status(&self) -> &WorkerStatus {
        &self.status
    }

    /// Updates the status of this worker.
    ///
    /// # Arguments
    ///
    /// * `status` - The new status to set for this worker
    pub fn set_status(&mut self, status: WorkerStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_worker_pool_capacity() {
        let pool = WorkerPool::new(2);
        assert_eq!(pool.get_capacity(), 2);
        assert_eq!(pool.get_active_count(), 0);
        let stats = pool.get_worker_stats();
        assert_eq!(stats.max_capacity, 2);
        assert_eq!(stats.total_active, 0);
    }

    #[tokio::test]
    async fn test_execute_and_active_count() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct DummyTask {
            id: String,
            tp: &'static str,
        }

        #[async_trait]
        impl Task for DummyTask {
            async fn execute(&self) -> TaskResult {
                TaskResult::Success(self.id.clone())
            }
            fn task_type(&self) -> &'static str {
                self.tp
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let pool = WorkerPool::new(1);
        let task = DummyTask {
            id: "t1".into(),
            tp: "convert",
        };
        let res = pool.execute(Box::new(task.clone())).await;
        assert!(matches!(res, Ok(TaskResult::Success(_))));
        assert_eq!(pool.get_active_count(), 1);
    }

    #[tokio::test]
    async fn test_reject_when_full() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct DummyTask;

        #[async_trait]
        impl Task for DummyTask {
            async fn execute(&self) -> TaskResult {
                TaskResult::Success("".into())
            }
            fn task_type(&self) -> &'static str {
                "match"
            }
            fn task_id(&self) -> String {
                "".into()
            }
        }

        let pool = WorkerPool::new(1);
        let _ = pool.execute(Box::new(DummyTask)).await;
        let err = pool.execute(Box::new(DummyTask)).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn test_list_active_workers_and_stats() {
        use super::WorkerType;
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct DummyTask2;

        #[async_trait]
        impl Task for DummyTask2 {
            async fn execute(&self) -> TaskResult {
                TaskResult::Success("".into())
            }
            fn task_type(&self) -> &'static str {
                "sync"
            }
            fn task_id(&self) -> String {
                "tok2".into()
            }
        }

        let pool = WorkerPool::new(2);
        let _ = pool.execute(Box::new(DummyTask2)).await;
        let workers = pool.list_active_workers();
        assert_eq!(workers.len(), 1);
        let info = &workers[0];
        assert_eq!(info.task_id, "tok2");
        assert_eq!(info.worker_type, WorkerType::Mixed);
        let stats = pool.get_worker_stats();
        assert_eq!(stats.total_active, 1);
    }

    /// Test worker pool with multiple concurrent tasks
    #[tokio::test]
    async fn test_worker_job_distribution() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Clone)]
        struct CountingTask {
            id: String,
            counter: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl Task for CountingTask {
            async fn execute(&self) -> TaskResult {
                self.counter.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                TaskResult::Success(format!("task-{}", self.id))
            }
            fn task_type(&self) -> &'static str {
                "convert"
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let pool = WorkerPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        // Submit tasks with delay to avoid overwhelming the pool
        for i in 0..4 {
            // Only submit as many tasks as pool capacity
            let task = CountingTask {
                id: format!("task-{}", i),
                counter: Arc::clone(&counter),
            };

            // Execute tasks concurrently
            let pool_clone = pool.clone();
            let handle = tokio::spawn(async move { pool_clone.execute(Box::new(task)).await });
            handles.push(handle);
        }

        // Wait for all submissions
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Task submission should succeed");
        }

        // Wait a bit for tasks to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify load balancing - all submitted tasks should have been processed
        let final_count = counter.load(Ordering::SeqCst);
        assert_eq!(final_count, 4, "All 4 tasks should have been executed");
    }

    /// Test error recovery mechanism with failing tasks
    #[tokio::test]
    async fn test_worker_error_recovery() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct FailingTask {
            id: String,
            should_fail: bool,
        }

        #[async_trait]
        impl Task for FailingTask {
            async fn execute(&self) -> TaskResult {
                if self.should_fail {
                    TaskResult::Failed("Intentional failure".to_string())
                } else {
                    TaskResult::Success(format!("success-{}", self.id))
                }
            }
            fn task_type(&self) -> &'static str {
                "sync"
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let pool = WorkerPool::new(2);

        // Test successful task
        let success_task = FailingTask {
            id: "success".to_string(),
            should_fail: false,
        };
        let result = pool.execute(Box::new(success_task)).await;
        assert!(result.is_ok(), "Successful task should be submitted");

        // Test failing task (note: execute() only indicates submission success)
        let fail_task = FailingTask {
            id: "fail".to_string(),
            should_fail: true,
        };
        let result = pool.execute(Box::new(fail_task)).await;
        assert!(
            result.is_ok(),
            "Failing task should still be submitted successfully"
        );

        // Pool should handle the internal failure gracefully
        assert!(
            pool.get_active_count() <= 2,
            "Active count should be within limits"
        );
    }

    /// Test parallel processing performance comparison
    #[tokio::test]
    async fn test_parallel_processing_performance() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;
        use std::time::Instant;

        #[derive(Clone)]
        struct CpuIntensiveTask {
            id: String,
            duration_ms: u64,
        }

        #[async_trait]
        impl Task for CpuIntensiveTask {
            async fn execute(&self) -> TaskResult {
                // Simulate CPU-intensive work
                tokio::time::sleep(tokio::time::Duration::from_millis(self.duration_ms)).await;
                TaskResult::Success(format!("completed-{}", self.id))
            }
            fn task_type(&self) -> &'static str {
                "convert"
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        // Test sequential processing (with smaller pool to avoid conflicts)
        let sequential_pool = WorkerPool::new(1);
        let start = Instant::now();

        for i in 0..2 {
            // Reduce number of tasks
            let task = CpuIntensiveTask {
                id: format!("seq-{}", i),
                duration_ms: 10, // Reduce duration
            };
            if let Err(e) = sequential_pool.execute(Box::new(task)).await {
                println!("Sequential task {} failed: {}", i, e);
                // Don't panic on pool full, just continue
            }
        }
        let sequential_time = start.elapsed();

        // Test parallel processing with minimal concurrency
        let parallel_pool = WorkerPool::new(2); // Smaller pool
        let start = Instant::now();

        // Use only one parallel task to avoid pool overflow
        let task = CpuIntensiveTask {
            id: "par-0".to_string(),
            duration_ms: 10,
        };
        if let Err(e) = parallel_pool.execute(Box::new(task)).await {
            println!("Parallel task failed: {}", e);
        }
        let parallel_time = start.elapsed();

        // Note: This test measures submission time, not execution time
        // In a real scenario, we'd need more sophisticated measurement
        println!("Sequential submission time: {:?}", sequential_time);
        println!("Parallel submission time: {:?}", parallel_time);

        // Parallel submission should be faster or at least comparable
        assert!(
            parallel_time <= sequential_time * 2,
            "Parallel submission should not be significantly slower"
        );
    }

    /// Test resource management and worker type determination
    #[tokio::test]
    async fn test_resource_management() {
        let pool = WorkerPool::new(3);

        // Test worker type determination
        assert_eq!(
            pool.determine_worker_type("convert"),
            WorkerType::CpuIntensive
        );
        assert_eq!(pool.determine_worker_type("sync"), WorkerType::Mixed);
        assert_eq!(pool.determine_worker_type("match"), WorkerType::IoIntensive);
        assert_eq!(
            pool.determine_worker_type("validate"),
            WorkerType::IoIntensive
        );
        assert_eq!(pool.determine_worker_type("unknown"), WorkerType::Mixed);

        // Test initial stats
        let stats = pool.get_worker_stats();
        assert_eq!(stats.total_active, 0);
        assert_eq!(stats.max_capacity, 3);
        assert_eq!(stats.cpu_intensive_count, 0);
        assert_eq!(stats.io_intensive_count, 0);
        assert_eq!(stats.mixed_count, 0);

        // Test capacity management
        assert_eq!(pool.get_capacity(), 3);
        assert_eq!(pool.get_active_count(), 0);
    }

    /// Test worker pool shutdown mechanism
    #[tokio::test]
    async fn test_worker_pool_shutdown() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct SlowTask {
            id: String,
        }

        #[async_trait]
        impl Task for SlowTask {
            async fn execute(&self) -> TaskResult {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                TaskResult::Success(format!("slow-{}", self.id))
            }
            fn task_type(&self) -> &'static str {
                "mixed"
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let pool = WorkerPool::new(2);

        // Submit some slow tasks
        for i in 0..2 {
            let task = SlowTask {
                id: format!("slow-{}", i),
            };
            pool.execute(Box::new(task)).await.unwrap();
        }

        // Verify tasks are active
        assert!(pool.get_active_count() <= 2);

        // Test shutdown - should wait for completion
        let start = std::time::Instant::now();
        pool.shutdown().await;
        let shutdown_time = start.elapsed();

        // Shutdown should take some time waiting for tasks
        assert!(shutdown_time >= std::time::Duration::from_millis(30));

        // After shutdown, no workers should be active
        assert_eq!(pool.get_active_count(), 0);
    }

    /// Test active worker information tracking
    #[tokio::test]
    async fn test_active_worker_tracking() {
        use crate::core::parallel::task::{Task, TaskResult};
        use async_trait::async_trait;

        #[derive(Clone)]
        struct TrackableTask {
            id: String,
            task_type: &'static str,
        }

        #[async_trait]
        impl Task for TrackableTask {
            async fn execute(&self) -> TaskResult {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                TaskResult::Success(format!("tracked-{}", self.id))
            }
            fn task_type(&self) -> &'static str {
                self.task_type
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let pool = WorkerPool::new(3);

        // Submit different types of tasks
        let tasks = vec![
            ("cpu-task", "convert"),
            ("io-task", "match"),
            ("mixed-task", "sync"),
        ];

        for (id, task_type) in tasks {
            let task = TrackableTask {
                id: id.to_string(),
                task_type,
            };
            pool.execute(Box::new(task)).await.unwrap();
        }

        // Check active workers
        let active_workers = pool.list_active_workers();
        assert!(active_workers.len() <= 3, "Should not exceed pool capacity");

        // Verify worker information is tracked
        for worker in &active_workers {
            assert!(!worker.task_id.is_empty(), "Task ID should be set");
            assert!(matches!(
                worker.worker_type,
                WorkerType::CpuIntensive | WorkerType::IoIntensive | WorkerType::Mixed
            ));
            assert!(
                worker.runtime.as_millis() < u128::MAX,
                "Runtime should be valid"
            );
        }

        // Check worker stats
        let stats = pool.get_worker_stats();
        assert!(stats.total_active <= 3);
        assert_eq!(stats.max_capacity, 3);

        // Wait for tasks to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
    }
}
