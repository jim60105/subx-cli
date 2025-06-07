use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;
use subx::services::audio::AudioAnalyzer;

fn bench_audio_envelope_extraction(c: &mut Criterion) {
    let analyzer = AudioAnalyzer::new(16000);
    let test_audio_path = Path::new("test_data/sample.mp4");

    if test_audio_path.exists() {
        c.bench_function("audio_envelope_extraction", |b| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async { analyzer.extract_envelope(black_box(test_audio_path)).await })
            })
        });
    }
}

fn bench_correlation_calculation(c: &mut Criterion) {
    let audio_data = vec![0.5f32; 16000];
    let subtitle_data = vec![1.0f32; 16000];

    c.bench_function("correlation_calculation", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for (a, s) in audio_data.iter().zip(subtitle_data.iter()) {
                sum += a * s;
            }
            black_box(sum)
        })
    });
}

criterion_group!(
    benches,
    bench_audio_envelope_extraction,
    bench_correlation_calculation
);
criterion_main!(benches);
