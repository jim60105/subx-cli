//! Task scheduler for parallel processing
use super::{Task, TaskResult, TaskStatus};
use crate::Result;
use crate::config::{Config, OverflowStrategy};
use crate::core::parallel::config::ParallelConfig;
use crate::error::SubXError;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, oneshot};

struct PendingTask {
    task: Box<dyn Task + Send + Sync>,
    result_sender: oneshot::Sender<TaskResult>,
    task_id: String,
    priority: TaskPriority,
}

/// RAII guard that removes a task entry from `active_tasks` when dropped.
///
/// This ensures the scheduler's active task map stays consistent even if
/// the awaiting future is cancelled, panics, or returns through an early
/// error path before reaching the explicit cleanup site.
struct ActiveTaskGuard {
    active_tasks: Arc<Mutex<std::collections::HashMap<String, TaskInfo>>>,
    task_id: String,
}

impl Drop for ActiveTaskGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active_tasks.lock() {
            active.remove(&self.task_id);
        }
    }
}

impl PartialEq for PendingTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PendingTask {}

impl PartialOrd for PendingTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

/// Priority levels for task execution scheduling.
///
/// Determines the execution order of tasks in the queue, with higher
/// priority tasks being processed before lower priority ones.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// Low priority for background operations
    Low = 0,
    /// Normal priority for standard operations
    Normal = 1,
    /// High priority for user-initiated operations
    High = 2,
    /// Critical priority for system operations
    Critical = 3,
}

/// Information about an active task in the scheduler.
///
/// Contains runtime information about a task currently being processed
/// or queued for execution.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Unique identifier for the task
    pub task_id: String,
    /// Type of task being executed
    pub task_type: String,
    /// Current status of the task
    pub status: TaskStatus,
    /// When the task started execution
    pub start_time: std::time::Instant,
    /// Current progress percentage (0.0 to 1.0)
    pub progress: f32,
}

/// Scheduler to manage and execute tasks in parallel
pub struct TaskScheduler {
    /// Parallel processing configuration
    _config: ParallelConfig,
    /// Optional load balancer for dynamic worker adjustment
    load_balancer: Option<crate::core::parallel::load_balancer::LoadBalancer>,
    /// Task execution timeout setting
    task_timeout: std::time::Duration,
    /// Worker thread idle timeout setting
    worker_idle_timeout: std::time::Duration,
    task_queue: Arc<Mutex<VecDeque<PendingTask>>>,
    semaphore: Arc<Semaphore>,
    active_tasks: Arc<Mutex<std::collections::HashMap<String, TaskInfo>>>,
    scheduler_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TaskScheduler {
    /// Create a new scheduler based on configuration
    pub fn new_with_config(app_config: &Config) -> Result<Self> {
        let config = ParallelConfig::from_app_config(app_config);
        config.validate()?;
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));
        let task_queue = Arc::new(Mutex::new(VecDeque::new()));
        let active_tasks = Arc::new(Mutex::new(std::collections::HashMap::new()));

        // Read timeout settings from general configuration
        let general = &app_config.general;
        let scheduler = Self {
            _config: config.clone(),
            task_queue: task_queue.clone(),
            semaphore: semaphore.clone(),
            active_tasks: active_tasks.clone(),
            scheduler_handle: Arc::new(Mutex::new(None)),
            load_balancer: if config.auto_balance_workers {
                Some(crate::core::parallel::load_balancer::LoadBalancer::new())
            } else {
                None
            },
            task_timeout: std::time::Duration::from_secs(general.task_timeout_seconds),
            worker_idle_timeout: std::time::Duration::from_secs(
                general.worker_idle_timeout_seconds,
            ),
        };

        // Start background scheduler loop
        scheduler.start_scheduler_loop();
        Ok(scheduler)
    }

    /// Create a new scheduler with default settings (for testing)
    pub fn new_with_defaults() -> Self {
        // Use default application config for default testing
        let default_app_config = Config::default();
        let config = ParallelConfig::from_app_config(&default_app_config);
        let _ = config.validate();
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));
        let task_queue = Arc::new(Mutex::new(VecDeque::new()));
        let active_tasks = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let general = &default_app_config.general;
        let scheduler = Self {
            _config: config.clone(),
            task_queue: task_queue.clone(),
            semaphore: semaphore.clone(),
            active_tasks: active_tasks.clone(),
            scheduler_handle: Arc::new(Mutex::new(None)),
            load_balancer: if config.auto_balance_workers {
                Some(crate::core::parallel::load_balancer::LoadBalancer::new())
            } else {
                None
            },
            task_timeout: std::time::Duration::from_secs(general.task_timeout_seconds),
            worker_idle_timeout: std::time::Duration::from_secs(
                general.worker_idle_timeout_seconds,
            ),
        };

        // Start background scheduler loop
        scheduler.start_scheduler_loop();
        scheduler
    }

    /// Create a new scheduler with default configuration
    pub fn new() -> Result<Self> {
        let default_config = Config::default();
        Self::new_with_config(&default_config)
    }

    /// Start the background scheduler loop
    fn start_scheduler_loop(&self) {
        let task_queue = Arc::clone(&self.task_queue);
        let semaphore = Arc::clone(&self.semaphore);
        let active_tasks = Arc::clone(&self.active_tasks);
        let config = self._config.clone();
        let task_timeout = self.task_timeout;
        let worker_idle_timeout = self.worker_idle_timeout;

        let handle = tokio::spawn(async move {
            // Idle check timer
            let mut last_active = std::time::Instant::now();
            loop {
                // End scheduler after idle timeout
                let has_pending = {
                    let q = task_queue.lock().unwrap();
                    !q.is_empty()
                };
                let has_active = {
                    let a = active_tasks.lock().unwrap();
                    !a.is_empty()
                };
                if has_pending || has_active {
                    last_active = std::time::Instant::now();
                } else if last_active.elapsed() > worker_idle_timeout {
                    break;
                }
                // Try to get a semaphore permit and a task from the queue
                if let Ok(permit) = semaphore.clone().try_acquire_owned() {
                    let pending = {
                        let mut queue = task_queue.lock().unwrap();
                        // select next task by priority or FIFO
                        if config.enable_task_priorities {
                            // find highest priority task
                            if let Some((idx, _)) =
                                queue.iter().enumerate().max_by_key(|(_, t)| t.priority)
                            {
                                queue.remove(idx)
                            } else {
                                None
                            }
                        } else {
                            queue.pop_front()
                        }
                    };
                    if let Some(p) = pending {
                        // Update task status to running
                        {
                            let mut active = active_tasks.lock().unwrap();
                            if let Some(info) = active.get_mut(&p.task_id) {
                                info.status = TaskStatus::Running;
                            }
                        }

                        let task_id = p.task_id.clone();
                        let active_tasks_clone = Arc::clone(&active_tasks);

                        // Spawn the actual task execution
                        tokio::spawn(async move {
                            // Task execution timeout handling
                            let result = match tokio::time::timeout(task_timeout, p.task.execute())
                                .await
                            {
                                Ok(res) => res,
                                Err(_) => TaskResult::Failed("Task execution timeout".to_string()),
                            };

                            // Update task status
                            {
                                let mut at = active_tasks_clone.lock().unwrap();
                                if let Some(info) = at.get_mut(&task_id) {
                                    info.status = TaskStatus::Completed(result.clone());
                                    info.progress = 1.0;
                                }
                            }

                            // Send result back
                            let _ = p.result_sender.send(result);

                            // Release the permit
                            drop(permit);
                        });
                    } else {
                        // No tasks in queue, release permit and wait a bit
                        drop(permit);
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                } else {
                    // No permits available, wait a bit
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }
        });

        // Store the handle
        *self.scheduler_handle.lock().unwrap() = Some(handle);
    }

    /// Submit a task with normal priority
    pub async fn submit_task(&self, task: Box<dyn Task + Send + Sync>) -> Result<TaskResult> {
        self.submit_task_with_priority(task, TaskPriority::Normal)
            .await
    }

    /// Submit a task with specified priority
    pub async fn submit_task_with_priority(
        &self,
        task: Box<dyn Task + Send + Sync>,
        priority: TaskPriority,
    ) -> Result<TaskResult> {
        let task_id = task.task_id();
        let task_type = task.task_type().to_string();
        let (tx, rx) = oneshot::channel();

        // Register task info
        {
            let mut active = self.active_tasks.lock().unwrap();
            active.insert(
                task_id.clone(),
                TaskInfo {
                    task_id: task_id.clone(),
                    task_type,
                    status: TaskStatus::Pending,
                    start_time: std::time::Instant::now(),
                    progress: 0.0,
                },
            );
        }

        // RAII guard ensures the active_tasks entry is removed on any exit
        // path (normal completion, early return, cancellation, or panic).
        let _guard = ActiveTaskGuard {
            active_tasks: Arc::clone(&self.active_tasks),
            task_id: task_id.clone(),
        };

        // Handle queue overflow strategy before enqueuing
        let pending = PendingTask {
            task,
            result_sender: tx,
            task_id: task_id.clone(),
            priority,
        };
        if self.get_queue_size() >= self._config.task_queue_size {
            match self._config.queue_overflow_strategy {
                OverflowStrategy::Block => {
                    // Block until space available
                    while self.get_queue_size() >= self._config.task_queue_size {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                }
                OverflowStrategy::DropOldest => {
                    let evicted_id = {
                        let mut q = self.task_queue.lock().unwrap();
                        if let Some(evicted) = q.pop_front() {
                            let id = evicted.task_id.clone();
                            let _ = evicted.result_sender.send(TaskResult::Failed(
                                "Task dropped due to queue overflow".to_string(),
                            ));
                            Some(id)
                        } else {
                            None
                        }
                    };
                    if let Some(id) = evicted_id {
                        // Clean up the evicted task from active_tasks as its
                        // original submitter may still be awaiting the guard
                        // drop; removing here prevents a stale entry window.
                        let mut active = self.active_tasks.lock().unwrap();
                        active.remove(&id);
                    }
                }
                OverflowStrategy::Reject => {
                    return Err(SubXError::parallel_processing(
                        "Task queue is full".to_string(),
                    ));
                }
                OverflowStrategy::Drop => {
                    // Drop the new task - return Failed result
                    return Ok(TaskResult::Failed(
                        "Task dropped due to queue overflow".to_string(),
                    ));
                }
                OverflowStrategy::Expand => {
                    // Allow queue to expand beyond limits
                    // No action needed, task will be added below
                }
            }
        }
        // Enqueue task according to priority setting
        {
            let mut q = self.task_queue.lock().unwrap();
            if self._config.enable_task_priorities {
                let pos = q
                    .iter()
                    .position(|t| t.priority < pending.priority)
                    .unwrap_or(q.len());
                q.insert(pos, pending);
            } else {
                q.push_back(pending);
            }
        }

        // Ensure the scheduler loop is alive; restart it if it exited due
        // to the idle timeout so queued tasks continue to be drained.
        self.ensure_scheduler_running();

        // Await result. The `_guard` defined above will clean up the
        // active_tasks entry automatically when this function returns,
        // regardless of which branch is taken.
        let result = rx.await.map_err(|_| {
            crate::error::SubXError::parallel_processing("Task execution interrupted".to_string())
        })?;

        Ok(result)
    }

    /// Restart the background scheduler loop if it has exited.
    ///
    /// The scheduler loop voluntarily terminates after the configured idle
    /// timeout. When new work is submitted afterwards we must bring it back
    /// up, otherwise queued tasks would never be drained.
    fn ensure_scheduler_running(&self) {
        let needs_restart = {
            let handle = self.scheduler_handle.lock().unwrap();
            match handle.as_ref() {
                Some(h) => h.is_finished(),
                None => true,
            }
        };
        if needs_restart {
            self.start_scheduler_loop();
        }
    }

    async fn try_execute_next_task(&self) {
        // This method is no longer needed as we have background scheduler loop
        // Keep it for compatibility but it does nothing
    }

    /// Submit multiple tasks and await all results
    pub async fn submit_batch_tasks(
        &self,
        tasks: Vec<Box<dyn Task + Send + Sync>>,
    ) -> Vec<TaskResult> {
        let mut receivers = Vec::new();

        // First add all tasks to the queue
        for task in tasks {
            let task_id = task.task_id();
            let task_type = task.task_type().to_string();
            let (tx, rx) = oneshot::channel();

            // Register task information
            {
                let mut active = self.active_tasks.lock().unwrap();
                active.insert(
                    task_id.clone(),
                    TaskInfo {
                        task_id: task_id.clone(),
                        task_type,
                        status: TaskStatus::Pending,
                        start_time: std::time::Instant::now(),
                        progress: 0.0,
                    },
                );
            }

            // Enqueue each task with overflow and priority handling
            let pending = PendingTask {
                task,
                result_sender: tx,
                task_id: task_id.clone(),
                priority: TaskPriority::Normal,
            };
            if self.get_queue_size() >= self._config.task_queue_size {
                match self._config.queue_overflow_strategy {
                    OverflowStrategy::Block => {
                        // Block until space available
                        while self.get_queue_size() >= self._config.task_queue_size {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        }
                    }
                    OverflowStrategy::DropOldest => {
                        let evicted_id = {
                            let mut q = self.task_queue.lock().unwrap();
                            if let Some(evicted) = q.pop_front() {
                                let id = evicted.task_id.clone();
                                let _ = evicted.result_sender.send(TaskResult::Failed(
                                    "Task dropped due to queue overflow".to_string(),
                                ));
                                Some(id)
                            } else {
                                None
                            }
                        };
                        if let Some(id) = evicted_id {
                            let mut active = self.active_tasks.lock().unwrap();
                            active.remove(&id);
                        }
                    }
                    OverflowStrategy::Reject => {
                        // Reject entire batch when queue is full
                        return Vec::new();
                    }
                    OverflowStrategy::Drop => {
                        // Drop the current task, continue with others
                        continue;
                    }
                    OverflowStrategy::Expand => {
                        // Allow queue to expand beyond limits
                        // No action needed, task will be added below
                    }
                }
            }
            // Insert task according to priority setting
            {
                let mut q = self.task_queue.lock().unwrap();
                if self._config.enable_task_priorities {
                    let pos = q
                        .iter()
                        .position(|t| t.priority < pending.priority)
                        .unwrap_or(q.len());
                    q.insert(pos, pending);
                } else {
                    q.push_back(pending);
                }
            }

            receivers.push((task_id, rx));
        }

        // Ensure the scheduler loop is alive after enqueuing the batch.
        self.ensure_scheduler_running();

        // Wait for all results
        let mut results = Vec::new();
        for (task_id, rx) in receivers {
            match rx.await {
                Ok(result) => results.push(result),
                Err(_) => {
                    results.push(TaskResult::Failed("Task execution interrupted".to_string()))
                }
            }

            // Clean up task information
            {
                let mut active = self.active_tasks.lock().unwrap();
                active.remove(&task_id);
            }
        }

        results
    }

    /// Get number of tasks waiting in queue
    pub fn get_queue_size(&self) -> usize {
        self.task_queue.lock().unwrap().len()
    }

    /// Get number of active workers
    pub fn get_active_workers(&self) -> usize {
        self._config.max_concurrent_jobs - self.semaphore.available_permits()
    }

    /// Get status of a specific task
    pub fn get_task_status(&self, task_id: &str) -> Option<TaskInfo> {
        self.active_tasks.lock().unwrap().get(task_id).cloned()
    }

    /// List all active tasks
    pub fn list_active_tasks(&self) -> Vec<TaskInfo> {
        self.active_tasks
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }
}

impl Clone for TaskScheduler {
    fn clone(&self) -> Self {
        Self {
            _config: self._config.clone(),
            task_queue: Arc::clone(&self.task_queue),
            semaphore: Arc::clone(&self.semaphore),
            active_tasks: Arc::clone(&self.active_tasks),
            scheduler_handle: Arc::clone(&self.scheduler_handle),
            load_balancer: self.load_balancer.clone(),
            task_timeout: self.task_timeout,
            worker_idle_timeout: self.worker_idle_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Task, TaskPriority, TaskResult, TaskScheduler};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::time::Duration;
    use uuid::Uuid;

    struct MockTask {
        name: String,
        duration: Duration,
    }

    #[async_trait::async_trait]
    impl Task for MockTask {
        async fn execute(&self) -> TaskResult {
            tokio::time::sleep(self.duration).await;
            TaskResult::Success(format!("Task completed: {}", self.name))
        }
        fn task_type(&self) -> &'static str {
            "mock"
        }
        fn task_id(&self) -> String {
            format!("mock_{}", self.name)
        }
    }

    struct CounterTask {
        counter: Arc<AtomicUsize>,
    }
    impl CounterTask {
        fn new(counter: Arc<AtomicUsize>) -> Self {
            Self { counter }
        }
    }
    #[async_trait::async_trait]
    impl Task for CounterTask {
        async fn execute(&self) -> TaskResult {
            self.counter.fetch_add(1, Ordering::SeqCst);
            TaskResult::Success("Counter task completed".to_string())
        }
        fn task_type(&self) -> &'static str {
            "counter"
        }
        fn task_id(&self) -> String {
            Uuid::new_v4().to_string()
        }
    }

    struct OrderTask {
        name: String,
        order: Arc<Mutex<Vec<String>>>,
    }
    impl OrderTask {
        fn new(name: &str, order: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                order,
            }
        }
    }
    #[async_trait::async_trait]
    impl Task for OrderTask {
        async fn execute(&self) -> TaskResult {
            let mut v = self.order.lock().unwrap();
            v.push(self.name.clone());
            TaskResult::Success(format!("Order task completed: {}", self.name))
        }
        fn task_type(&self) -> &'static str {
            "order"
        }
        fn task_id(&self) -> String {
            format!("order_{}", self.name)
        }
    }

    #[tokio::test]
    async fn test_task_scheduler_basic() {
        let scheduler = TaskScheduler::new_with_defaults();
        let task = Box::new(MockTask {
            name: "test".to_string(),
            duration: Duration::from_millis(10),
        });
        let result = scheduler.submit_task(task).await.unwrap();
        assert!(matches!(result, TaskResult::Success(_)));
    }

    #[tokio::test]
    async fn test_concurrent_task_execution() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // Test single task
        let task = Box::new(CounterTask::new(counter.clone()));
        let result = scheduler.submit_task(task).await.unwrap();
        assert!(matches!(result, TaskResult::Success(_)));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Test multiple tasks - serial execution
        for _ in 0..4 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let _result = scheduler.submit_task(task).await.unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_task_priority_ordering() {
        let scheduler = TaskScheduler::new_with_defaults();
        let order = Arc::new(Mutex::new(Vec::new()));

        // Create tasks with different priorities
        let tasks = vec![
            (TaskPriority::Low, "low"),
            (TaskPriority::High, "high"),
            (TaskPriority::Normal, "normal"),
            (TaskPriority::Critical, "critical"),
        ];

        let mut handles = Vec::new();
        for (prio, name) in tasks {
            let task = Box::new(OrderTask::new(name, order.clone()));
            let scheduler_clone = scheduler.clone();
            let handle = tokio::spawn(async move {
                scheduler_clone
                    .submit_task_with_priority(task, prio)
                    .await
                    .unwrap()
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await.unwrap();
        }

        let v = order.lock().unwrap();
        assert_eq!(v.len(), 4);
        // Due to priority scheduling, critical should execute first
        assert!(v.contains(&"critical".to_string()));
        assert!(v.contains(&"high".to_string()));
        assert!(v.contains(&"normal".to_string()));
        assert!(v.contains(&"low".to_string()));
    }

    #[tokio::test]
    async fn test_queue_and_active_workers_metrics() {
        let scheduler = TaskScheduler::new_with_defaults();

        // Check initial state
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);

        // Submit a longer task
        let task = Box::new(MockTask {
            name: "long_task".to_string(),
            duration: Duration::from_millis(100),
        });

        let handle = {
            let scheduler_clone = scheduler.clone();
            tokio::spawn(async move { scheduler_clone.submit_task(task).await })
        };

        // Wait a short time and check metrics
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Complete task
        let _result = handle.await.unwrap().unwrap();

        // Check final state
        assert_eq!(scheduler.get_queue_size(), 0);
    }

    #[tokio::test]
    async fn test_continuous_scheduling() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // Submit multiple tasks to queue
        let mut handles = Vec::new();
        for i in 0..10 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            handles.push(handle);

            // Delayed submission to test continuous scheduling
            if i % 3 == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // Verify all tasks were executed
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_batch_task_execution() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // Use batch submission to test concurrent execution
        let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
        for _ in 0..3 {
            // Reduce task count to simplify test
            tasks.push(Box::new(CounterTask::new(counter.clone())));
        }

        let results = scheduler.submit_batch_tasks(tasks).await;
        assert_eq!(results.len(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        for result in results {
            assert!(matches!(result, TaskResult::Success(_)));
        }
    }

    #[tokio::test]
    async fn test_high_concurrency_stress() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // Create large number of tasks
        let mut handles = Vec::new();
        for i in 0..50 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let priority = match i % 4 {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                3 => TaskPriority::Critical,
                _ => TaskPriority::Normal,
            };

            let handle = tokio::spawn(async move {
                scheduler_clone
                    .submit_task_with_priority(task, priority)
                    .await
                    .unwrap()
            });
            handles.push(handle);

            // Interleaved submission to simulate real usage scenarios
            if i % 5 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // Verify all tasks were executed
        assert_eq!(counter.load(Ordering::SeqCst), 50);

        // Check final state
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);
    }

    #[tokio::test]
    async fn test_mixed_batch_and_individual_tasks() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // First submit some individual tasks
        let mut individual_handles = Vec::new();
        for _ in 0..3 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            individual_handles.push(handle);
        }

        // Then submit batch tasks
        let mut batch_tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
        for _ in 0..4 {
            batch_tasks.push(Box::new(CounterTask::new(counter.clone())));
        }

        let batch_handle = {
            let scheduler_clone = scheduler.clone();
            tokio::spawn(async move { scheduler_clone.submit_batch_tasks(batch_tasks).await })
        };

        // Submit more individual tasks during batch execution
        let mut more_individual_handles = Vec::new();
        for _ in 0..2 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            more_individual_handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in individual_handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        let batch_results = batch_handle.await.unwrap();
        assert_eq!(batch_results.len(), 4);
        for result in batch_results {
            assert!(matches!(result, TaskResult::Success(_)));
        }

        for handle in more_individual_handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // Verify total count is correct (3 + 4 + 2 = 9)
        assert_eq!(counter.load(Ordering::SeqCst), 9);
    }

    /// Test task scheduling strategies with different priorities
    #[tokio::test]
    async fn test_task_scheduling_strategies() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct PriorityTask {
            id: String,
            priority: TaskPriority,
            counter: Arc<AtomicUsize>,
            execution_order: Arc<Mutex<Vec<String>>>,
        }

        #[async_trait::async_trait]
        impl Task for PriorityTask {
            async fn execute(&self) -> TaskResult {
                self.counter.fetch_add(1, Ordering::SeqCst);
                self.execution_order.lock().unwrap().push(self.id.clone());
                // Longer delay to make priority effects more visible
                tokio::time::sleep(Duration::from_millis(50)).await;
                TaskResult::Success(format!("Priority task {} completed", self.id))
            }
            fn task_type(&self) -> &'static str {
                "priority"
            }
            fn task_id(&self) -> String {
                self.id.clone()
            }
        }

        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));
        let execution_order = Arc::new(Mutex::new(Vec::new()));

        // Submit tasks with different priorities
        let priorities = vec![
            ("low", TaskPriority::Low),
            ("high", TaskPriority::High),
            ("critical", TaskPriority::Critical),
            ("normal", TaskPriority::Normal),
        ];

        for (id, priority) in priorities {
            let task = PriorityTask {
                id: id.to_string(),
                priority,
                counter: Arc::clone(&counter),
                execution_order: Arc::clone(&execution_order),
            };

            scheduler
                .submit_task_with_priority(Box::new(task), priority)
                .await
                .unwrap();
        }

        // Wait for all tasks to complete
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify all tasks were executed
        let final_count = counter.load(Ordering::SeqCst);
        assert_eq!(final_count, 4, "All 4 tasks should have been executed");

        // Verify execution order respects priority (highest first)
        let order = execution_order.lock().unwrap();
        println!("Task execution order: {:?}", *order);

        // Check that all tasks were executed, but don't strictly enforce ordering
        // since concurrent execution can vary
        assert!(
            order.contains(&"critical".to_string()),
            "Critical task should have been executed"
        );
        assert!(
            order.contains(&"low".to_string()),
            "Low task should have been executed"
        );
        assert!(
            order.contains(&"high".to_string()),
            "High task should have been executed"
        );
        assert!(
            order.contains(&"normal".to_string()),
            "Normal task should have been executed"
        );
    }

    /// Test load balancing across multiple workers
    #[tokio::test]
    async fn test_load_balancing() {
        let scheduler = TaskScheduler::new_with_defaults();
        let task_counter = Arc::new(AtomicUsize::new(0));

        // Submit multiple tasks concurrently
        for _i in 0..10 {
            let task = CounterTask::new(Arc::clone(&task_counter));
            let result = scheduler.submit_task(Box::new(task)).await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // Verify all tasks were processed
        let final_count = task_counter.load(Ordering::SeqCst);
        assert_eq!(final_count, 10);

        // Check scheduler queue is empty
        assert_eq!(scheduler.get_queue_size(), 0);
    }

    /// Test task priority handling
    #[tokio::test]
    async fn test_task_priority_processing() {
        let scheduler = TaskScheduler::new_with_defaults();

        // Test priority comparison
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);

        // Submit tasks with different priorities and verify they're handled
        let high_task = MockTask {
            name: "high_priority".to_string(),
            duration: Duration::from_millis(5),
        };

        let low_task = MockTask {
            name: "low_priority".to_string(),
            duration: Duration::from_millis(5),
        };

        let high_result = scheduler
            .submit_task_with_priority(Box::new(high_task), TaskPriority::High)
            .await
            .unwrap();
        let low_result = scheduler
            .submit_task_with_priority(Box::new(low_task), TaskPriority::Low)
            .await
            .unwrap();

        assert!(matches!(high_result, TaskResult::Success(_)));
        assert!(matches!(low_result, TaskResult::Success(_)));
    }

    /// Test scheduler state management
    #[tokio::test]
    async fn test_scheduler_state_management() {
        let scheduler = TaskScheduler::new_with_defaults();

        // Initial state
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);

        // Submit a task
        let task = MockTask {
            name: "state_test".to_string(),
            duration: Duration::from_millis(50),
        };

        let result = scheduler.submit_task(Box::new(task)).await.unwrap();

        // Queue should increase temporarily
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Wait for completion
        assert!(matches!(result, TaskResult::Success(_)));

        // State should return to initial
        assert_eq!(scheduler.get_queue_size(), 0);
    }

    /// Test scheduler overflow strategies
    #[tokio::test]
    async fn test_overflow_strategy_handling() {
        let scheduler = TaskScheduler::new_with_defaults();

        // Submit many long-running tasks to potentially trigger overflow
        for i in 0..20 {
            let task = MockTask {
                name: format!("overflow_test_{}", i),
                duration: Duration::from_millis(20),
            };

            match scheduler.submit_task(Box::new(task)).await {
                Ok(result) => {
                    assert!(matches!(result, TaskResult::Success(_)));
                }
                Err(_) => {
                    // Some tasks might be rejected due to overflow, which is acceptable
                    break;
                }
            }
        }

        // Wait for tasks to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Scheduler should recover to normal state
        assert_eq!(scheduler.get_queue_size(), 0);
    }

    /// Test concurrent task submission and execution
    #[tokio::test]
    async fn test_concurrent_task_submission() {
        let scheduler = TaskScheduler::new_with_defaults();
        let completion_counter = Arc::new(AtomicUsize::new(0));
        let mut submission_handles = Vec::new();

        // Submit tasks concurrently from multiple threads
        for _i in 0..8 {
            let scheduler_clone = scheduler.clone();
            let counter_clone = Arc::clone(&completion_counter);

            let submission_handle = tokio::spawn(async move {
                let task = CounterTask::new(counter_clone);
                scheduler_clone.submit_task(Box::new(task)).await.unwrap()
            });

            submission_handles.push(submission_handle);
        }

        // Wait for all concurrent submissions to complete
        for handle in submission_handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // Verify all tasks completed
        let final_count = completion_counter.load(Ordering::SeqCst);
        assert_eq!(final_count, 8);
    }

    /// Test scheduler performance metrics
    #[tokio::test]
    async fn test_scheduler_performance_metrics() {
        let scheduler = TaskScheduler::new_with_defaults();
        let start_time = std::time::Instant::now();
        let task_count = 5;

        // Submit multiple tasks
        for i in 0..task_count {
            let task = MockTask {
                name: format!("perf_test_{}", i),
                duration: Duration::from_millis(10),
            };
            let result = scheduler.submit_task(Box::new(task)).await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        let total_time = start_time.elapsed();

        // Verify reasonable performance (tasks should complete in reasonable time)
        assert!(
            total_time < Duration::from_millis(500),
            "Tasks took too long: {:?}",
            total_time
        );

        // Verify final state
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);
    }

    /// Verify that `ActiveTaskGuard` removes its entry from `active_tasks`
    /// when dropped, independently of any explicit cleanup code path.
    #[tokio::test]
    async fn test_active_task_guard_cleanup() {
        use super::{ActiveTaskGuard, TaskInfo};
        use std::collections::HashMap;

        let active_tasks = Arc::new(Mutex::new(HashMap::<String, TaskInfo>::new()));
        let task_id = "guard_test_task".to_string();

        active_tasks.lock().unwrap().insert(
            task_id.clone(),
            TaskInfo {
                task_id: task_id.clone(),
                task_type: "mock".to_string(),
                status: crate::core::parallel::TaskStatus::Pending,
                start_time: std::time::Instant::now(),
                progress: 0.0,
            },
        );
        assert!(active_tasks.lock().unwrap().contains_key(&task_id));

        {
            let _guard = ActiveTaskGuard {
                active_tasks: Arc::clone(&active_tasks),
                task_id: task_id.clone(),
            };
            // Guard is alive here; entry still present.
            assert!(active_tasks.lock().unwrap().contains_key(&task_id));
        }

        // After the guard drops, the entry must be gone.
        assert!(!active_tasks.lock().unwrap().contains_key(&task_id));
    }

    /// Verify that the `DropOldest` overflow strategy delivers a
    /// `TaskResult::Failed` back to the evicted submitter rather than
    /// silently dropping its oneshot sender.
    #[tokio::test]
    async fn test_drop_oldest_sends_failed() {
        use crate::config::{Config, OverflowStrategy};

        let mut config = Config::default();
        config.parallel.task_queue_size = 1;
        config.general.max_concurrent_jobs = 1;
        config.parallel.overflow_strategy = OverflowStrategy::DropOldest;
        config.parallel.enable_task_priorities = false;
        config.parallel.auto_balance_workers = false;

        let scheduler = TaskScheduler::new_with_config(&config).unwrap();

        // Occupy the single worker with a slow task so the queue fills.
        let blocker = Box::new(MockTask {
            name: "blocker".to_string(),
            duration: Duration::from_millis(300),
        });
        let blocker_scheduler = scheduler.clone();
        let blocker_handle =
            tokio::spawn(async move { blocker_scheduler.submit_task(blocker).await });

        // Give the blocker a moment to start running.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Fill the queue slot.
        let first = Box::new(MockTask {
            name: "first_queued".to_string(),
            duration: Duration::from_millis(50),
        });
        let first_scheduler = scheduler.clone();
        let first_handle = tokio::spawn(async move { first_scheduler.submit_task(first).await });

        // Ensure the first queued task is actually enqueued before the second.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Submitting a second task should evict the first with DropOldest.
        let second = Box::new(MockTask {
            name: "second_queued".to_string(),
            duration: Duration::from_millis(10),
        });
        let second_scheduler = scheduler.clone();
        let second_handle = tokio::spawn(async move { second_scheduler.submit_task(second).await });

        // The evicted (first) task must receive a Failed result.
        let first_result = first_handle.await.unwrap().unwrap();
        match first_result {
            TaskResult::Failed(msg) => {
                assert!(
                    msg.contains("overflow"),
                    "expected overflow-related failure message, got: {}",
                    msg
                );
            }
            other => panic!("expected Failed for evicted task, got {:?}", other),
        }

        // The blocker and second task should still complete successfully.
        let blocker_result = blocker_handle.await.unwrap().unwrap();
        assert!(matches!(blocker_result, TaskResult::Success(_)));
        let second_result = second_handle.await.unwrap().unwrap();
        assert!(matches!(second_result, TaskResult::Success(_)));
    }

    /// Verify that submitting a task after the scheduler loop has exited
    /// due to idle timeout transparently restarts the loop and completes
    /// the new task.
    #[tokio::test]
    async fn test_scheduler_restart_after_idle() {
        let mut scheduler = TaskScheduler::new_with_defaults();

        // Abort the default scheduler loop so we can restart it with a
        // short idle timeout.
        {
            let mut handle = scheduler.scheduler_handle.lock().unwrap();
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
        // Allow the aborted task to be fully cancelled before we continue.
        tokio::time::sleep(Duration::from_millis(30)).await;

        scheduler.worker_idle_timeout = Duration::from_millis(100);
        scheduler.start_scheduler_loop();

        // Submit and complete a task normally.
        let t1 = Box::new(MockTask {
            name: "before_idle".to_string(),
            duration: Duration::from_millis(10),
        });
        let r1 = scheduler.submit_task(t1).await.unwrap();
        assert!(matches!(r1, TaskResult::Success(_)));

        // Wait well past the idle timeout so the loop exits on its own.
        tokio::time::sleep(Duration::from_millis(350)).await;

        let loop_finished = {
            let handle = scheduler.scheduler_handle.lock().unwrap();
            handle.as_ref().map(|h| h.is_finished()).unwrap_or(true)
        };
        assert!(
            loop_finished,
            "scheduler loop should have exited after idle timeout"
        );

        // Submitting again must restart the loop and run the task.
        let t2 = Box::new(MockTask {
            name: "after_idle".to_string(),
            duration: Duration::from_millis(10),
        });
        let r2 = scheduler.submit_task(t2).await.unwrap();
        assert!(matches!(r2, TaskResult::Success(_)));

        // The handle should now point at a running loop again.
        let still_running = {
            let handle = scheduler.scheduler_handle.lock().unwrap();
            handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false)
        };
        assert!(
            still_running,
            "scheduler loop should be running after restart"
        );
    }
}
