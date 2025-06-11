//! Parallel processing configuration module
use crate::config::{Config, OverflowStrategy};
use crate::error::SubXError;

/// Configuration for parallel processing behavior.
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum number of concurrent jobs (global limit)
    pub max_concurrent_jobs: usize,
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
        // Test invalid max_concurrent_jobs and task_queue_size settings
        let invalid_jobs = ParallelConfig {
            max_concurrent_jobs: 0,
            task_queue_size: 1,
            enable_task_priorities: true,
            auto_balance_workers: false,
            queue_overflow_strategy: OverflowStrategy::Block,
        };
        assert!(invalid_jobs.validate().is_err());

        let invalid_queue = ParallelConfig {
            max_concurrent_jobs: 1,
            task_queue_size: 0,
            enable_task_priorities: true,
            auto_balance_workers: false,
            queue_overflow_strategy: OverflowStrategy::Block,
        };
        assert!(invalid_queue.validate().is_err());

        let valid = ParallelConfig {
            max_concurrent_jobs: 1,
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
            app_cfg.parallel.overflow_strategy
        );
    }
}

impl ParallelConfig {
    /// Construct ParallelConfig from application Config.
    pub fn from_app_config(config: &Config) -> Self {
        let p = &config.parallel;
        Self {
            max_concurrent_jobs: config.general.max_concurrent_jobs,
            task_queue_size: p.task_queue_size,
            enable_task_priorities: p.enable_task_priorities,
            auto_balance_workers: p.auto_balance_workers,
            queue_overflow_strategy: p.overflow_strategy.clone(),
        }
    }

    /// Validate configuration values for correctness.
    pub fn validate(&self) -> Result<(), SubXError> {
        if self.max_concurrent_jobs == 0 {
            return Err(SubXError::config(
                "Max concurrent jobs (max_concurrent_jobs) must be greater than 0",
            ));
        }
        if self.task_queue_size == 0 {
            return Err(SubXError::config(
                "Queue size (task_queue_size) must be greater than 0",
            ));
        }
        // Overflow strategy is always valid.
        Ok(())
    }
}
