use criterion::{Criterion, criterion_group, criterion_main};

/// 舊的轉碼基準測試
fn benchmark_old_transcoding_method(c: &mut Criterion) {
    // TODO: 實作舊轉碼方法性能測試
    c.bench_function("old_transcode", |b| b.iter(|| ()));
}

/// 新的直接載入基準測試
fn benchmark_new_direct_method(c: &mut Criterion) {
    // TODO: 實作直接載入方法性能測試
    c.bench_function("new_direct", |b| b.iter(|| ()));
}

criterion_group!(
    vad_performance,
    benchmark_old_transcoding_method,
    benchmark_new_direct_method
);
criterion_main!(vad_performance);
