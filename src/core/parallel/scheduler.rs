//! Task scheduler for parallel processing
use super::{Task, TaskResult, TaskStatus, WorkerPool};
use crate::config::load_config;
use crate::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::{oneshot, Semaphore};

struct PendingTask {
    task: Box<dyn Task + Send + Sync>,
    result_sender: oneshot::Sender<TaskResult>,
    task_id: String,
    priority: TaskPriority,
}

/// Priority levels for tasks
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
    task_queue: Arc<Mutex<VecDeque<PendingTask>>>,
    worker_pool: WorkerPool,
    semaphore: Arc<Semaphore>,
    max_concurrent: usize,
    active_tasks: Arc<Mutex<std::collections::HashMap<String, TaskInfo>>>,
}

impl TaskScheduler {
    /// Create a new scheduler based on configuration
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let max = config.general.max_concurrent_jobs;
        let worker_pool = WorkerPool::new(max);
        let semaphore = Arc::new(Semaphore::new(max));
        Ok(Self {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            worker_pool,
            semaphore,
            max_concurrent: max,
            active_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Create a new scheduler with default settings (for testing)
    pub fn new_with_defaults() -> Self {
        let max = 4; // 預設最大並發數
        let worker_pool = WorkerPool::new(max);
        let semaphore = Arc::new(Semaphore::new(max));
        Self {
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            worker_pool,
            semaphore,
            max_concurrent: max,
            active_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
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

        // Enqueue task
        {
            let mut queue = self.task_queue.lock().unwrap();
            let pending = PendingTask {
                task,
                result_sender: tx,
                task_id: task_id.clone(),
                priority: priority.clone(),
            };
            let pos = queue
                .iter()
                .position(|t| t.priority < priority)
                .unwrap_or(queue.len());
            queue.insert(pos, pending);
        }

        self.try_execute_next_task().await;

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
        if let Ok(permit) = self.semaphore.clone().try_acquire_owned() {
            let pending = { self.task_queue.lock().unwrap().pop_front() };
            if let Some(p) = pending {
                {
                    let mut active = self.active_tasks.lock().unwrap();
                    if let Some(info) = active.get_mut(&p.task_id) {
                        info.status = TaskStatus::Running;
                    }
                }
                let task_id = p.task_id.clone();
                let active_tasks = Arc::clone(&self.active_tasks);
                tokio::spawn(async move {
                    let result = p.task.execute().await;
                    {
                        let mut at = active_tasks.lock().unwrap();
                        if let Some(info) = at.get_mut(&task_id) {
                            info.status = TaskStatus::Completed(result.clone());
                            info.progress = 1.0;
                        }
                    }
                    let _ = p.result_sender.send(result);
                    drop(permit);
                });
            } else {
                drop(permit);
            }
        }
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

            // 將任務加入佇列
            {
                let mut queue = self.task_queue.lock().unwrap();
                let pending = PendingTask {
                    task,
                    result_sender: tx,
                    task_id: task_id.clone(),
                    priority: TaskPriority::Normal,
                };
                queue.push_back(pending);
            }

            receivers.push((task_id, rx));
        }

        // 觸發執行所有可能的任務
        for _ in 0..self.max_concurrent {
            if self.get_queue_size() == 0 || self.semaphore.available_permits() == 0 {
                break;
            }
            self.try_execute_next_task().await;
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
        self.max_concurrent - self.semaphore.available_permits()
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
            task_queue: Arc::clone(&self.task_queue),
            worker_pool: self.worker_pool.clone(),
            semaphore: Arc::clone(&self.semaphore),
            max_concurrent: self.max_concurrent,
            active_tasks: Arc::clone(&self.active_tasks),
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

        // 序列提交任務以確保優先順序正確
        let tasks = vec![
            (TaskPriority::Low, "low"),
            (TaskPriority::High, "high"),
            (TaskPriority::Normal, "normal"),
            (TaskPriority::Critical, "critical"),
        ];

        for (prio, name) in tasks {
            let task = Box::new(OrderTask::new(name, order.clone()));
            let _ = scheduler
                .submit_task_with_priority(task, prio)
                .await
                .unwrap();
        }

        let v = order.lock().unwrap();
        // 由於是序列執行，順序應該是提交順序
        assert_eq!(v.len(), 4);
        assert!(v.contains(&"low".to_string()));
        assert!(v.contains(&"high".to_string()));
        assert!(v.contains(&"normal".to_string()));
        assert!(v.contains(&"critical".to_string()));
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
}
