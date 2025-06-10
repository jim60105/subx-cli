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

/// Retries an operation with an exponential backoff mechanism.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SubXError;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    /// Test basic retry mechanism
    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };

        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let operation = || async {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;
            if *count == 1 {
                Err(SubXError::AiService("First attempt fails".to_string()))
            } else {
                Ok("Success on second attempt".to_string())
            }
        };

        let result = retry_with_backoff(operation, &config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success on second attempt");
        assert_eq!(*attempt_count.lock().unwrap(), 2);
    }

    /// Test maximum retry attempts limit
    #[tokio::test]
    async fn test_retry_exhaust_max_attempts() {
        let config = RetryConfig {
            max_attempts: 2,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };

        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();

        let operation = || async {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;
            Err(SubXError::AiService("Always fails".to_string()))
        };

        let result: Result<String> = retry_with_backoff(operation, &config).await;
        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 2);
    }

    /// Test exponential backoff delay
    #[tokio::test]
    async fn test_exponential_backoff_timing() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_millis(200),
            backoff_multiplier: 2.0,
        };

        let attempt_times = Arc::new(Mutex::new(Vec::new()));
        let attempt_times_clone = attempt_times.clone();

        let operation = || async {
            let start_time = Instant::now();
            attempt_times_clone.lock().unwrap().push(start_time);
            Err(SubXError::AiService(
                "Always fails for timing test".to_string(),
            ))
        };

        let _overall_start = Instant::now();
        let _result: Result<String> = retry_with_backoff(operation, &config).await;

        let times = attempt_times.lock().unwrap();
        assert_eq!(times.len(), 3);

        // Verify delay times increase (considering execution time tolerance)
        if times.len() >= 2 {
            let delay1 = times[1].duration_since(times[0]);
            // First delay should be approximately 50ms (±20ms tolerance)
            assert!(delay1 >= Duration::from_millis(30));
            assert!(delay1 <= Duration::from_millis(100));
        }
    }

    /// Test maximum delay cap limit
    #[tokio::test]
    async fn test_max_delay_cap() {
        let config = RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(200), // Low cap
            backoff_multiplier: 3.0,               // High multiplier
        };

        let attempt_times = Arc::new(Mutex::new(Vec::new()));
        let attempt_times_clone = attempt_times.clone();

        let operation = || async {
            attempt_times_clone.lock().unwrap().push(Instant::now());
            Err(SubXError::AiService("Always fails".to_string()))
        };

        let _result: Result<String> = retry_with_backoff(operation, &config).await;

        let times = attempt_times.lock().unwrap();

        // Verify subsequent delays don't exceed max_delay
        if times.len() >= 3 {
            let delay2 = times[2].duration_since(times[1]);
            // Second delay should be capped at max_delay (±50ms tolerance)
            assert!(delay2 <= Duration::from_millis(250));
        }
    }

    /// Test configuration validity validation
    #[test]
    fn test_retry_config_validation() {
        // Valid configuration
        let valid_config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };
        assert!(valid_config.base_delay <= valid_config.max_delay);
        assert!(valid_config.max_attempts > 0);
        assert!(valid_config.backoff_multiplier > 1.0);
    }

    /// Test AI service integration simulation scenario
    #[tokio::test]
    async fn test_ai_service_integration_simulation() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };

        // Simulate AI service calls
        let request_count = Arc::new(Mutex::new(0));
        let request_count_clone = request_count.clone();

        let mock_ai_request = || async {
            let mut count = request_count_clone.lock().unwrap();
            *count += 1;

            match *count {
                1 => Err(SubXError::AiService("Network timeout".to_string())),
                2 => Err(SubXError::AiService("Rate limit exceeded".to_string())),
                3 => Ok("AI analysis complete".to_string()),
                _ => unreachable!(),
            }
        };

        let result = retry_with_backoff(mock_ai_request, &config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "AI analysis complete");
        assert_eq!(*request_count.lock().unwrap(), 3);
    }
}
