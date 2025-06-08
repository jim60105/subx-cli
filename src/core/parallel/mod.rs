//! Parallel processing system module
pub mod pool;
pub mod scheduler;
pub mod task;
pub mod worker;

pub use scheduler::TaskScheduler;
pub use task::{FileProcessingTask, ProcessingOperation, Task, TaskResult, TaskStatus};
pub use worker::{Worker, WorkerPool};
