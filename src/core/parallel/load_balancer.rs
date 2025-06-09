//! Load balancer for dynamic adjustment of worker distributions
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Balancer that tracks CPU and I/O load history and suggests optimal worker distribution.
#[derive(Debug, Clone)]
pub struct LoadBalancer {
    cpu_load_history: VecDeque<f64>,
    io_load_history: VecDeque<f64>,
    last_balance_time: Instant,
    balance_interval: Duration,
}

impl LoadBalancer {
    /// Create a new load balancer with default settings.
    pub fn new() -> Self {
        Self {
            cpu_load_history: VecDeque::with_capacity(10),
            io_load_history: VecDeque::with_capacity(10),
            last_balance_time: Instant::now(),
            balance_interval: Duration::from_secs(30),
        }
    }

    /// Check if it's time to rebalance according to the interval.
    pub fn should_rebalance(&self) -> bool {
        self.last_balance_time.elapsed() >= self.balance_interval
    }

    /// Calculate optimal distribution of CPU and I/O workers.
    ///
    /// Returns a tuple (cpu_workers, io_workers) indicating suggested new limits.
    pub fn calculate_optimal_distribution(&self) -> (usize, usize) {
        // Default behavior: maintain current distribution
        // Actual balancing logic to be implemented in future extensions.
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_defaults() {
        let balancer = LoadBalancer::new();
        assert!(!balancer.should_rebalance());
        let (cpu, io) = balancer.calculate_optimal_distribution();
        assert_eq!((cpu, io), (0, 0));
    }
}
