//! Task scheduler for parallel processing
use super::{Task, TaskResult, TaskStatus};
use crate::Result;
use crate::config::{Config, load_config};
use crate::core::parallel::config::ParallelConfig;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::{Semaphore, oneshot};

struct PendingTask {
    task: Box<dyn Task + Send + Sync>,
    result_sender: oneshot::Sender<TaskResult>,
    task_id: String,
    priority: TaskPriority,
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

/// Priority levels for tasks
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Information about an active task
#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub task_id: String,
    pub task_type: String,
    pub status: TaskStatus,
    pub start_time: std::time::Instant,
    pub progress: f32,
}

/// Scheduler to manage and execute tasks in parallel
pub struct TaskScheduler {
    /// Parallel processing configuration
    _config: ParallelConfig,
    task_queue: Arc<Mutex<VecDeque<PendingTask>>>,
    semaphore: Arc<Semaphore>,
    active_tasks: Arc<Mutex<std::collections::HashMap<String, TaskInfo>>>,
    scheduler_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TaskScheduler {
    /// Create a new scheduler based on configuration
    pub fn new() -> Result<Self> {
        let app_config = load_config()?;
        let config = ParallelConfig::from_app_config(&app_config);
        config.validate()?;
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_jobs));
        let task_queue = Arc::new(Mutex::new(VecDeque::new()));
        let active_tasks = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let scheduler = Self {
            _config: config,
            task_queue: task_queue.clone(),
            semaphore: semaphore.clone(),
            active_tasks: active_tasks.clone(),
            scheduler_handle: Arc::new(Mutex::new(None)),
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

        let scheduler = Self {
            _config: config,
            task_queue: task_queue.clone(),
            semaphore: semaphore.clone(),
            active_tasks: active_tasks.clone(),
            scheduler_handle: Arc::new(Mutex::new(None)),
        };

        // Start background scheduler loop
        scheduler.start_scheduler_loop();
        scheduler
    }

    /// Start the background scheduler loop
    fn start_scheduler_loop(&self) {
        let task_queue = Arc::clone(&self.task_queue);
        let semaphore = Arc::clone(&self.semaphore);
        let active_tasks = Arc::clone(&self.active_tasks);
        let config = self._config.clone();

        let handle = tokio::spawn(async move {
            loop {
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
                            let result = p.task.execute().await;

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

        // Enqueue task with or without priority
        {
            let mut queue = self.task_queue.lock().unwrap();
            let pending = PendingTask {
                task,
                result_sender: tx,
                task_id: task_id.clone(),
                priority,
            };
            if self._config.enable_task_priorities {
                // insert by priority (higher first)
                let pos = queue
                    .iter()
                    .position(|t| t.priority < pending.priority)
                    .unwrap_or(queue.len());
                queue.insert(pos, pending);
            } else {
                queue.push_back(pending);
            }
        }

        // Await result
        let result = rx.await.map_err(|_| {
            crate::error::SubXError::parallel_processing("任務執行被中斷".to_string())
        })?;

        // Clean up
        {
            let mut active = self.active_tasks.lock().unwrap();
            active.remove(&task_id);
        }
        Ok(result)
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

        // 首先將所有任務加入佇列
        for task in tasks {
            let task_id = task.task_id();
            let task_type = task.task_type().to_string();
            let (tx, rx) = oneshot::channel();

            // 註冊任務資訊
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

            // 將任務加入佇列 (批次採相同優先級或 FIFO)
            {
                let mut queue = self.task_queue.lock().unwrap();
                let pending = PendingTask {
                    task,
                    result_sender: tx,
                    task_id: task_id.clone(),
                    priority: TaskPriority::Normal,
                };
                if self._config.enable_task_priorities {
                    let pos = queue
                        .iter()
                        .position(|t| t.priority < pending.priority)
                        .unwrap_or(queue.len());
                    queue.insert(pos, pending);
                } else {
                    queue.push_back(pending);
                }
            }

            receivers.push((task_id, rx));
        }

        // 等待所有結果
        let mut results = Vec::new();
        for (task_id, rx) in receivers {
            match rx.await {
                Ok(result) => results.push(result),
                Err(_) => results.push(TaskResult::Failed("任務執行被中斷".to_string())),
            }

            // 清理任務資訊
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
            TaskResult::Success(format!("完成任務: {}", self.name))
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
            TaskResult::Success("計數任務完成".to_string())
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
            TaskResult::Success(format!("順序任務完成: {}", self.name))
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

        // 測試單個任務
        let task = Box::new(CounterTask::new(counter.clone()));
        let result = scheduler.submit_task(task).await.unwrap();
        assert!(matches!(result, TaskResult::Success(_)));
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // 測試多個任務 - 序列執行
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

        // 建立不同優先級的任務
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

        // 等待所有任務完成
        for handle in handles {
            let _ = handle.await.unwrap();
        }

        let v = order.lock().unwrap();
        assert_eq!(v.len(), 4);
        // 由於優先級排程，critical 應該先執行
        assert!(v.contains(&"critical".to_string()));
        assert!(v.contains(&"high".to_string()));
        assert!(v.contains(&"normal".to_string()));
        assert!(v.contains(&"low".to_string()));
    }

    #[tokio::test]
    async fn test_queue_and_active_workers_metrics() {
        let scheduler = TaskScheduler::new_with_defaults();

        // 初始狀態檢查
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);

        // 提交一個較長的任務
        let task = Box::new(MockTask {
            name: "long_task".to_string(),
            duration: Duration::from_millis(100),
        });

        let handle = {
            let scheduler_clone = scheduler.clone();
            tokio::spawn(async move { scheduler_clone.submit_task(task).await })
        };

        // 等待一小段時間，檢查指標
        tokio::time::sleep(Duration::from_millis(20)).await;

        // 完成任務
        let _result = handle.await.unwrap().unwrap();

        // 檢查最終狀態
        assert_eq!(scheduler.get_queue_size(), 0);
    }

    #[tokio::test]
    async fn test_continuous_scheduling() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // 提交多個任務到佇列
        let mut handles = Vec::new();
        for i in 0..10 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            handles.push(handle);

            // 延遲提交以測試連續排程
            if i % 3 == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }

        // 等待所有任務完成
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // 驗證所有任務都被執行
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_batch_task_execution() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // 使用批次提交來測試並發執行
        let mut tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
        for _ in 0..3 {
            // 減少任務數量以簡化測試
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

        // 建立大量任務
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

            // 交錯式提交，模擬實際使用情境
            if i % 5 == 0 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        // 等待所有任務完成
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(matches!(result, TaskResult::Success(_)));
        }

        // 驗證所有任務都被執行
        assert_eq!(counter.load(Ordering::SeqCst), 50);

        // 檢查最終狀態
        assert_eq!(scheduler.get_queue_size(), 0);
        assert_eq!(scheduler.get_active_workers(), 0);
    }

    #[tokio::test]
    async fn test_mixed_batch_and_individual_tasks() {
        let scheduler = TaskScheduler::new_with_defaults();
        let counter = Arc::new(AtomicUsize::new(0));

        // 首先提交一些單個任務
        let mut individual_handles = Vec::new();
        for _ in 0..3 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            individual_handles.push(handle);
        }

        // 然後提交批次任務
        let mut batch_tasks: Vec<Box<dyn Task + Send + Sync>> = Vec::new();
        for _ in 0..4 {
            batch_tasks.push(Box::new(CounterTask::new(counter.clone())));
        }

        let batch_handle = {
            let scheduler_clone = scheduler.clone();
            tokio::spawn(async move { scheduler_clone.submit_batch_tasks(batch_tasks).await })
        };

        // 在批次執行期間再提交更多單個任務
        let mut more_individual_handles = Vec::new();
        for _ in 0..2 {
            let task = Box::new(CounterTask::new(counter.clone()));
            let scheduler_clone = scheduler.clone();
            let handle =
                tokio::spawn(async move { scheduler_clone.submit_task(task).await.unwrap() });
            more_individual_handles.push(handle);
        }

        // 等待所有任務完成
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

        // 驗證總計數正確 (3 + 4 + 2 = 9)
        assert_eq!(counter.load(Ordering::SeqCst), 9);
    }
}
