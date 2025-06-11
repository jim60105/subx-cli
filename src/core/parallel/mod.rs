//! High-performance parallel processing system for subtitle operations.
//!
//! This module provides a sophisticated task scheduling and execution framework
//! designed specifically for subtitle processing workloads. It offers intelligent
//! load balancing, resource management, and fault tolerance for CPU-intensive
//! operations like format conversion, AI analysis, and audio synchronization.
//!
//! # Core Features
//!
//! ## Intelligent Task Scheduling
//! - **Priority-Based Queuing**: Tasks are prioritized based on complexity and user preferences
//! - **Resource-Aware Scheduling**: Considers CPU, memory, and I/O constraints
//! - **Adaptive Load Balancing**: Dynamically adjusts worker allocation based on system load
//! - **Dependency Management**: Handles task dependencies and execution ordering
//!
//! ## Worker Pool Management
//! - **Dynamic Scaling**: Automatically adjusts worker count based on system resources
//! - **Specialized Workers**: Different worker types for different operation categories
//! - **Health Monitoring**: Tracks worker performance and handles failures gracefully
//! - **Resource Isolation**: Prevents resource contention between concurrent tasks
//!
//! ## Performance Optimization
//! - **Batch Processing**: Groups similar tasks for efficient execution
//! - **Memory Pool**: Reuses memory allocations to reduce garbage collection
//! - **Cache-Aware Scheduling**: Optimizes for CPU cache locality
//! - **NUMA Awareness**: Considers system topology for optimal performance
//!
//! # Architecture Overview
//!
//! The parallel processing system follows a producer-consumer pattern with
//! multiple specialized components:
//!
//! ```text
//! ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
//! │   Task Queue    │────│   Scheduler      │────│  Load Balancer  │
//! │   - Priority    │    │   - Dispatch     │    │   - Resource    │
//! │   - Dependencies│    │   - Monitoring   │    │   - Scaling     │
//! │   - Batching    │    │   - Recovery     │    │   - Affinity    │
//! └─────────────────┘    └──────────────────┘    └─────────────────┘
//!         │                        │                        │
//!         └────────────────────────┼────────────────────────┘
//!                                  │
//!                    ┌─────────────────────────┐
//!                    │     Worker Pool         │
//!                    │   ┌───────────────────┐ │
//!                    │   │ Format Converter  │ │
//!                    │   │ AI Analyzer       │ │
//!                    │   │ Audio Processor   │ │
//!                    │   │ File Manager      │ │
//!                    │   └───────────────────┘ │
//!                    └─────────────────────────┘
//! ```
//!
//! # Usage Examples
//!
//! ## Basic Task Execution
//!
//! ```rust,ignore
//! use subx_cli::core::parallel::{TaskScheduler, Task, ProcessingOperation};
//! use std::path::PathBuf;
//!
//! // Create a task scheduler with default configuration
//! let scheduler = TaskScheduler::new().await?;
//!
//! // Define a format conversion task
//! let task = Task::new(
//!     ProcessingOperation::FormatConversion {
//!         input_path: PathBuf::from("input.srt"),
//!         output_path: PathBuf::from("output.ass"),
//!         target_format: "ass".to_string(),
//!     },
//!     1, // Normal priority
//! );
//!
//! // Submit task and wait for completion
//! let result = scheduler.submit_and_wait(task).await?;
//! println!("Task completed: {:?}", result);
//! ```
//!
//! ## Batch Processing
//!
//! ```rust,ignore
//! use subx_cli::core::parallel::{FileProcessingTask, TaskScheduler};
//!
//! let scheduler = TaskScheduler::new().await?;
//! let mut tasks = Vec::new();
//!
//! // Create multiple tasks for batch processing
//! for file_path in subtitle_files {
//!     let task = FileProcessingTask::new(
//!         file_path,
//!         ProcessingOperation::EncodingDetection,
//!         2, // Higher priority for encoding detection
//!     );
//!     tasks.push(task);
//! }
//!
//! // Submit all tasks for parallel execution
//! let results = scheduler.submit_batch(tasks).await?;
//!
//! for result in results {
//!     match result {
//!         Ok(output) => println!("Success: {:?}", output),
//!         Err(error) => eprintln!("Failed: {}", error),
//!     }
//! }
//! ```
//!
//! ## Custom Worker Configuration
//!
//! ```rust,ignore
//! use subx_cli::core::parallel::{TaskScheduler, WorkerConfig};
//!
//! let config = WorkerConfig {
//!     max_workers: 8,
//!     min_workers: 2,
//!     cpu_intensive_workers: 4,
//!     io_intensive_workers: 2,
//!     memory_limit_mb: 1024,
//!     task_timeout_secs: 300,
//! };
//!
//! let scheduler = TaskScheduler::with_config(config).await?;
//! ```
//!
//! # Task Types
//!
//! ## Format Conversion Tasks
//! - Convert between different subtitle formats (SRT, ASS, VTT, SUB)
//! - Preserve styling information where possible
//! - Handle encoding conversion automatically
//!
//! ## AI Analysis Tasks
//! - Semantic content analysis for matching
//! - Language detection and verification
//! - Quality assessment and scoring
//! - Content similarity comparison
//!
//! ## Audio Processing Tasks
//! - Speech detection and timing extraction
//! - Audio-subtitle synchronization
//! - Voice activity detection
//! - Audio quality analysis
//!
//! ## File Management Tasks
//! - Batch file operations (copy, move, rename)
//! - Encoding detection and conversion
//! - Metadata extraction and validation
//! - Directory structure analysis
//!
//! # Performance Characteristics
//!
//! ## CPU Optimization
//! - **Work Stealing**: Idle workers steal tasks from busy workers
//! - **Cache Affinity**: Tasks are scheduled to maintain CPU cache locality
//! - **Thread Pinning**: Critical workers can be pinned to specific CPU cores
//! - **SIMD Utilization**: Vectorized operations where applicable
//!
//! ## Memory Management
//! - **Pool Allocation**: Reuses memory buffers to reduce allocation overhead
//! - **Memory Pressure Handling**: Automatically reduces concurrency under memory pressure
//! - **Garbage Collection Optimization**: Minimizes allocations in hot paths
//! - **Memory-Mapped I/O**: Uses memory mapping for large file operations
//!
//! ## I/O Optimization
//! - **Asynchronous I/O**: Non-blocking file operations where possible
//! - **Read-Ahead**: Predictive file reading based on task patterns
//! - **Write Coalescing**: Batches small writes for better performance
//! - **Network Optimization**: Optimized handling of AI service requests
//!
//! # Error Handling and Recovery
//!
//! ## Task-Level Recovery
//! - **Automatic Retry**: Failed tasks are retried with exponential backoff
//! - **Fallback Strategies**: Alternative approaches for failed operations
//! - **Partial Results**: Returns partial results when possible
//! - **Progress Preservation**: Saves intermediate results for long-running tasks
//!
//! ## System-Level Recovery
//! - **Worker Restart**: Automatically restarts failed workers
//! - **Resource Cleanup**: Ensures proper cleanup after failures
//! - **State Persistence**: Maintains scheduler state across restarts
//! - **Graceful Degradation**: Continues operation with reduced capacity
//!
//! # Monitoring and Observability
//!
//! The parallel system provides comprehensive monitoring:
//! - **Task Metrics**: Execution time, success rate, resource usage
//! - **Worker Health**: CPU usage, memory consumption, error rates
//! - **System Performance**: Overall throughput, queue depth, response times
//! - **Resource Utilization**: CPU, memory, disk, and network usage
//!
//! # Thread Safety
//!
//! All components are designed for concurrent access:
//! - Lock-free data structures where possible
//! - Minimal contention in critical paths
//! - Safe sharing of resources between threads
//! - Proper synchronization for shared state

pub mod config;
pub mod load_balancer;
pub mod pool;
pub mod scheduler;
pub mod task;
pub mod worker;

pub use scheduler::TaskScheduler;
pub use task::{FileProcessingTask, ProcessingOperation, Task, TaskResult, TaskStatus};
