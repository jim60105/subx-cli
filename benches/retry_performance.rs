use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;
use subx_cli::{
    error::SubXError,
    services::ai::retry::{RetryConfig, retry_with_backoff},
};
use tokio::runtime::Runtime;

fn bench_retry_immediate_success(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("retry_immediate_success", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = RetryConfig {
                    max_attempts: 3,
                    base_delay: Duration::from_millis(1),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                };

                let operation = || async { Ok::<String, SubXError>("Success".to_string()) };
                let result = retry_with_backoff(operation, &config).await;
                std::hint::black_box(result)
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
                    base_delay: Duration::from_millis(1),
                    max_delay: Duration::from_secs(1),
                    backoff_multiplier: 2.0,
                };

                use std::sync::atomic::{AtomicUsize, Ordering};
                let attempt = AtomicUsize::new(0);
                let operation = || async {
                    let current_attempt = attempt.fetch_add(1, Ordering::SeqCst);
                    if current_attempt < 2 {
                        Err(SubXError::config("Failure"))
                    } else {
                        Ok("Success".to_string())
                    }
                };

                let result = retry_with_backoff(operation, &config).await;
                std::hint::black_box(result)
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
