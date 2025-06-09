//! Parallel processing configuration module
use crate::config::{Config, OverflowStrategy};
use crate::error::SubXError;

/// Configuration for parallel processing behavior.
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum number of concurrent jobs (global limit)
    pub max_concurrent_jobs: usize,
    /// Limit for CPU-intensive tasks.
    pub cpu_intensive_limit: usize,
    /// Limit for IO-intensive tasks.
    pub io_intensive_limit: usize,
    /// Maximum task queue size.
    pub task_queue_size: usize,
    /// Whether task priorities are enabled.
    pub enable_task_priorities: bool,
    /// Whether workers auto-balance according to load.
    pub auto_balance_workers: bool,
    /// Strategy to apply when the task queue reaches its maximum size.
    pub queue_overflow_strategy: OverflowStrategy,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_parallel_config_validation() {
        let invalid = ParallelConfig {
            max_concurrent_jobs: 1,
            cpu_intensive_limit: 0,
            io_intensive_limit: 1,
            task_queue_size: 1,
            enable_task_priorities: true,
            auto_balance_workers: false,
            queue_overflow_strategy: OverflowStrategy::Block,
        };
        assert!(invalid.validate().is_err());

        let valid = ParallelConfig {
            max_concurrent_jobs: 1,
            cpu_intensive_limit: 1,
            io_intensive_limit: 1,
            task_queue_size: 1,
            enable_task_priorities: true,
            auto_balance_workers: false,
            queue_overflow_strategy: OverflowStrategy::Block,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_from_app_config_defaults() {
        let app_cfg = Config::default();
        let pc = ParallelConfig::from_app_config(&app_cfg);
        assert_eq!(pc.max_concurrent_jobs, app_cfg.general.max_concurrent_jobs);
        assert_eq!(pc.cpu_intensive_limit, app_cfg.parallel.cpu_intensive_limit);
        assert_eq!(pc.io_intensive_limit, app_cfg.parallel.io_intensive_limit);
        assert_eq!(pc.task_queue_size, app_cfg.parallel.task_queue_size);
        assert_eq!(
            pc.enable_task_priorities,
            app_cfg.parallel.enable_task_priorities
        );
        assert_eq!(
            pc.auto_balance_workers,
            app_cfg.parallel.auto_balance_workers
        );
        assert_eq!(
            pc.queue_overflow_strategy,
            app_cfg.parallel.queue_overflow_strategy
        );
    }
}

impl ParallelConfig {
    /// Construct ParallelConfig from application Config.
    pub fn from_app_config(config: &Config) -> Self {
        let p = &config.parallel;
        Self {
            max_concurrent_jobs: config.general.max_concurrent_jobs,
            cpu_intensive_limit: p.cpu_intensive_limit,
            io_intensive_limit: p.io_intensive_limit,
            task_queue_size: p.task_queue_size,
            enable_task_priorities: p.enable_task_priorities,
            auto_balance_workers: p.auto_balance_workers,
            queue_overflow_strategy: p.queue_overflow_strategy,
        }
    }

    /// Validate configuration values for correctness.
    pub fn validate(&self) -> Result<(), SubXError> {
        if self.max_concurrent_jobs == 0 {
            return Err(SubXError::config(
                "最大併發任務數 (max_concurrent_jobs) 需大於 0",
            ));
        }
        if self.cpu_intensive_limit == 0 {
            return Err(SubXError::config(
                "CPU 密集型任務限制 (cpu_intensive_limit) 需大於 0",
            ));
        }
        if self.io_intensive_limit == 0 {
            return Err(SubXError::config(
                "I/O 密集型任務限制 (io_intensive_limit) 需大於 0",
            ));
        }
        if self.task_queue_size == 0 {
            return Err(SubXError::config("佇列大小 (task_queue_size) 需大於 0"));
        }
        // Overflow strategy is always valid.
        Ok(())
    }
}
