use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::hint::black_box;

/// Generate a deterministic unique identifier for a file
fn generate_file_id(relative_path: &str, file_size: u64) -> String {
    let mut hasher = DefaultHasher::new();
    relative_path.hash(&mut hasher);
    file_size.hash(&mut hasher);

    format!("file_{:016x}", hasher.finish())
}

fn bench_file_id_generation(c: &mut Criterion) {
    c.bench_function("file_id_generation_single", |b| {
        b.iter(|| {
            generate_file_id(
                black_box("test/directory/very_long_filename_with_unicode_subtitles.mkv"),
                black_box(1024 * 1024 * 1024), // 1GB
            )
        })
    });

    c.bench_function("file_id_generation_batch_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                generate_file_id(
                    black_box(&format!("season{}/episode{:03}.mkv", i / 10 + 1, i)),
                    black_box(1000000 + i as u64),
                );
            }
        })
    });

    c.bench_function("file_id_generation_batch_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                generate_file_id(
                    black_box(&format!("movies/year{}/movie_{:04}.mkv", 2020 + i / 100, i)),
                    black_box(1000000 + i as u64),
                );
            }
        })
    });

    // Test different file path lengths
    c.bench_function("file_id_generation_long_path", |b| {
        let long_path = "very/long/directory/structure/with/many/nested/folders/and/unicode/characters/videos/subtitles/season_one/episode_one/final_filename.mkv";
        b.iter(|| {
            generate_file_id(black_box(long_path), black_box(5000000000)) // 5GB
        })
    });
}

fn bench_id_collision_resistance(c: &mut Criterion) {
    use std::collections::HashSet;

    c.bench_function("collision_test_10000_files", |b| {
        b.iter(|| {
            let mut ids = HashSet::new();
            let mut collisions = 0;

            for i in 0..10000 {
                let id = generate_file_id(
                    black_box(&format!("test_dir_{}/file_{:06}.mkv", i / 1000, i)),
                    black_box(1000000 + (i * 137) as u64), // Use prime number to avoid patterns
                );

                if !ids.insert(id) {
                    collisions += 1;
                }
            }

            black_box(collisions) // Expected to be 0
        })
    });
}

criterion_group!(
    benches,
    bench_file_id_generation,
    bench_id_collision_resistance
);
criterion_main!(benches);
