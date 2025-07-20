use crate::cli::DuplicateStrategy;
use crate::types::FileInfo;
use anyhow::Result;
use blake3::Hasher;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, info, warn};

/// Duplicate detector using BLAKE3 hashing
pub struct DuplicateDetector {
    strategy: DuplicateStrategy,
    hash_cache: HashMap<PathBuf, String>,
    duplicates: HashMap<String, Vec<PathBuf>>,
    /// Hashes of files that already exist in the output directory
    existing_hashes: HashMap<String, PathBuf>,
}

impl DuplicateDetector {
    pub fn new(strategy: DuplicateStrategy) -> Self {
        Self {
            strategy,
            hash_cache: HashMap::new(),
            duplicates: HashMap::new(),
            existing_hashes: HashMap::new(),
        }
    }

    /// Pre-scan output directory to build hash index of existing files
    pub async fn scan_output_directory(&mut self, output_dir: &Path, file_types: &[crate::types::FileType]) -> Result<()> {
        use crate::scanner::Scanner;
        use tokio::sync::mpsc;
        
        info!("Pre-scanning output directory for existing files: {:?}", output_dir);
        
        if !output_dir.exists() {
            debug!("Output directory does not exist yet, skipping pre-scan");
            return Ok(());
        }
        
        let scanner = Scanner::new(output_dir.to_path_buf())
            .with_file_types(file_types.to_vec())
            .with_batch_size(1000);
        
        let (tx, mut rx) = mpsc::channel(1000);
        
        // Start scanning in background
        let scan_handle = tokio::spawn(async move {
            scanner.scan(tx).await
        });
        
        let mut count = 0;
        while let Some(file_info) = rx.recv().await {
            match self.compute_hash(&file_info.path).await {
                Ok(hash) => {
                    self.existing_hashes.insert(hash, file_info.path);
                    count += 1;
                    if count % 100 == 0 {
                        debug!("Pre-scanned {} existing files", count);
                    }
                }
                Err(e) => {
                    warn!("Failed to hash existing file {:?}: {}", file_info.path, e);
                }
            }
        }
        
        scan_handle.await??;
        info!("Pre-scan complete: found {} existing files in output directory", count);
        
        Ok(())
    }

    /// Compute hash for a file
    pub async fn compute_hash(&mut self, path: &Path) -> Result<String> {
        // Check cache first
        if let Some(hash) = self.hash_cache.get(path) {
            return Ok(hash.clone());
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

    /// Check if a file is a duplicate and handle according to strategy
    pub async fn check_duplicate(&mut self, file_info: &mut FileInfo) -> Result<bool> {
        let hash = self.compute_hash(&file_info.path).await?;
        file_info.hash = Some(hash.clone());

        // First check if this file already exists in the output directory
        if let Some(existing_path) = self.existing_hashes.get(&hash) {
            info!("File already exists in output directory: {:?} (original: {:?})", 
                  existing_path, file_info.path);
            // Always skip files that already exist in output
            return Ok(true);
        }

        // Then check for duplicates within the current batch
        if let Some(existing_paths) = self.duplicates.get_mut(&hash) {
            // This is a duplicate within the current batch
            info!("Duplicate found: {:?}", file_info.path);
            existing_paths.push(file_info.path.clone());

            match self.strategy {
                DuplicateStrategy::Skip => Ok(true),
                DuplicateStrategy::Rename => Ok(false), // Will be handled by organizer
                DuplicateStrategy::Replace => Ok(false), // Will be handled by organizer
            }
        } else {
            // First occurrence of this file in the current batch
            self.duplicates.insert(hash, vec![file_info.path.clone()]);
            Ok(false)
        }
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
        stats
    }
}

#[derive(Debug, Default)]
pub struct DuplicateStatistics {
    pub unique_files: usize,
    pub duplicate_groups: usize,
    pub total_duplicates: usize,
    pub existing_in_output: usize,
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
}
