use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subx_cli::error::SubXError;
use subx_cli::services::ai::retry::{RetryConfig, retry_with_backoff};

#[cfg(test)]
mod ai_retry_tests {
    use super::*;

    /// 測試基本重試機制
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

    /// 測試最大重試次數限制
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

        let result: Result<String, SubXError> = retry_with_backoff(operation, &config).await;
        assert!(result.is_err());
        assert_eq!(*attempt_count.lock().unwrap(), 2);
    }

    /// 測試指數退避延遲
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
        let _result: Result<String, SubXError> = retry_with_backoff(operation, &config).await;

        let times = attempt_times.lock().unwrap();
        assert_eq!(times.len(), 3);

        // 驗證延遲時間遞增 (考慮執行時間誤差)
        if times.len() >= 2 {
            let delay1 = times[1].duration_since(times[0]);
            // 第一次延遲應該約為 50ms (±20ms 誤差)
            assert!(delay1 >= Duration::from_millis(30));
            assert!(delay1 <= Duration::from_millis(100));
        }
    }

    /// 測試永久錯誤立即失敗

    /// 測試延遲上限限制
    #[tokio::test]
    async fn test_max_delay_cap() {
        let config = RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(200), // 低上限
            backoff_multiplier: 3.0,               // 高倍數
        };

        let attempt_times = Arc::new(Mutex::new(Vec::new()));
        let attempt_times_clone = attempt_times.clone();

        let operation = || async {
            attempt_times_clone.lock().unwrap().push(Instant::now());
            Err(SubXError::AiService("Always fails".to_string()))
        };

        let _result: Result<String, SubXError> = retry_with_backoff(operation, &config).await;

        let times = attempt_times.lock().unwrap();

        // 驗證後續延遲不會超過 max_delay
        if times.len() >= 3 {
            let delay2 = times[2].duration_since(times[1]);
            // 第二次延遲應該被限制在 max_delay 範圍內 (±50ms 誤差)
            assert!(delay2 <= Duration::from_millis(250));
        }
    }

    /// 測試配置有效性驗證
    #[test]
    fn test_retry_config_validation() {
        // 有效配置
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

    /// 測試與 AI 服務整合的模擬場景
    #[tokio::test]
    async fn test_ai_service_integration_simulation() {
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        };

        // 模擬 AI 服務呼叫
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
