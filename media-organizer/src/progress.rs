use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use std::time::Duration;

/// Progress tracking for media organization operations
pub struct ProgressTracker {
    multi: Arc<MultiProgress>,
    main_bar: ProgressBar,
    scan_bar: Option<ProgressBar>,
    duplicate_bar: Option<ProgressBar>,
    process_bar: Option<ProgressBar>,
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

        Self {
            multi,
            main_bar,
            scan_bar: None,
            duplicate_bar: None,
            process_bar: None,
        }
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
