use crate::cli::DuplicateStrategy;
use crate::types::FileInfo;
use anyhow::Result;
use blake3::Hasher;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tracing::{debug, info};

/// Duplicate detector using BLAKE3 hashing
pub struct DuplicateDetector {
    strategy: DuplicateStrategy,
    hash_cache: HashMap<PathBuf, String>,
    duplicates: HashMap<String, Vec<PathBuf>>,
}

impl DuplicateDetector {
    pub fn new(strategy: DuplicateStrategy) -> Self {
        Self {
            strategy,
            hash_cache: HashMap::new(),
            duplicates: HashMap::new(),
        }
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
        
        if let Some(existing_paths) = self.duplicates.get_mut(&hash) {
            // This is a duplicate
            info!("Duplicate found: {:?}", file_info.path);
            existing_paths.push(file_info.path.clone());
            
            match self.strategy {
                DuplicateStrategy::Skip => Ok(true),
                DuplicateStrategy::Rename => Ok(false), // Will be handled by organizer
                DuplicateStrategy::Replace => Ok(false), // Will be handled by organizer
            }
        } else {
            // First occurrence of this file
            self.duplicates.insert(hash, vec![file_info.path.clone()]);
            Ok(false)
        }
    }
    
    /// Get a renamed path for a duplicate file
    pub fn get_renamed_path(&self, _original: &Path, destination: &Path) -> PathBuf {
        let parent = destination.parent().unwrap_or_else(|| Path::new(""));
        let stem = destination.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let extension = destination.extension().and_then(|e| e.to_str()).unwrap_or("");
        
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
        self.hash_cache.contains_key(path) || 
        self.duplicates.values().any(|paths| paths.iter().any(|p| p == path))
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
        stats
    }
}

#[derive(Debug, Default)]
pub struct DuplicateStatistics {
    pub unique_files: usize,
    pub duplicate_groups: usize,
    pub total_duplicates: usize,
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
}