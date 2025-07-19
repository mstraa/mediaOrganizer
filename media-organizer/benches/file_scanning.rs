use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::Path;
use tempfile::tempdir;

fn benchmark_file_type_detection(c: &mut Criterion) {
    c.bench_function("file_type_detection", |b| {
        b.iter(|| {
            // Benchmark file type detection
            let paths = vec![
                Path::new("test.jpg"),
                Path::new("video.MP4"),
                Path::new("image.HEIC"),
                Path::new("raw.CR2"),
                Path::new("document.txt"),
            ];

            for path in paths {
                black_box(media_organizer::types::FileType::from_extension(path));
            }
        });
    });
}

fn benchmark_hash_computation(c: &mut Criterion) {
    // This would require creating temporary files
    // For now, this is a placeholder
    c.bench_function("blake3_hashing", |b| {
        b.iter(|| {
            let data = vec![0u8; 1024 * 1024]; // 1MB of data
            let mut hasher = blake3::Hasher::new();
            hasher.update(&data);
            black_box(hasher.finalize());
        });
    });
}

criterion_group!(
    benches,
    benchmark_file_type_detection,
    benchmark_hash_computation
);
criterion_main!(benches);
