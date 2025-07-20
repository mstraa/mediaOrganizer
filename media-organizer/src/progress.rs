use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Pid, System};
use tokio::task::JoinHandle;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

/// Progress tracking for media organization operations
pub struct ProgressTracker {
    multi: Arc<MultiProgress>,
    main_bar: ProgressBar,
    scan_bar: Option<ProgressBar>,
    duplicate_bar: Option<ProgressBar>,
    process_bar: Option<ProgressBar>,

    // Metrics
    files_scanned: Arc<AtomicUsize>,
    files_hashed: Arc<AtomicUsize>,
    files_organized: Arc<AtomicUsize>,
    bytes_processed: Arc<AtomicU64>,
    duplicates_found: Arc<AtomicUsize>,
    errors_count: Arc<AtomicUsize>,

    // Timing
    start_time: Instant,
    #[allow(dead_code)]
    phase_times: Arc<Mutex<HashMap<String, Duration>>>,

    // Performance monitoring
    metrics_handle: Option<JoinHandle<()>>,
    peak_memory: Arc<AtomicU64>,
    peak_cpu: Arc<AtomicU64>,
}

/// Summary report of the operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    pub total_files: usize,
    pub processed_files: usize,
    pub total_size: u64,
    pub duplicates_found: usize,
    pub space_saved: u64,
    pub errors: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
    pub elapsed_time: Duration,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub scan_rate: f64,     // files/second
    pub hash_rate: f64,     // MB/second
    pub organize_rate: f64, // files/second
    pub peak_memory: u64,   // bytes
    pub peak_cpu: f64,      // percentage
}

impl ProgressTracker {
    pub fn new(quiet: bool) -> Self {
        let multi = Arc::new(MultiProgress::new());

        let main_bar = if quiet {
            ProgressBar::hidden()
        } else {
            multi.add(ProgressBar::new_spinner())
        };

        main_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        main_bar.set_message("Initializing...");

        let mut tracker = Self {
            multi,
            main_bar,
            scan_bar: None,
            duplicate_bar: None,
            process_bar: None,
            files_scanned: Arc::new(AtomicUsize::new(0)),
            files_hashed: Arc::new(AtomicUsize::new(0)),
            files_organized: Arc::new(AtomicUsize::new(0)),
            bytes_processed: Arc::new(AtomicU64::new(0)),
            duplicates_found: Arc::new(AtomicUsize::new(0)),
            errors_count: Arc::new(AtomicUsize::new(0)),
            start_time: Instant::now(),
            phase_times: Arc::new(Mutex::new(HashMap::new())),
            metrics_handle: None,
            peak_memory: Arc::new(AtomicU64::new(0)),
            peak_cpu: Arc::new(AtomicU64::new(0)),
        };

        if !quiet {
            tracker.start_metrics_display();
            tracker.start_performance_monitoring();
        }

        tracker
    }

    /// Start the scanning phase
    pub fn start_scanning(&mut self, total_estimate: Option<u64>) {
        self.main_bar.set_message("Scanning for media files...");

        let bar = if let Some(total) = total_estimate {
            self.multi.add(ProgressBar::new(total))
        } else {
            self.multi.add(ProgressBar::new_spinner())
        };

        bar.set_style(
            ProgressStyle::default_bar()
                .template("  Scanning: [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        self.scan_bar = Some(bar);
    }

    /// Update scanning progress
    pub fn update_scan(&self, count: u64) {
        if let Some(ref bar) = self.scan_bar {
            bar.set_position(count);
        }
    }

    /// Finish scanning phase
    pub fn finish_scanning(&mut self, total_found: u64) {
        if let Some(bar) = self.scan_bar.take() {
            bar.finish_with_message(format!("Found {total_found} media files"));
        }
        self.main_bar.set_message("Scan complete");
    }

    /// Start the duplicate detection phase
    pub fn start_duplicate_detection(&mut self, total: u64) {
        self.main_bar.set_message("Detecting duplicates...");

        let bar = self.multi.add(ProgressBar::new(total));

        bar.set_style(
            ProgressStyle::default_bar()
                .template("  Duplicates: [{bar:40.yellow/blue}] {pos}/{len} files ({percent}%) [{elapsed_precise}]")
                .unwrap()
                .progress_chars("#>-"),
        );

        self.duplicate_bar = Some(bar);
    }

    /// Update duplicate detection progress
    pub fn update_duplicate_detection(&self, current: u64) {
        if let Some(ref bar) = self.duplicate_bar {
            bar.set_position(current);
        }
    }

    /// Finish duplicate detection phase
    pub fn finish_duplicate_detection(&mut self) {
        if let Some(bar) = self.duplicate_bar.take() {
            bar.finish_with_message("Duplicate detection complete");
        }
        self.main_bar.set_message("Duplicate detection complete");
    }

    /// Start the processing phase
    pub fn start_processing(&mut self, total: u64) {
        self.main_bar.set_message("Processing media files...");

        let bar = self.multi.add(ProgressBar::new(total));

        bar.set_style(
            ProgressStyle::default_bar()
                .template("  Processing: [{bar:40.green/blue}] {pos}/{len} files ({percent}%) [{elapsed_precise}]")
                .unwrap()
                .progress_chars("#>-"),
        );

        self.process_bar = Some(bar);
    }

    /// Update processing progress
    pub fn update_process(&self, current: u64, message: Option<&str>) {
        if let Some(ref bar) = self.process_bar {
            bar.set_position(current);
            if let Some(msg) = message {
                bar.set_message(msg.to_string());
            }
        }
    }

    /// Report an error during processing
    pub fn report_error(&self, error: &str) {
        if let Some(ref bar) = self.process_bar {
            bar.println(format!("  ❌ Error: {error}"));
        }
    }

    /// Report a skipped file
    #[allow(dead_code)]
    pub fn report_skip(&self, reason: &str) {
        if let Some(ref bar) = self.process_bar {
            bar.println(format!("  ⏭️  Skipped: {reason}"));
        }
    }

    /// Report a successful operation
    pub fn report_success(&self, message: &str) {
        if let Some(ref bar) = self.process_bar {
            bar.println(format!("  ✅ {message}"));
        }
    }

    /// Finish processing phase
    pub fn finish_processing(&mut self) {
        if let Some(bar) = self.process_bar.take() {
            bar.finish_with_message("Processing complete");
        }
    }

    /// Complete all progress tracking
    pub fn finish(&self, summary: &str) {
        self.main_bar.finish_with_message(summary.to_string());
    }

    /// Enable steady tick for spinner animations
    pub fn enable_steady_tick(&self) {
        self.main_bar.enable_steady_tick(Duration::from_millis(100));

        if let Some(ref bar) = self.scan_bar {
            bar.enable_steady_tick(Duration::from_millis(100));
        }

        if let Some(ref bar) = self.process_bar {
            bar.enable_steady_tick(Duration::from_millis(100));
        }
    }

    /// Start real-time metrics display
    fn start_metrics_display(&mut self) {
        let multi = self.multi.clone();
        let files_scanned = self.files_scanned.clone();
        let files_hashed = self.files_hashed.clone();
        let files_organized = self.files_organized.clone();
        let bytes_processed = self.bytes_processed.clone();
        let duplicates = self.duplicates_found.clone();
        let errors = self.errors_count.clone();
        let start_time = self.start_time;
        let peak_memory = self.peak_memory.clone();
        let peak_cpu = self.peak_cpu.clone();

        // Create metrics display bar
        let metrics_bar = multi.add(ProgressBar::new_spinner());
        metrics_bar.set_style(
            ProgressStyle::default_spinner()
                .template("📊 {msg}")
                .unwrap(),
        );

        self.metrics_handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            let mut last_files = 0usize;
            let mut last_bytes = 0u64;
            let mut last_time = Instant::now();

            loop {
                interval.tick().await;

                let current_files = files_scanned.load(Ordering::Relaxed)
                    + files_hashed.load(Ordering::Relaxed)
                    + files_organized.load(Ordering::Relaxed);
                let current_bytes = bytes_processed.load(Ordering::Relaxed);
                let current_time = Instant::now();
                let elapsed = current_time.duration_since(last_time).as_secs_f64();

                if elapsed > 0.0 {
                    let files_per_sec = (current_files - last_files) as f64 / elapsed;
                    let mb_per_sec = (current_bytes - last_bytes) as f64 / 1_048_576.0 / elapsed;

                    let mem_mb = peak_memory.load(Ordering::Relaxed) as f64 / 1_048_576.0;
                    let cpu = peak_cpu.load(Ordering::Relaxed) as f64 / 100.0;

                    let duplicates_count = duplicates.load(Ordering::Relaxed);
                    let errors_count = errors.load(Ordering::Relaxed);
                    let total_elapsed = start_time.elapsed();

                    let msg = format!(
                        "Speed: {:.1} files/s | {:.1} MB/s | Mem: {:.0} MB | CPU: {:.0}% | Dups: {} | Errs: {} | Time: {}",
                        files_per_sec,
                        mb_per_sec,
                        mem_mb,
                        cpu,
                        duplicates_count,
                        errors_count,
                        format_duration(total_elapsed)
                    );

                    metrics_bar.set_message(msg);

                    last_files = current_files;
                    last_bytes = current_bytes;
                    last_time = current_time;
                }
            }
        }));
    }

    /// Start performance monitoring
    fn start_performance_monitoring(&mut self) {
        let peak_memory = self.peak_memory.clone();
        let peak_cpu = self.peak_cpu.clone();

        tokio::spawn(async move {
            let mut system = System::new_all();
            let pid = Pid::from(std::process::id() as usize);

            loop {
                system.refresh_processes();

                if let Some(process) = system.process(pid) {
                    let memory = process.memory();
                    let cpu = (process.cpu_usage() * 100.0) as u64;

                    // Update peak values
                    peak_memory.fetch_max(memory, Ordering::Relaxed);
                    peak_cpu.fetch_max(cpu, Ordering::Relaxed);
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Update metric counters
    pub fn increment_files_scanned(&self, count: usize) {
        self.files_scanned.fetch_add(count, Ordering::Relaxed);
    }

    pub fn increment_files_hashed(&self, count: usize) {
        self.files_hashed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn increment_files_organized(&self, count: usize) {
        self.files_organized.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_bytes_processed(&self, bytes: u64) {
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn increment_duplicates(&self, count: usize) {
        self.duplicates_found.fetch_add(count, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.errors_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record phase timing
    #[allow(dead_code)]
    pub fn record_phase_time(&self, phase: &str, duration: Duration) {
        if let Ok(mut times) = self.phase_times.lock() {
            times.insert(phase.to_string(), duration);
        }
    }

    /// Generate final report
    pub fn generate_report(&self) -> ProgressReport {
        let elapsed = self.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();

        let files_scanned = self.files_scanned.load(Ordering::Relaxed);
        let files_hashed = self.files_hashed.load(Ordering::Relaxed);
        let files_organized = self.files_organized.load(Ordering::Relaxed);
        let total_files = files_scanned.max(files_hashed).max(files_organized);

        let scan_rate = if elapsed_secs > 0.0 {
            files_scanned as f64 / elapsed_secs
        } else {
            0.0
        };
        let hash_rate = if elapsed_secs > 0.0 {
            self.bytes_processed.load(Ordering::Relaxed) as f64 / 1_048_576.0 / elapsed_secs
        } else {
            0.0
        };
        let organize_rate = if elapsed_secs > 0.0 {
            files_organized as f64 / elapsed_secs
        } else {
            0.0
        };

        ProgressReport {
            total_files,
            processed_files: files_organized,
            total_size: self.bytes_processed.load(Ordering::Relaxed),
            duplicates_found: self.duplicates_found.load(Ordering::Relaxed),
            space_saved: 0, // TODO: Calculate from duplicate sizes
            errors: vec![], // TODO: Collect actual error messages
            performance_metrics: PerformanceMetrics {
                scan_rate,
                hash_rate,
                organize_rate,
                peak_memory: self.peak_memory.load(Ordering::Relaxed),
                peak_cpu: self.peak_cpu.load(Ordering::Relaxed) as f64 / 100.0,
            },
            elapsed_time: elapsed,
        }
    }

    /// Print summary report
    pub fn print_summary(&self) {
        let report = self.generate_report();

        println!("\n📊 Operation Summary");
        println!("══════════════════════════════════════");
        println!("✅ Files processed: {}", report.processed_files);
        println!("📁 Total size: {}", format_bytes(report.total_size));
        println!("🔍 Duplicates found: {}", report.duplicates_found);
        if report.space_saved > 0 {
            println!("💾 Space saved: {}", format_bytes(report.space_saved));
        }
        println!("⏱️  Time elapsed: {}", format_duration(report.elapsed_time));
        println!("⚡ Performance:");
        println!(
            "   ├─ Scan rate: {:.1} files/sec",
            report.performance_metrics.scan_rate
        );
        println!(
            "   ├─ Hash rate: {:.1} MB/sec",
            report.performance_metrics.hash_rate
        );
        println!(
            "   ├─ Organize rate: {:.1} files/sec",
            report.performance_metrics.organize_rate
        );
        println!(
            "   ├─ Peak memory: {}",
            format_bytes(report.performance_metrics.peak_memory)
        );
        println!(
            "   └─ Peak CPU: {:.1}%",
            report.performance_metrics.peak_cpu * 100.0
        );

        if !report.errors.is_empty() {
            println!("\n⚠️  Errors encountered: {}", report.errors.len());
            for (i, error) in report.errors.iter().take(5).enumerate() {
                println!("   {}. {}", i + 1, error);
            }
            if report.errors.len() > 5 {
                println!("   ... and {} more", report.errors.len() - 5);
            }
        }
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if let Some(handle) = self.metrics_handle.take() {
            handle.abort();
        }
    }
}

/// Output the report as JSON
pub fn report_as_json(report: &ProgressReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Helper to format bytes into human readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f64 = bytes as f64;
    let exponent = (bytes_f64.ln() / 1024_f64.ln()).floor() as usize;
    let exponent = exponent.min(UNITS.len() - 1);
    let value = bytes_f64 / 1024_f64.powi(exponent as i32);

    if exponent == 0 {
        format!("{} {}", bytes, UNITS[exponent])
    } else {
        format!("{:.2} {}", value, UNITS[exponent])
    }
}

/// Helper to format duration into human readable format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{mins}m {secs}s")
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{hours}h {mins}m")
    }
}

/// Set up structured logging with tracing
pub fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let is_terminal = atty::is(atty::Stream::Stdout);

    if is_terminal {
        // Terminal output with colors and formatting
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_thread_ids(false)
            .with_level(true)
            .init();
    } else {
        // Non-terminal output (CI/CD) - simple format without ANSI codes
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_ansi(false)
            .with_target(true)
            .with_thread_ids(true)
            .with_level(true)
            .init();
    }

    info!("Media organizer initialized");
    debug!("Debug logging enabled");
}
