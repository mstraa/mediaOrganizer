# Task 6: Implement Progress Tracking and Reporting

## Objective
Create a comprehensive progress tracking system that provides real-time feedback, performance metrics, and detailed reporting for all operations while maintaining minimal performance overhead.

## Prerequisites
- Tasks 2-5 completed (CLI, Scanner, Duplicate Detection, Organization)
- `indicatif` crate for progress bars
- `tracing` crate for structured logging

## Implementation Steps

### 1. Define Progress Tracking Structure (`src/progress.rs`)
```rust
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub struct ProgressTracker {
    multi_progress: MultiProgress,
    scan_progress: ProgressBar,
    hash_progress: ProgressBar,
    organize_progress: ProgressBar,
    
    // Metrics
    files_scanned: Arc<AtomicUsize>,
    files_hashed: Arc<AtomicUsize>,
    files_organized: Arc<AtomicUsize>,
    bytes_processed: Arc<AtomicU64>,
    duplicates_found: Arc<AtomicUsize>,
    errors_count: Arc<AtomicUsize>,
    
    // Timing
    start_time: Instant,
    phase_times: Arc<Mutex<HashMap<String, Duration>>>,
}

pub struct ProgressReport {
    pub total_files: usize,
    pub processed_files: usize,
    pub total_size: u64,
    pub duplicates_found: usize,
    pub space_saved: u64,
    pub errors: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
}
```

### 2. Implement Multi-Phase Progress Bars
```rust
impl ProgressTracker {
    pub fn new() -> Self {
        let multi = MultiProgress::new();
        
        let scan_bar = multi.add(ProgressBar::new_spinner());
        scan_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} Scanning: {msg} [{elapsed}] Files: {pos}")
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        );
        
        let hash_bar = multi.add(ProgressBar::new(0));
        hash_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} Hashing: [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) [{elapsed}/{eta}]")
                .progress_chars("#>-")
        );
        
        // Similar for organize_bar
    }
}
```

### 3. Add Real-Time Metrics Display
```rust
pub fn start_metrics_display(&self) {
    let metrics = self.clone();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            let fps = metrics.calculate_files_per_second();
            let mbps = metrics.calculate_megabytes_per_second();
            let eta = metrics.estimate_time_remaining();
            
            metrics.update_display(fps, mbps, eta);
        }
    });
}
```

### 4. Implement Performance Monitoring
```rust
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub scan_rate: f64,      // files/second
    pub hash_rate: f64,      // MB/second
    pub organize_rate: f64,  // files/second
    pub peak_memory: u64,    // bytes
    pub cpu_usage: f32,      // percentage
    pub io_wait_time: Duration,
}

impl ProgressTracker {
    pub async fn monitor_performance(&self) {
        use sysinfo::{System, SystemExt, ProcessExt};
        
        let mut system = System::new_all();
        let pid = std::process::id();
        
        loop {
            system.refresh_process(pid);
            
            if let Some(process) = system.process(pid) {
                self.update_performance_metrics(
                    process.memory(),
                    process.cpu_usage(),
                );
            }
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

### 5. Add Structured Logging
```rust
use tracing::{info, warn, error, debug, span, Level};

pub fn setup_logging(verbose: bool) {
    let level = if verbose { Level::DEBUG } else { Level::INFO };
    
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(level)
        .init();
}

// Usage throughout the application
span!(Level::INFO, "scanning", path = ?input_dir);
info!("Found {} media files", count);
debug!("Processing file: {}", file_path.display());
```

### 6. Create Summary Report Generator
```rust
impl ProgressTracker {
    pub fn generate_report(&self) -> ProgressReport {
        let elapsed = self.start_time.elapsed();
        
        ProgressReport {
            total_files: self.files_scanned.load(Ordering::Relaxed),
            processed_files: self.files_organized.load(Ordering::Relaxed),
            total_size: self.bytes_processed.load(Ordering::Relaxed),
            duplicates_found: self.duplicates_found.load(Ordering::Relaxed),
            space_saved: self.calculate_space_saved(),
            errors: self.collect_errors(),
            performance_metrics: self.get_performance_metrics(),
            elapsed_time: elapsed,
        }
    }
    
    pub fn print_summary(&self) {
        let report = self.generate_report();
        
        println!("\n📊 Operation Summary");
        println!("══════════════════════════════════════");
        println!("✅ Files processed: {}", report.processed_files);
        println!("📁 Total size: {}", format_bytes(report.total_size));
        println!("🔍 Duplicates found: {}", report.duplicates_found);
        println!("💾 Space saved: {}", format_bytes(report.space_saved));
        println!("⏱️  Time elapsed: {}", format_duration(report.elapsed_time));
        println!("⚡ Average speed: {:.2} files/sec", 
            report.processed_files as f64 / report.elapsed_time.as_secs_f64());
        
        if !report.errors.is_empty() {
            println!("\n⚠️  Errors encountered: {}", report.errors.len());
        }
    }
}
```

### 7. Implement Progress Persistence
```rust
pub async fn save_progress(&self, checkpoint_file: &Path) -> Result<()> {
    let checkpoint = ProgressCheckpoint {
        timestamp: Utc::now(),
        files_processed: self.files_organized.load(Ordering::Relaxed),
        current_phase: self.current_phase.lock().clone(),
        completed_operations: self.get_completed_operations(),
    };
    
    let json = serde_json::to_string_pretty(&checkpoint)?;
    tokio::fs::write(checkpoint_file, json).await?;
    
    Ok(())
}
```

## Integration Requirements
- Thread-safe progress updates from all modules
- Minimal performance impact (<1% overhead)
- Graceful degradation if terminal doesn't support progress bars
- Support for both interactive and non-interactive modes
- JSON output option for automation

## Testing Strategy
- Unit tests for metric calculations
- Integration tests with mock operations
- Performance overhead measurement
- Test progress bar rendering
- Verify thread safety

## Success Criteria
- [x] Real-time progress updates work smoothly
- [x] Performance metrics are accurate
- [ ] Less than 1% performance overhead (needs performance testing)
- [x] Summary report is comprehensive
- [x] Works in CI/CD environments
- [x] Thread-safe updates from parallel operations

## Example Output
```
⠸ Scanning: /Users/photos [00:02:34] Files: 125,432
████████████████████░░░░░░░░░░░░░░░░░░░░ 45% Hashing: 56,432/125,432 [00:05:12/00:06:30]
██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 25% Organizing: 31,234/125,432 [00:03:45/00:11:00]

📊 Live Metrics:
├─ Scan rate: 1,234 files/sec
├─ Hash rate: 523.4 MB/sec
├─ Organization rate: 234 files/sec
├─ Memory usage: 187 MB
└─ CPU usage: 78%
```

## Next Steps
After completing progress tracking, the application is ready for:
- Integration testing of all components
- Performance optimization
- Final testing and release preparation