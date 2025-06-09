use crate::Result;
use tokio::time::{Duration, sleep};

/// Retry configuration for AI service operations.
///
/// Configures the retry behavior for AI API calls, including
/// backoff strategies and maximum attempt limits.
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial delay between retries
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

/// 使用指數退避機制重試操作
pub async fn retry_with_backoff<F, Fut, T>(operation: F, config: &RetryConfig) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);

                if attempt < config.max_attempts - 1 {
                    let delay = std::cmp::min(
                        Duration::from_millis(
                            (config.base_delay.as_millis() as f64
                                * config.backoff_multiplier.powi(attempt as i32))
                                as u64,
                        ),
                        config.max_delay,
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
