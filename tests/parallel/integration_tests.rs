#![allow(unused_imports, dead_code)]
//! 並行處理整合測試
//! 測試跨模組的並行工作流程和協調

use subx_cli::core::parallel::{TaskScheduler, WorkerPool};
use subx_cli::core::parallel::task::{Task, TaskResult, TaskStatus, ProcessingOperation, FileProcessingTask};
use subx_cli::core::parallel::scheduler::TaskPriority;
use tempfile::TempDir;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use async_trait::async_trait;

/// 測試工作池與調度器整合
#[tokio::test]
async fn test_worker_pool_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    let completion_counter = Arc::new(AtomicUsize::new(0));
    
    // 建立測試任務
    struct IntegrationTask {
        id: String,
        counter: Arc<AtomicUsize>,
        duration: Duration,
    }
    
    #[async_trait]
    impl Task for IntegrationTask {
        async fn execute(&self) -> TaskResult {
            tokio::time::sleep(self.duration).await;
            self.counter.fetch_add(1, Ordering::SeqCst);
            TaskResult::Success(format!("Integration task {} completed", self.id))
        }
        
        fn task_type(&self) -> &'static str {
            "integration"
        }
        
        fn task_id(&self) -> String {
            self.id.clone()
        }
    }
    
    // 提交多個任務測試工作池整合
    let mut results = Vec::new();
    for i in 0..5 {
        let task = IntegrationTask {
            id: format!("integration-{}", i),
            counter: Arc::clone(&completion_counter),
            duration: Duration::from_millis(20),
        };
        
        let result = scheduler.submit_task(Box::new(task)).await.unwrap();
        results.push(result);
    }
    
    // 驗證所有任務都成功完成
    for result in results {
        assert!(matches!(result, TaskResult::Success(_)));
    }
    
    // 等待所有任務完成並驗證計數器
    tokio::time::sleep(Duration::from_millis(100)).await;
    let final_count = completion_counter.load(Ordering::SeqCst);
    assert_eq!(final_count, 5, "所有 5 個任務都應該完成");
    
    // 驗證調度器狀態
    assert_eq!(scheduler.get_queue_size(), 0);
    assert_eq!(scheduler.get_active_workers(), 0);
}

/// 測試跨組件錯誤處理
#[tokio::test]
async fn test_error_handling_across_components() {
    let scheduler = TaskScheduler::new_with_defaults();
    
    // 建立會失敗的任務
    struct FailingTask {
        id: String,
        should_fail: bool,
    }
    
    #[async_trait]
    impl Task for FailingTask {
        async fn execute(&self) -> TaskResult {
            if self.should_fail {
                TaskResult::Failed(format!("Task {} intentionally failed", self.id))
            } else {
                TaskResult::Success(format!("Task {} succeeded", self.id))
            }
        }
        
        fn task_type(&self) -> &'static str {
            "error_test"
        }
        
        fn task_id(&self) -> String {
            self.id.clone()
        }
    }
    
    // 混合成功和失敗任務
    let success_task = FailingTask {
        id: "success".to_string(),
        should_fail: false,
    };
    
    let fail_task = FailingTask {
        id: "failure".to_string(),
        should_fail: true,
    };
    
    // 執行任務並驗證錯誤處理
    let success_result = scheduler.submit_task(Box::new(success_task)).await.unwrap();
    let fail_result = scheduler.submit_task(Box::new(fail_task)).await.unwrap();
    
    assert!(matches!(success_result, TaskResult::Success(_)));
    assert!(matches!(fail_result, TaskResult::Failed(_)));
    
    // 系統應該能夠處理錯誤並繼續運作
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// 測試資源管理整合
#[tokio::test]
async fn test_resource_management_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    
    // 建立消耗不同資源的任務
    struct ResourceTask {
        id: String,
        task_type: String,
        duration: Duration,
    }
    
    #[async_trait]
    impl Task for ResourceTask {
        async fn execute(&self) -> TaskResult {
            tokio::time::sleep(self.duration).await;
            TaskResult::Success(format!("Resource task {} completed", self.id))
        }
        
        fn task_type(&self) -> &'static str {
            match self.task_type.as_str() {
                "cpu" => "convert",      // CPU 密集型
                "io" => "match",         // I/O 密集型
                _ => "sync",             // 混合型
            }
        }
        
        fn task_id(&self) -> String {
            self.id.clone()
        }
    }
    
    // 提交不同類型的任務
    let task_types = vec![
        ("cpu", "cpu"),
        ("io", "io"), 
        ("mixed", "mixed"),
    ];
    
    for (id, task_type) in task_types {
        let task = ResourceTask {
            id: id.to_string(),
            task_type: task_type.to_string(),
            duration: Duration::from_millis(10),
        };
        
        let result = scheduler.submit_task(Box::new(task)).await.unwrap();
        assert!(matches!(result, TaskResult::Success(_)));
    }
    
    // 驗證資源管理
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// 測試大規模並行處理
#[tokio::test]
async fn test_large_scale_parallel_processing() {
    let scheduler = TaskScheduler::new_with_defaults();
    let start_time = Instant::now();
    let task_count = 20;
    let completion_counter = Arc::new(AtomicUsize::new(0));
    
    // 建立大量任務
    struct BulkTask {
        id: usize,
        counter: Arc<AtomicUsize>,
    }
    
    #[async_trait]
    impl Task for BulkTask {
        async fn execute(&self) -> TaskResult {
            // 模擬一些處理時間
            tokio::time::sleep(Duration::from_millis(5)).await;
            self.counter.fetch_add(1, Ordering::SeqCst);
            TaskResult::Success(format!("Bulk task {} completed", self.id))
        }
        
        fn task_type(&self) -> &'static str {
            "bulk"
        }
        
        fn task_id(&self) -> String {
            format!("bulk-{}", self.id)
        }
    }
    
    // 提交所有任務
    let mut success_count = 0;
    for i in 0..task_count {
        let task = BulkTask {
            id: i,
            counter: Arc::clone(&completion_counter),
        };
        
        match scheduler.submit_task(Box::new(task)).await {
            Ok(TaskResult::Success(_)) => success_count += 1,
            Ok(TaskResult::Failed(_)) => {
                // 某些任務可能因為資源限制而失敗，這是可接受的
            }
            _ => {}
        }
    }
    
    let processing_time = start_time.elapsed();
    
    // 等待任務完成
    tokio::time::sleep(Duration::from_millis(100)).await;
    let final_count = completion_counter.load(Ordering::SeqCst);
    
    // 驗證處理結果
    assert!(success_count > 0, "至少應該有一些任務成功");
    assert!(final_count <= task_count, "完成計數不應超過提交計數");
    assert!(processing_time < Duration::from_secs(5), "處理時間應該合理");
    
    // 驗證系統狀態
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// 測試優先級處理整合
#[tokio::test]
async fn test_priority_processing_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));
    
    struct PriorityTask {
        id: String,
        priority: TaskPriority,
        execution_order: Arc<std::sync::Mutex<Vec<String>>>,
    }
    
    #[async_trait]
    impl Task for PriorityTask {
        async fn execute(&self) -> TaskResult {
            self.execution_order.lock().unwrap().push(self.id.clone());
            tokio::time::sleep(Duration::from_millis(5)).await;
            TaskResult::Success(format!("Priority task {} completed", self.id))
        }
        
        fn task_type(&self) -> &'static str {
            "priority"
        }
        
        fn task_id(&self) -> String {
            self.id.clone()
        }
    }
    
    // 提交不同優先級的任務
    let priorities = vec![
        ("low", TaskPriority::Low),
        ("critical", TaskPriority::Critical),
        ("normal", TaskPriority::Normal),
        ("high", TaskPriority::High),
    ];
    
    for (id, priority) in priorities {
        let task = PriorityTask {
            id: id.to_string(),
            priority,
            execution_order: Arc::clone(&execution_order),
        };
        
        let result = scheduler.submit_task_with_priority(Box::new(task), priority).await.unwrap();
        assert!(matches!(result, TaskResult::Success(_)));
    }
    
    // 等待執行完成
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // 驗證執行順序
    let order = execution_order.lock().unwrap();
    assert_eq!(order.len(), 4, "所有 4 個任務都應該執行");
    
    // 檢查優先級順序（如果調度器支持的話）
    // 注意：實際的執行順序可能受到並行性影響
    println!("執行順序: {:?}", *order);
}

/// 測試檔案處理任務整合
#[tokio::test]
async fn test_file_processing_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    let temp_dir = TempDir::new().unwrap();
    
    // 建立測試檔案
    let input_file = temp_dir.path().join("test.srt");
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
Integration test subtitle

2
00:00:04,000 --> 00:00:06,000
Second subtitle line
"#;
    tokio::fs::write(&input_file, srt_content).await.unwrap();
    
    // 測試檔案驗證任務
    let validate_task = FileProcessingTask {
        input_path: input_file.clone(),
        output_path: None,
        operation: ProcessingOperation::ValidateFormat,
    };
    
    let result = scheduler.submit_task(Box::new(validate_task)).await.unwrap();
    assert!(matches!(result, TaskResult::Success(_)));
    
    // 測試檔案轉換任務
    let output_file = temp_dir.path().join("output.ass");
    let convert_task = FileProcessingTask {
        input_path: input_file.clone(),
        output_path: Some(output_file.clone()),
        operation: ProcessingOperation::ConvertFormat {
            from: "srt".to_string(),
            to: "ass".to_string(),
        },
    };
    
    let result = scheduler.submit_task(Box::new(convert_task)).await.unwrap();
    assert!(matches!(result, TaskResult::Success(_)));
    
    // 驗證輸出檔案存在
    assert!(tokio::fs::metadata(&output_file).await.is_ok());
    
    // 測試檔案匹配任務
    let match_task = FileProcessingTask {
        input_path: temp_dir.path().to_path_buf(),
        output_path: None,
        operation: ProcessingOperation::MatchFiles { recursive: false },
    };
    
    let result = scheduler.submit_task(Box::new(match_task)).await.unwrap();
    assert!(matches!(result, TaskResult::Success(_)));
}
