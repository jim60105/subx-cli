use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::time::Duration;
use subx_cli::services::ai::retry::{RetryConfig, RetryError, retry_with_backoff};
use tokio::runtime::Runtime;

fn bench_retry_immediate_success(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("retry_immediate_success", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = RetryConfig {
                    max_attempts: 3,
                    initial_delay: Duration::from_millis(1),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                };

                let operation = || async { Ok::<String, RetryError>("Success".to_string()) };
                let result = retry_with_backoff(&config, operation).await;
                black_box(result)
            })
        })
    });
}

fn bench_retry_with_failures(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("retry_with_two_failures", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = RetryConfig {
                    max_attempts: 3,
                    initial_delay: Duration::from_millis(1),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                };

                let mut attempt = 0;
                let operation = || async {
                    attempt += 1;
                    if attempt <= 2 {
                        Err(RetryError::Temporary("Failure".to_string()))
                    } else {
                        Ok("Success".to_string())
                    }
                };

                let result = retry_with_backoff(&config, operation).await;
                black_box(result)
            })
        })
    });
}

criterion_group!(
    benches,
    bench_retry_immediate_success,
    bench_retry_with_failures
);
criterion_main!(benches);
