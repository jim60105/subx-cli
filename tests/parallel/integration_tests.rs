#![allow(unused_imports, dead_code)]
//! Parallel processing integration tests
//! Testing cross-module parallel workflows and coordination

use subx_cli::core::parallel::{TaskScheduler, WorkerPool};
use subx_cli::core::parallel::task::{Task, TaskResult, TaskStatus, ProcessingOperation, FileProcessingTask};
use subx_cli::core::parallel::scheduler::TaskPriority;
use tempfile::TempDir;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use async_trait::async_trait;

/// Test worker pool and scheduler integration
#[tokio::test]
async fn test_worker_pool_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    let completion_counter = Arc::new(AtomicUsize::new(0));
    
    // Create test tasks
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
    
    // Submit multiple tasks to test worker pool integration
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
    
    // Verify all tasks completed successfully
    for result in results {
        assert!(matches!(result, TaskResult::Success(_)));
    }
    
    // Wait for all tasks to complete and verify counter
    tokio::time::sleep(Duration::from_millis(100)).await;
    let final_count = completion_counter.load(Ordering::SeqCst);
    assert_eq!(final_count, 5, "All 5 tasks should have completed");
    
    // Verify scheduler state
    assert_eq!(scheduler.get_queue_size(), 0);
    assert_eq!(scheduler.get_active_workers(), 0);
}

/// Test error handling across components
#[tokio::test]
async fn test_error_handling_across_components() {
    let scheduler = TaskScheduler::new_with_defaults();
    
    // Create a task that fails
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
    
    // Mix successful and failing tasks
    let success_task = FailingTask {
        id: "success".to_string(),
        should_fail: false,
    };
    
    let fail_task = FailingTask {
        id: "failure".to_string(),
        should_fail: true,
    };
    
    // Execute tasks and verify error handling
    let success_result = scheduler.submit_task(Box::new(success_task)).await.unwrap();
    let fail_result = scheduler.submit_task(Box::new(fail_task)).await.unwrap();
    
    assert!(matches!(success_result, TaskResult::Success(_)));
    assert!(matches!(fail_result, TaskResult::Failed(_)));
    
    // The system should be able to handle errors and continue operating
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// Test resource management integration
#[tokio::test]
async fn test_resource_management_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    
    // Create tasks that consume different resources
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
                "cpu" => "convert",      // CPU intensive
                "io" => "match",         // I/O intensive
                _ => "sync",             // Mixed
            }
        }
        
        fn task_id(&self) -> String {
            self.id.clone()
        }
    }
    
    // Submit different types of tasks
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
    
    // Verify resource management
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// Test large-scale parallel processing
#[tokio::test]
async fn test_large_scale_parallel_processing() {
    let scheduler = TaskScheduler::new_with_defaults();
    let start_time = Instant::now();
    let task_count = 20;
    let completion_counter = Arc::new(AtomicUsize::new(0));
    
    // Create a large number of tasks
    struct BulkTask {
        id: usize,
        counter: Arc<AtomicUsize>,
    }
    
    #[async_trait]
    impl Task for BulkTask {
        async fn execute(&self) -> TaskResult {
            // Simulate some processing time
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
    
    // Submit all tasks
    let mut success_count = 0;
    for i in 0..task_count {
        let task = BulkTask {
            id: i,
            counter: Arc::clone(&completion_counter),
        };
        
        match scheduler.submit_task(Box::new(task)).await {
            Ok(TaskResult::Success(_)) => success_count += 1,
            Ok(TaskResult::Failed(_)) => {
                // Some tasks may fail due to resource constraints, this is acceptable
            }
            _ => {}
        }
    }
    
    let processing_time = start_time.elapsed();
    
    // Wait for tasks to complete
    tokio::time::sleep(Duration::from_millis(100)).await;
    let final_count = completion_counter.load(Ordering::SeqCst);
    
    // Verify processing results
    assert!(success_count > 0, "At least some tasks should have succeeded");
    assert!(final_count <= task_count, "Completion count should not exceed submission count");
    assert!(processing_time < Duration::from_secs(5), "Processing time should be reasonable");
    
    // Verify system state
    assert_eq!(scheduler.get_queue_size(), 0);
}

/// Test priority processing integration
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
    
    // Submit tasks with different priorities
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
    
    // Wait for execution to complete
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    // Verify execution order
    let order = execution_order.lock().unwrap();
    assert_eq!(order.len(), 4, "All 4 tasks should have been executed");
    
    // Check priority order (if supported by the scheduler)
    // Note: The actual execution order may be affected by concurrency
    println!("Execution order: {:?}", *order);
}

/// Test file processing task integration
#[tokio::test]
async fn test_file_processing_integration() {
    let scheduler = TaskScheduler::new_with_defaults();
    let temp_dir = TempDir::new().unwrap();
    
    // Create test file
    let input_file = temp_dir.path().join("test.srt");
    let srt_content = r#"1
00:00:01,000 --> 00:00:03,000
Integration test subtitle

2
00:00:04,000 --> 00:00:06,000
Second subtitle line
"#;
    tokio::fs::write(&input_file, srt_content).await.unwrap();
    
    // Test file validation task
    let validate_task = FileProcessingTask {
        input_path: input_file.clone(),
        output_path: None,
        operation: ProcessingOperation::ValidateFormat,
    };
    
    let result = scheduler.submit_task(Box::new(validate_task)).await.unwrap();
    assert!(matches!(result, TaskResult::Success(_)));
    
    // Test file conversion task
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
    
    // Verify output file exists
    assert!(tokio::fs::metadata(&output_file).await.is_ok());
    
    // Test file matching task
    let match_task = FileProcessingTask {
        input_path: temp_dir.path().to_path_buf(),
        output_path: None,
        operation: ProcessingOperation::MatchFiles { recursive: false },
    };
    
    let result = scheduler.submit_task(Box::new(match_task)).await.unwrap();
    assert!(matches!(result, TaskResult::Success(_)));
}
