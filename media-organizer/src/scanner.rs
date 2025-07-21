use crate::types::{FileInfo, FileType, MediaMetadata};
use anyhow::Result;
use chrono::{DateTime, Local, TimeZone};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;
use walkdir::WalkDir;

/// Scanner for finding and analyzing media files
pub struct Scanner {
    input_dir: PathBuf,
    file_types: Option<Vec<FileType>>,
    exclude_patterns: Vec<String>,
    min_size: u64,
    max_size: Option<u64>,
    follow_links: bool,
    date_range: Option<(DateTime<Local>, DateTime<Local>)>,
    batch_size: usize,
    worker_threads: usize,
}

impl Scanner {
    pub fn new(input_dir: PathBuf) -> Self {
        Self {
            input_dir,
            file_types: None,
            exclude_patterns: Vec::new(),
            min_size: 0,
            max_size: None,
            follow_links: false,
            date_range: None,
            batch_size: 1000,
            worker_threads: num_cpus::get(),
        }
    }

    pub fn with_file_types(mut self, types: Vec<FileType>) -> Self {
        self.file_types = Some(types);
        self
    }

    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    pub fn with_size_limits(mut self, min: u64, max: Option<u64>) -> Self {
        self.min_size = min;
        self.max_size = max;
        self
    }

    pub fn with_follow_links(mut self, follow: bool) -> Self {
        self.follow_links = follow;
        self
    }

    #[allow(dead_code)]
    pub fn with_date_range(mut self, start: DateTime<Local>, end: DateTime<Local>) -> Self {
        self.date_range = Some((start, end));
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn with_worker_threads(mut self, threads: usize) -> Self {
        self.worker_threads = threads.max(1);
        self
    }

    /// Scan the input directory and send file information through the channel
    pub async fn scan(&self, tx: mpsc::Sender<FileInfo>) -> Result<()> {
        let scanner = Arc::new(self.clone_config());
        let (batch_tx, mut batch_rx) = mpsc::channel::<Vec<PathBuf>>(10);

        // Spawn file discovery task
        let discovery_scanner = scanner.clone();
        let discovery_handle =
            tokio::spawn(async move { discovery_scanner.discover_files(batch_tx).await });

        // Process batches in parallel
        let process_scanner = scanner.clone();
        let process_handle = tokio::spawn(async move {
            while let Some(batch) = batch_rx.recv().await {
                let scanner = process_scanner.clone();
                let tx = tx.clone();

                // Process batch in parallel using rayon
                let results: Vec<_> = batch
                    .par_iter()
                    .filter_map(|path| scanner.process_file(path).ok())
                    .collect();

                // Send results
                for file_info in results {
                    if tx.send(file_info).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            }
        });

        // Wait for completion
        discovery_handle.await??;
        // batch_rx will be dropped when process_handle completes
        process_handle.await?;

        Ok(())
    }

    /// Clone scanner configuration for Arc sharing
    fn clone_config(&self) -> Self {
        Self {
            input_dir: self.input_dir.clone(),
            file_types: self.file_types.clone(),
            exclude_patterns: self.exclude_patterns.clone(),
            min_size: self.min_size,
            max_size: self.max_size,
            follow_links: self.follow_links,
            date_range: self.date_range,
            batch_size: self.batch_size,
            worker_threads: self.worker_threads,
        }
    }

    /// Discover files and send them in batches
    async fn discover_files(&self, tx: mpsc::Sender<Vec<PathBuf>>) -> Result<()> {
        let input_dir = self.input_dir.clone();
        let follow_links = self.follow_links;
        let file_types = self.file_types.clone();
        let batch_size = self.batch_size;
        let exclude_patterns = self.exclude_patterns.clone();

        // Use blocking task for walkdir
        let batch_tx = tx;
        tokio::task::spawn_blocking(move || -> Result<()> {
            let walker = WalkDir::new(&input_dir)
                .follow_links(follow_links)
                .into_iter();

            let mut batch = Vec::with_capacity(batch_size);
            let mut files_discovered = 0;

            for entry in walker {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Error accessing path: {e}");
                        continue;
                    },
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();

                // Check if should process
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') {
                    continue;
                }

                // Check exclude patterns
                let skip = exclude_patterns.iter().any(|pattern| {
                    name.contains(pattern) || path.to_string_lossy().contains(pattern)
                });
                if skip {
                    continue;
                }

                // Quick file type check
                let file_type = FileType::from_extension(path);
                if file_type == FileType::Unknown {
                    continue;
                }

                if let Some(ref types) = file_types {
                    if !types.contains(&file_type) {
                        continue;
                    }
                }

                batch.push(path.to_path_buf());
                files_discovered += 1;

                // Send batch when full
                if batch.len() >= batch_size {
                    if batch_tx.blocking_send(batch.clone()).is_err() {
                        break; // Receiver dropped
                    }
                    batch.clear();
                }
            }

            // Send remaining files
            if !batch.is_empty() {
                let _ = batch_tx.blocking_send(batch);
            }

            info!("File discovery complete: {files_discovered} files found");
            Ok(())
        })
        .await??;

        Ok(())
    }

    /// Process a single file and create FileInfo
    fn process_file(&self, path: &Path) -> Result<FileInfo> {
        let file_type = FileType::from_extension(path);
        let metadata = std::fs::metadata(path)?;

        // Check file size
        let size = metadata.len();
        if size < self.min_size {
            anyhow::bail!("File too small");
        }

        if let Some(max) = self.max_size {
            if size > max {
                anyhow::bail!("File too large");
            }
        }

        // Get timestamps
        let modified = DateTime::<Local>::from(metadata.modified()?);
        let created = metadata.created().ok().map(DateTime::<Local>::from);

        // Check date range
        if let Some((start, end)) = self.date_range {
            let check_date = created.as_ref().unwrap_or(&modified);
            if check_date < &start || check_date > &end {
                anyhow::bail!("File outside date range");
            }
        }

        // Extract metadata (placeholder for now, would use exif crate)
        let media_metadata = self.extract_metadata(path, &file_type);

        Ok(FileInfo {
            path: path.to_path_buf(),
            file_type,
            size,
            modified,
            created,
            hash: None, // Will be computed later if duplicate detection is enabled
            metadata: media_metadata,
        })
    }

    /// Extract media-specific metadata
    fn extract_metadata(&self, path: &Path, file_type: &FileType) -> MediaMetadata {
        let mut metadata = MediaMetadata::default();

        // For now, we'll use the file's modification time as the date taken
        // This is a reasonable fallback when EXIF data is not available
        if let Ok(file_metadata) = std::fs::metadata(path) {
            if let Ok(modified) = file_metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let datetime = chrono::Local.timestamp_opt(duration.as_secs() as i64, 0).single();
                    metadata.date_taken = datetime;
                }
            }
        }

        // Basic metadata based on file type
        // In a production implementation, we would:
        // - Use kamadak-exif or similar for image EXIF data
        // - Use ffmpeg bindings for video metadata
        // - Extract actual dimensions, camera info, etc.
        if file_type.is_image() {
            // Placeholder values - would be extracted from actual image
            // metadata.width = Some(extracted_width);
            // metadata.height = Some(extracted_height);
            // metadata.camera_make = Some(extracted_make);
            // metadata.camera_model = Some(extracted_model);
        } else if file_type.is_video() {
            // Placeholder values - would be extracted from actual video
            // metadata.duration = Some(extracted_duration);
            // metadata.width = Some(extracted_width);
            // metadata.height = Some(extracted_height);
        }

        metadata
    }
}

/// Performance metrics for scanning
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct ScanMetrics {
    pub files_discovered: usize,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub total_size: u64,
    pub scan_duration: std::time::Duration,
}

impl Scanner {
    /// Scan with performance metrics
    #[allow(dead_code)]
    pub async fn scan_with_metrics(&self, tx: mpsc::Sender<FileInfo>) -> Result<ScanMetrics> {
        let start = std::time::Instant::now();
        let mut metrics = ScanMetrics::default();

        // TODO: Integrate metrics collection into scan process
        self.scan(tx).await?;

        metrics.scan_duration = start.elapsed();
        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_type_detection() {
        let jpg_path = Path::new("test.jpg");
        assert_eq!(FileType::from_extension(jpg_path), FileType::Jpeg);

        let mov_path = Path::new("video.MOV");
        assert_eq!(FileType::from_extension(mov_path), FileType::Mov);

        let unknown_path = Path::new("document.txt");
        assert_eq!(FileType::from_extension(unknown_path), FileType::Unknown);
    }

    #[test]
    fn test_scanner_builder() {
        let scanner = Scanner::new(PathBuf::from("/test"))
            .with_file_types(vec![FileType::Jpeg, FileType::Mp4])
            .with_size_limits(1024, Some(1024 * 1024))
            .with_batch_size(500)
            .with_worker_threads(4);

        assert_eq!(scanner.min_size, 1024);
        assert_eq!(scanner.max_size, Some(1024 * 1024));
        assert_eq!(scanner.batch_size, 500);
        assert_eq!(scanner.worker_threads, 4);
    }

    #[tokio::test]
    async fn test_scanner_basic_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = Scanner::new(temp_dir.path().to_path_buf());

        // Create test files with sufficient size
        let test_content = vec![0u8; 1024]; // 1KB of data
        fs::write(temp_dir.path().join("photo1.jpg"), &test_content).unwrap();
        fs::write(temp_dir.path().join("photo2.png"), &test_content).unwrap();
        fs::write(temp_dir.path().join("video.mp4"), &test_content).unwrap();
        fs::write(temp_dir.path().join("document.txt"), b"not media").unwrap();

        let (tx, mut rx) = mpsc::channel(100);

        // Run scanner in background
        let scan_handle = tokio::spawn(async move { scanner.scan(tx).await });

        // Collect results
        let mut files = Vec::new();
        while let Some(file) = rx.recv().await {
            files.push(file);
        }

        scan_handle.await.unwrap().unwrap();

        // Verify results
        assert_eq!(files.len(), 3); // Only media files
        assert!(files.iter().any(|f| f.path.ends_with("photo1.jpg")));
        assert!(files.iter().any(|f| f.path.ends_with("photo2.png")));
        assert!(files.iter().any(|f| f.path.ends_with("video.mp4")));
    }
}
