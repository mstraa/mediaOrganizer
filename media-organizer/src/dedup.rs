use anyhow::Result;
use blake3::Hasher;
use chrono::Local;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, error, info};

use crate::cli::DedupArgs;
use crate::progress::ProgressTracker;
use crate::scanner::Scanner;
use crate::types::FileInfo;

/// Information about a duplicate group
#[derive(Debug)]
struct DuplicateGroup {
    /// The hash identifying this group
    hash: String,
    /// Files with this hash, sorted by creation date (oldest first)
    files: Vec<FileInfo>,
    /// Total size that would be freed by removing duplicates
    space_savings: u64,
}

/// Deduplicator - finds and removes duplicate files
pub struct Deduplicator {
    args: DedupArgs,
    progress: ProgressTracker,
}

impl Deduplicator {
    pub fn new(args: DedupArgs) -> Self {
        let progress = ProgressTracker::new(args.quiet);
        Self { args, progress }
    }

    /// Main entry point for deduplication
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting deduplication process");
        info!("Scanning directory: {}", self.args.directory.display());

        // Phase 1: Scan files
        let files = self.scan_files().await?;
        
        if files.is_empty() {
            self.progress.finish("No media files found");
            return Ok(());
        }

        // Phase 2: Find duplicates
        let duplicate_groups = self.find_duplicates(files).await?;
        
        if duplicate_groups.is_empty() {
            self.progress.finish("No duplicate files found");
            return Ok(());
        }

        // Phase 3: Report findings
        self.report_duplicates(&duplicate_groups);

        // Phase 4: Delete duplicates (if not dry run)
        if !self.args.dry_run {
            self.delete_duplicates(duplicate_groups).await?;
        }

        Ok(())
    }

    /// Scan directory for media files
    async fn scan_files(&mut self) -> Result<Vec<FileInfo>> {
        self.progress.start_scanning(None);
        self.progress.enable_steady_tick();

        let mut scanner = Scanner::new(self.args.directory.clone());

        // Configure scanner
        if let Some(file_types) = self.args.get_file_types() {
            scanner = scanner.with_file_types(file_types);
        }

        if let Some((min, max)) = self.args.get_size_limits() {
            scanner = scanner.with_size_limits(min, max);
        }

        scanner = scanner
            .with_batch_size(1000)
            .with_worker_threads(self.args.get_worker_count())
            .with_exclude_patterns(self.args.exclude.clone())
            .with_follow_links(self.args.follow_links);

        // Create channel for streaming files
        let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

        // Start scanning in background
        let scan_handle = tokio::spawn(async move { scanner.scan(tx).await });

        // Collect files
        let mut files = Vec::new();
        let mut file_count = 0;

        while let Some(file_info) = rx.recv().await {
            file_count += 1;
            self.progress.update_scan(file_count);
            self.progress.increment_files_scanned(1);
            self.progress.add_bytes_processed(file_info.size);

            if self.args.verbose {
                debug!("Found: {:?} ({} bytes)", file_info.path, file_info.size);
            }

            files.push(file_info);
        }

        scan_handle.await??;
        self.progress.finish_scanning(file_count);

        Ok(files)
    }

    /// Find duplicate files by computing hashes
    async fn find_duplicates(&mut self, files: Vec<FileInfo>) -> Result<Vec<DuplicateGroup>> {
        self.progress.start_duplicate_detection(files.len() as u64);

        let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();

        // Hash all files
        for (idx, mut file_info) in files.into_iter().enumerate() {
            self.progress.update_duplicate_detection(idx as u64 + 1);

            match self.compute_hash(&file_info.path).await {
                Ok(hash) => {
                    file_info.hash = Some(hash.clone());
                    self.progress.increment_files_hashed(1);
                    hash_groups.entry(hash).or_insert_with(Vec::new).push(file_info);
                }
                Err(e) => {
                    error!("Failed to hash {:?}: {}", file_info.path, e);
                    self.progress.increment_errors();
                }
            }
        }

        self.progress.finish_duplicate_detection();

        // Convert to duplicate groups, keeping only groups with duplicates
        let mut duplicate_groups = Vec::new();
        
        for (hash, mut files) in hash_groups {
            if files.len() > 1 {
                // Sort by creation date (oldest first), then by modification date if creation is not available
                files.sort_by(|a, b| {
                    let a_date = a.created.as_ref().unwrap_or(&a.modified);
                    let b_date = b.created.as_ref().unwrap_or(&b.modified);
                    a_date.cmp(b_date)
                });

                // Calculate space savings (all files except the oldest)
                let space_savings = files.iter().skip(1).map(|f| f.size).sum();

                duplicate_groups.push(DuplicateGroup {
                    hash,
                    files,
                    space_savings,
                });
            }
        }

        // Sort groups by space savings (largest first)
        duplicate_groups.sort_by(|a, b| b.space_savings.cmp(&a.space_savings));

        Ok(duplicate_groups)
    }

    /// Compute BLAKE3 hash for a file
    async fn compute_hash(&self, path: &Path) -> Result<String> {
        debug!("Computing hash for {:?}", path);

        let file = File::open(path).await?;
        let mut reader = BufReader::new(file);
        let mut hasher = Hasher::new();

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let hash = hasher.finalize().to_hex().to_string();
        Ok(hash)
    }

    /// Report duplicate groups found
    fn report_duplicates(&self, duplicate_groups: &[DuplicateGroup]) {
        let total_groups = duplicate_groups.len();
        let total_duplicates: usize = duplicate_groups.iter()
            .map(|g| g.files.len() - 1)
            .sum();
        let total_space_savings: u64 = duplicate_groups.iter()
            .map(|g| g.space_savings)
            .sum();

        println!("\n📊 Duplicate Detection Summary:");
        println!("================================");
        println!("Duplicate groups found: {}", total_groups);
        println!("Total duplicate files: {}", total_duplicates);
        println!("Space that can be freed: {}", format_size(total_space_savings));

        if self.args.verbose || self.args.dry_run {
            println!("\n📁 Duplicate Groups (oldest file will be kept):");
            println!("------------------------------------------------");

            for (idx, group) in duplicate_groups.iter().enumerate() {
                println!("\nGroup {} (hash: {}...)", idx + 1, &group.hash[..8]);
                println!("Space savings: {}", format_size(group.space_savings));
                
                for (file_idx, file) in group.files.iter().enumerate() {
                    let date = file.created.as_ref().unwrap_or(&file.modified);
                    let status = if file_idx == 0 { "KEEP" } else { "DELETE" };
                    println!("  [{:6}] {} - {} - {}",
                        status,
                        date.format("%Y-%m-%d %H:%M:%S"),
                        format_size(file.size),
                        file.path.display()
                    );
                }
            }
        }

        if self.args.dry_run {
            println!("\n⚠️  DRY RUN MODE - No files will be deleted");
        }
    }

    /// Delete duplicate files (keeping the oldest)
    async fn delete_duplicates(&mut self, duplicate_groups: Vec<DuplicateGroup>) -> Result<()> {
        // Ask for confirmation unless --force is specified
        if !self.args.force {
            let total_files: usize = duplicate_groups.iter()
                .map(|g| g.files.len() - 1)
                .sum();
            let total_space: u64 = duplicate_groups.iter()
                .map(|g| g.space_savings)
                .sum();

            println!("\n⚠️  WARNING: This will delete {} files and free {}",
                total_files, format_size(total_space));
            println!("Are you sure you want to proceed? (yes/no): ");

            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut response = String::new();
            io::stdin().read_line(&mut response)?;

            if !response.trim().eq_ignore_ascii_case("yes") {
                println!("Operation cancelled");
                return Ok(());
            }
        }

        // Start deletion
        self.progress.start_processing(
            duplicate_groups.iter().map(|g| g.files.len() - 1).sum::<usize>() as u64
        );

        let mut deleted_count = 0;
        let mut error_count = 0;
        let mut freed_space = 0u64;
        let mut deleted_files = Vec::new();

        for group in duplicate_groups {
            // Skip the first file (oldest) - that's the one we keep
            for file in group.files.into_iter().skip(1) {
                self.progress.update_process(
                    deleted_count + error_count + 1,
                    Some(&file.path.display().to_string())
                );

                match fs::remove_file(&file.path).await {
                    Ok(_) => {
                        deleted_count += 1;
                        freed_space += file.size;
                        deleted_files.push(file.path.clone());
                        self.progress.increment_files_organized(1);
                        
                        if self.args.verbose {
                            info!("Deleted: {}", file.path.display());
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        self.progress.increment_errors();
                        error!("Failed to delete {}: {}", file.path.display(), e);
                    }
                }
            }
        }

        self.progress.finish_processing();

        // Save deleted files list if requested
        if let Some(save_path) = &self.args.save_list {
            self.save_deleted_list(&deleted_files, save_path).await?;
        }

        // Final summary
        let summary = format!(
            "Deduplication complete: {} files deleted, {} errors, {} freed",
            deleted_count, error_count, format_size(freed_space)
        );
        self.progress.finish(&summary);
        self.progress.print_summary();

        if self.args.json {
            let report = self.progress.generate_report();
            if let Ok(json) = crate::progress::report_as_json(&report) {
                println!("\n{}", json);
            }
        }

        Ok(())
    }

    /// Save list of deleted files to a file
    async fn save_deleted_list(&self, deleted_files: &[PathBuf], save_path: &Path) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut file = fs::File::create(save_path).await?;
        
        for path in deleted_files {
            file.write_all(path.to_string_lossy().as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        info!("Saved list of deleted files to: {}", save_path.display());
        Ok(())
    }
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: f64 = 1024.0;

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }
}