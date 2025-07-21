use crate::cli::DuplicateStrategy;
use crate::database::HashDatabase;
use crate::progress::ProgressTracker;
use crate::types::FileInfo;
use anyhow::Result;
use blake3::Hasher;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, info, warn};

/// Duplicate detector using BLAKE3 hashing with persistent database
pub struct DuplicateDetector {
    strategy: DuplicateStrategy,
    /// In-memory hash cache for current session
    hash_cache: HashMap<PathBuf, String>,
    /// Duplicates found in current batch
    duplicates: HashMap<String, Vec<PathBuf>>,
    /// Hashes of files that already exist in the output directory
    existing_hashes: HashMap<String, PathBuf>,
    /// Persistent database of file hashes
    database: HashDatabase,
    /// Output directory path
    output_dir: PathBuf,
    /// Whether database needs saving
    database_modified: bool,
    /// Number of parallel workers for hashing (None = use all CPU cores)
    hash_workers: Option<usize>,
    /// Space saved by skipping duplicates (in bytes)
    space_saved: u64,
}

impl DuplicateDetector {
    pub fn new(strategy: DuplicateStrategy) -> Self {
        Self {
            strategy,
            hash_cache: HashMap::new(),
            duplicates: HashMap::new(),
            existing_hashes: HashMap::new(),
            database: HashDatabase::new(),
            output_dir: PathBuf::new(),
            database_modified: false,
            hash_workers: None, // Use all CPU cores by default
            space_saved: 0,
        }
    }

    /// Set the number of parallel workers for hashing
    pub fn with_hash_workers(mut self, workers: usize) -> Self {
        self.hash_workers = Some(workers);
        self
    }

    /// Configure the rayon thread pool for hashing operations
    fn configure_thread_pool(&self) {
        if let Some(workers) = self.hash_workers {
            rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build_global()
                .ok(); // Ignore error if already configured
        }
    }

    /// Initialize with database from output directory
    pub async fn with_database(strategy: DuplicateStrategy, output_dir: &Path) -> Result<Self> {
        info!("Loading hash database from output directory");
        
        let database = HashDatabase::load(output_dir).await?;
        
        Ok(Self {
            strategy,
            hash_cache: HashMap::new(),
            duplicates: HashMap::new(),
            existing_hashes: HashMap::new(),
            database,
            output_dir: output_dir.to_path_buf(),
            database_modified: false,
            hash_workers: None,
            space_saved: 0,
        })
    }
    
    /// Initialize with database from output directory, with progress tracking
    pub async fn with_database_and_progress(strategy: DuplicateStrategy, output_dir: &Path, progress: &ProgressTracker) -> Result<Self> {
        info!("Loading hash database from output directory");
        
        progress.report_success("Loading duplicate database...");
        let database = HashDatabase::load(output_dir).await?;
        
        let stats = database.stats();
        progress.report_success(&format!("Loaded database with {} entries ({} unique hashes)", 
            stats.total_files, stats.unique_hashes));
        
        Ok(Self {
            strategy,
            hash_cache: HashMap::new(),
            duplicates: HashMap::new(),
            existing_hashes: HashMap::new(),
            database,
            output_dir: output_dir.to_path_buf(),
            database_modified: false,
            hash_workers: None,
            space_saved: 0,
        })
    }

    /// Pre-scan output directory to build hash index of existing files with progress tracking
    pub async fn scan_output_directory_with_progress(&mut self, output_dir: &Path, file_types: &[crate::types::FileType], progress: Option<&ProgressTracker>) -> Result<()> {
        use crate::scanner::Scanner;
        use tokio::sync::mpsc;
        
        info!("Pre-scanning output directory for existing files: {:?}", output_dir);
        
        self.output_dir = output_dir.to_path_buf();
        
        // First, try to load existing database
        let _db_loaded = match HashDatabase::load(output_dir).await {
            Ok(db) => {
                info!("Loaded existing database with {} entries", db.stats().total_files);
                
                if let Some(p) = progress {
                    p.report_success(&format!("Loaded existing database with {} entries", db.stats().total_files));
                }
                
                self.database = db;
                
                // Clean up obsolete entries
                let removed = self.database.cleanup().await?;
                if removed > 0 {
                    self.database_modified = true;
                    if let Some(p) = progress {
                        p.report_success(&format!("Cleaned up {removed} obsolete entries"));
                    }
                }
                
                // Populate existing_hashes from database
                let stats = self.database.stats();
                for (hash, paths) in self.database.hash_index.iter() {
                    if let Some(first_path) = paths.first() {
                        self.existing_hashes.insert(hash.clone(), first_path.clone());
                    }
                }
                
                info!("Database contains {} unique hashes, {} duplicates in {} groups", 
                      stats.unique_hashes, stats.total_duplicates, stats.duplicate_groups);
                true
            }
            Err(e) => {
                debug!("Could not load database ({}), will scan files", e);
                
                if let Some(p) = progress {
                    p.report_success("No existing database found, will create new one");
                }
                
                if !output_dir.exists() {
                    debug!("Output directory does not exist yet, skipping pre-scan");
                    return Ok(());
                }
                false
            }
        };
        
        // Create progress bar for hash computation
        let hash_progress = if let Some(_p) = progress {
            let bar = ProgressBar::new_spinner();
            bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg} [{elapsed_precise}]")
                    .unwrap()
            );
            bar.set_message("Scanning for new/modified files...");
            Some(bar)
        } else {
            None
        };
        
        // Scan for new files not in database
        let scanner = Scanner::new(output_dir.to_path_buf())
            .with_file_types(file_types.to_vec())
            .with_batch_size(1000);
        
        let (tx, mut rx) = mpsc::channel(1000);
        
        // Start scanning in background
        let scan_handle = tokio::spawn(async move {
            scanner.scan(tx).await
        });
        
        let mut count = 0;
        let mut new_count = 0;
        let mut files_to_hash = Vec::new();
        
        // First pass: collect files that need hashing
        while let Some(file_info) = rx.recv().await {
            count += 1;
            
            // Check if file is already in database with current metadata
            let needs_rehash = if let Some(entry) = self.database.get(&file_info.path) {
                // Check if file was modified since last hash
                let file_modified = file_info.modified.timestamp();
                file_modified != entry.modified || file_info.size != entry.size
            } else {
                true
            };
            
            if needs_rehash {
                files_to_hash.push(file_info);
            } else {
                // Use hash from database
                if let Some(entry) = self.database.get(&file_info.path) {
                    self.existing_hashes.insert(entry.hash.clone(), file_info.path);
                }
            }
        }
        
        scan_handle.await??;
        
        // Update progress bar for hashing phase
        if let Some(bar) = &hash_progress {
            if !files_to_hash.is_empty() {
                bar.set_length(files_to_hash.len() as u64);
                bar.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} Hashing files: {bar:40.cyan/blue} {pos}/{len} [{elapsed_precise}]")
                        .unwrap()
                        .progress_chars("=>-")
                );
            }
        }
        
        // Second pass: hash the files that need it (in parallel)
        if !files_to_hash.is_empty() {
            let progress_arc = hash_progress.as_ref().map(|bar| Arc::new(bar.clone()));
            let hash_results = self.compute_hashes_parallel(files_to_hash, progress_arc);
            
            for (file_info, hash_result) in hash_results {
                match hash_result {
                    Ok(hash) => {
                        self.existing_hashes.insert(hash, file_info.path);
                        new_count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to hash existing file {:?}: {}", file_info.path, e);
                    }
                }
            }
        }
        
        if let Some(bar) = hash_progress {
            bar.finish_with_message(format!("Pre-scan complete: {count} files processed, {new_count} new/modified files hashed"));
        }
        
        info!("Pre-scan complete: {} files processed, {} new/modified files hashed", count, new_count);
        
        // Save database if modified
        if self.database_modified {
            if let Some(p) = progress {
                p.report_success("Saving updated database...");
            }
            self.save_database().await?;
            if let Some(p) = progress {
                p.report_success(&format!("Database saved with {} entries", self.database.stats().total_files));
            }
        }
        
        Ok(())
    }
    
    /// Pre-scan output directory to build hash index of existing files
    pub async fn scan_output_directory(&mut self, output_dir: &Path, file_types: &[crate::types::FileType]) -> Result<()> {
        self.scan_output_directory_with_progress(output_dir, file_types, None).await
    }

    /// Compute hash for a file (async version for single files)
    pub async fn compute_hash(&mut self, path: &Path) -> Result<String> {
        // Check in-memory cache first
        if let Some(hash) = self.hash_cache.get(path) {
            return Ok(hash.clone());
        }

        // Check database cache
        if let Some(entry) = self.database.get(path) {
            // Verify file hasn't changed
            let metadata = tokio::fs::metadata(path).await?;
            let modified = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs() as i64;
            let size = metadata.len();
            
            if modified == entry.modified && size == entry.size {
                self.hash_cache.insert(path.to_path_buf(), entry.hash.clone());
                return Ok(entry.hash.clone());
            }
        }

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
        self.hash_cache.insert(path.to_path_buf(), hash.clone());

        Ok(hash)
    }

    /// Compute hash for a file (sync version for parallel processing)
    fn compute_hash_sync(path: &Path) -> Result<String> {
        use std::fs::File;
        use std::io::{BufReader, Read};
        
        debug!("Computing hash for {:?}", path);
        
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Hasher::new();
        
        // Use larger buffer for better performance
        let mut buffer = vec![0u8; 256 * 1024]; // 256KB buffer
        
        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute hashes for multiple files in parallel
    pub fn compute_hashes_parallel(&mut self, files: Vec<FileInfo>, progress_bar: Option<Arc<ProgressBar>>) -> Vec<(FileInfo, Result<String>)> {
        // Configure thread pool if needed
        self.configure_thread_pool();
        
        // Filter out files already in cache
        let (cached, to_compute): (Vec<_>, Vec<_>) = files.into_iter()
            .partition(|file_info| {
                // Check in-memory cache
                if self.hash_cache.contains_key(&file_info.path) {
                    return true;
                }
                
                // Check database cache
                if let Some(entry) = self.database.get(&file_info.path) {
                    // Quick check without async metadata
                    if file_info.size == entry.size {
                        return true;
                    }
                }
                
                false
            });
        
        // Get cached results
        let mut results: Vec<(FileInfo, Result<String>)> = cached.into_iter()
            .map(|file_info| {
                let hash = self.hash_cache.get(&file_info.path)
                    .cloned()
                    .or_else(|| self.database.get(&file_info.path).map(|e| e.hash.clone()))
                    .ok_or_else(|| anyhow::anyhow!("Cache miss"));
                (file_info, hash)
            })
            .collect();
        
        // Compute remaining hashes in parallel
        let computed: Vec<(FileInfo, Result<String>)> = to_compute
            .into_par_iter()
            .map(|file_info| {
                let hash_result = Self::compute_hash_sync(&file_info.path);
                
                // Update progress bar if provided
                if let Some(ref bar) = progress_bar {
                    bar.inc(1);
                }
                
                (file_info, hash_result)
            })
            .collect();
        
        // Update cache with computed hashes
        for (file_info, hash_result) in &computed {
            if let Ok(hash) = hash_result {
                self.hash_cache.insert(file_info.path.clone(), hash.clone());
                
                // Update database
                self.database.insert(
                    file_info.path.clone(),
                    hash.clone(),
                    file_info.size,
                    file_info.modified.timestamp(),
                );
                self.database_modified = true;
            }
        }
        
        results.extend(computed);
        results
    }

    /// Check if a file is a duplicate and handle according to strategy
    pub async fn check_duplicate(&mut self, file_info: &mut FileInfo) -> Result<bool> {
        let hash = self.compute_hash(&file_info.path).await?;
        file_info.hash = Some(hash.clone());

        // Update database with this file's hash
        self.database.insert(
            file_info.path.clone(),
            hash.clone(),
            file_info.size,
            file_info.modified.timestamp(),
        );
        self.database_modified = true;

        // First check if this file already exists in the output directory
        if let Some(existing_path) = self.existing_hashes.get(&hash) {
            info!("File already exists in output directory: {:?} (original: {:?})", 
                  existing_path, file_info.path);
            // Always skip files that already exist in output
            self.space_saved += file_info.size;
            return Ok(true);
        }

        // Then check for duplicates within the current batch
        if let Some(existing_paths) = self.duplicates.get_mut(&hash) {
            // This is a duplicate within the current batch
            info!("Duplicate found: {:?}", file_info.path);
            existing_paths.push(file_info.path.clone());

            match self.strategy {
                DuplicateStrategy::Skip => {
                    self.space_saved += file_info.size;
                    Ok(true)
                },
                DuplicateStrategy::Rename => Ok(false), // Will be handled by organizer
                DuplicateStrategy::Replace => Ok(false), // Will be handled by organizer
            }
        } else {
            // First occurrence of this file in the current batch
            self.duplicates.insert(hash, vec![file_info.path.clone()]);
            Ok(false)
        }
    }

    /// Save the database to disk
    pub async fn save_database(&self) -> Result<()> {
        if !self.output_dir.as_os_str().is_empty() {
            self.database.save(&self.output_dir).await?;
        }
        Ok(())
    }

    /// Get a renamed path for a duplicate file
    pub fn get_renamed_path(&self, _original: &Path, destination: &Path) -> PathBuf {
        let parent = destination.parent().unwrap_or_else(|| Path::new(""));
        let stem = destination
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let extension = destination
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut counter = 1;
        loop {
            let new_name = if extension.is_empty() {
                format!("{stem}_{counter}")
            } else {
                format!("{stem}_{counter}.{extension}")
            };

            let new_path = parent.join(&new_name);

            // Check if this path already exists in our tracking
            if !self.is_path_known(&new_path) {
                return new_path;
            }

            counter += 1;
        }
    }

    /// Check if a path is already known (either original or as a duplicate)
    fn is_path_known(&self, path: &Path) -> bool {
        self.hash_cache.contains_key(path)
            || self
                .duplicates
                .values()
                .any(|paths| paths.iter().any(|p| p == path))
    }

    /// Get statistics about duplicates found
    pub fn get_statistics(&self) -> DuplicateStatistics {
        let mut stats = DuplicateStatistics::default();

        for paths in self.duplicates.values() {
            if paths.len() > 1 {
                stats.duplicate_groups += 1;
                stats.total_duplicates += paths.len() - 1; // Don't count the original
            }
        }

        stats.unique_files = self.duplicates.len();
        stats.existing_in_output = self.existing_hashes.len();
        
        // Add database statistics
        let db_stats = self.database.stats();
        stats.database_entries = db_stats.total_files;
        stats.database_duplicates = db_stats.total_duplicates;
        stats.space_saved = self.space_saved;
        
        stats
    }
}

#[derive(Debug, Default)]
pub struct DuplicateStatistics {
    pub unique_files: usize,
    pub duplicate_groups: usize,
    pub total_duplicates: usize,
    pub existing_in_output: usize,
    pub database_entries: usize,
    pub database_duplicates: usize,
    pub space_saved: u64,  // Bytes saved by skipping duplicates
}

// Implement Drop to save database when detector is dropped
impl Drop for DuplicateDetector {
    fn drop(&mut self) {
        if self.database_modified && !self.output_dir.as_os_str().is_empty() {
            // Use blocking I/O in drop
            if let Err(e) = std::fs::create_dir_all(&self.output_dir) {
                warn!("Failed to create output directory for database: {}", e);
                return;
            }
            
            // Convert to blocking save
            let db_data = match bincode::serialize(&self.database) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to serialize database: {}", e);
                    return;
                }
            };
            
            let compressed = match zstd::encode_all(&db_data[..], 3) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to compress database: {}", e);
                    return;
                }
            };
            
            let db_path = self.output_dir.join("db.mediaorg");
            if let Err(e) = std::fs::write(&db_path, compressed) {
                warn!("Failed to save database: {}", e);
            } else {
                debug!("Database saved to {:?}", db_path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_hash_computation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"Hello, world!").await.unwrap();

        let mut detector = DuplicateDetector::new(DuplicateStrategy::Skip);
        let hash = detector.compute_hash(&file_path).await.unwrap();

        // BLAKE3 hash of "Hello, world!"
        assert_eq!(hash.len(), 64); // BLAKE3 produces 32-byte hashes (64 hex chars)
    }

    #[tokio::test]
    async fn test_cross_directory_duplicate_detection() {
        use crate::types::{FileType, MediaMetadata};
        use chrono::Local;
        
        let dir = tempdir().unwrap();
        let output_dir = dir.path().join("output");
        let input_dir = dir.path().join("input");
        
        fs::create_dir_all(&output_dir).await.unwrap();
        fs::create_dir_all(&input_dir).await.unwrap();
        
        // Create an existing file in output directory
        let existing_file = output_dir.join("existing.jpg");
        fs::write(&existing_file, b"existing content").await.unwrap();
        
        // Create a duplicate file in input directory
        let duplicate_file = input_dir.join("duplicate.jpg");
        fs::write(&duplicate_file, b"existing content").await.unwrap();
        
        // Create a new file in input directory
        let new_file = input_dir.join("new.jpg");
        fs::write(&new_file, b"new content").await.unwrap();
        
        let mut detector = DuplicateDetector::new(DuplicateStrategy::Skip);
        
        // Pre-scan output directory - using all image types
        let image_types = vec![
            FileType::Jpeg, FileType::Png, FileType::Heic, FileType::Raw,
            FileType::Gif, FileType::Bmp, FileType::Tiff, FileType::Webp
        ];
        detector.scan_output_directory(&output_dir, &image_types).await.unwrap();
        
        // Check that existing files are detected
        let stats = detector.get_statistics();
        assert_eq!(stats.existing_in_output, 1);
        
        // Test duplicate detection
        let mut duplicate_info = crate::types::FileInfo {
            path: duplicate_file.clone(),
            file_type: FileType::Jpeg,
            size: 16,
            modified: Local::now(),
            created: Some(Local::now()),
            hash: None,
            metadata: MediaMetadata::default(),
        };
        
        let should_skip = detector.check_duplicate(&mut duplicate_info).await.unwrap();
        assert!(should_skip, "Duplicate of existing file should be skipped");
        
        // Test new file detection
        let mut new_info = crate::types::FileInfo {
            path: new_file.clone(),
            file_type: FileType::Jpeg,
            size: 11,
            modified: Local::now(),
            created: Some(Local::now()),
            hash: None,
            metadata: MediaMetadata::default(),
        };
        
        let should_skip = detector.check_duplicate(&mut new_info).await.unwrap();
        assert!(!should_skip, "New file should not be skipped");
    }

    #[tokio::test]
    async fn test_database_persistence() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path().join("output");
        fs::create_dir_all(&output_dir).await.unwrap();
        
        let file1 = dir.path().join("file1.jpg");
        fs::write(&file1, b"content1").await.unwrap();
        
        // First run - compute and save hash
        {
            let mut detector = DuplicateDetector::with_database(
                DuplicateStrategy::Skip, 
                &output_dir
            ).await.unwrap();
            
            let hash = detector.compute_hash(&file1).await.unwrap();
            assert!(!hash.is_empty());
            
            // Force save
            detector.save_database().await.unwrap();
        }
        
        // Second run - should load from database
        {
            let mut detector = DuplicateDetector::with_database(
                DuplicateStrategy::Skip, 
                &output_dir
            ).await.unwrap();
            
            // This should use cached hash
            let hash = detector.compute_hash(&file1).await.unwrap();
            assert!(!hash.is_empty());
        }
    }
}