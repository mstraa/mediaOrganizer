use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use media_organizer::scanner::Scanner;
use media_organizer::types::FileType;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn create_test_files(dir: &TempDir, count: usize) {
    for i in 0..count {
        let ext = match i % 4 {
            0 => "jpg",
            1 => "png",
            2 => "mp4",
            _ => "mov",
        };
        let filename = format!("file_{:06}.{}", i, ext);
        let content = format!("test content {}", i);
        fs::write(dir.path().join(filename), content.as_bytes()).unwrap();
    }
}

fn benchmark_scanner_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_throughput");

    for file_count in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            file_count,
            |b, &file_count| {
                let temp_dir = TempDir::new().unwrap();
                create_test_files(&temp_dir, file_count);

                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let scanner = Scanner::new(temp_dir.path().to_path_buf())
                            .with_batch_size(100)
                            .with_worker_threads(4);

                        let (tx, mut rx) = mpsc::channel(1000);

                        let scan_handle = tokio::spawn(async move { scanner.scan(tx).await });

                        let mut count = 0;
                        while let Some(_) = rx.recv().await {
                            count += 1;
                        }

                        scan_handle.await.unwrap().unwrap();
                        black_box(count);
                    });
                });
            },
        );
    }

    group.finish();
}

fn benchmark_scanner_memory(c: &mut Criterion) {
    c.bench_function("scanner_memory_10k_files", |b| {
        let temp_dir = TempDir::new().unwrap();
        create_test_files(&temp_dir, 10000);

        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let scanner = Scanner::new(temp_dir.path().to_path_buf()).with_batch_size(1000);

                let (tx, mut rx) = mpsc::channel(100); // Small buffer to test streaming

                let scan_handle = tokio::spawn(async move { scanner.scan(tx).await });

                // Simulate slow consumer to test memory pressure
                while let Some(_) = rx.recv().await {
                    tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
                }

                scan_handle.await.unwrap().unwrap();
            });
        });
    });
}

fn benchmark_file_type_detection(c: &mut Criterion) {
    c.bench_function("file_type_detection", |b| {
        let paths = vec![
            PathBuf::from("test.jpg"),
            PathBuf::from("test.JPEG"),
            PathBuf::from("test.png"),
            PathBuf::from("test.heic"),
            PathBuf::from("test.mp4"),
            PathBuf::from("test.MOV"),
            PathBuf::from("test.unknown"),
        ];

        b.iter(|| {
            for path in &paths {
                black_box(FileType::from_extension(path));
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_scanner_throughput,
    benchmark_scanner_memory,
    benchmark_file_type_detection
);
criterion_main!(benches);
