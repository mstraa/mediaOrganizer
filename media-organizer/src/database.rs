use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

const DB_FILENAME: &str = "db.mediaorg";
const DB_VERSION: u32 = 1;

/// Persistent database for storing file hashes
#[derive(Debug, Serialize, Deserialize)]
pub struct HashDatabase {
    version: u32,
    /// Map of file paths to their BLAKE3 hashes
    pub hashes: HashMap<PathBuf, HashEntry>,
    /// Reverse index: hash -> paths
    pub hash_index: HashMap<String, Vec<PathBuf>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    pub hash: String,
    pub size: u64,
    pub modified: i64, // Unix timestamp
}

impl HashDatabase {
    /// Create a new empty database
    pub fn new() -> Self {
        Self {
            version: DB_VERSION,
            hashes: HashMap::new(),
            hash_index: HashMap::new(),
        }
    }

    /// Load database from output directory
    pub async fn load(output_dir: &Path) -> Result<Self> {
        let db_path = output_dir.join(DB_FILENAME);
        
        if !db_path.exists() {
            debug!("Database file does not exist at {:?}, creating new", db_path);
            return Ok(Self::new());
        }

        info!("Loading hash database from {:?}", db_path);
        
        let mut file = fs::File::open(&db_path).await
            .context("Failed to open database file")?;
        
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await
            .context("Failed to read database file")?;
        
        // Try to decompress first (in case it's compressed)
        let data = match zstd::decode_all(&contents[..]) {
            Ok(decompressed) => decompressed,
            Err(_) => {
                // Not compressed, use as-is
                contents
            }
        };
        
        let db: Self = bincode::deserialize(&data)
            .context("Failed to deserialize database")?;
        
        if db.version != DB_VERSION {
            warn!("Database version mismatch: expected {}, got {}. Creating new database.", 
                  DB_VERSION, db.version);
            return Ok(Self::new());
        }
        
        info!("Loaded {} hashes from database", db.hashes.len());
        Ok(db)
    }

    /// Save database to output directory
    pub async fn save(&self, output_dir: &Path) -> Result<()> {
        let db_path = output_dir.join(DB_FILENAME);
        let temp_path = output_dir.join(format!("{DB_FILENAME}.tmp"));
        
        info!("Saving hash database to {:?}", db_path);
        
        // Serialize to binary
        let data = bincode::serialize(self)
            .context("Failed to serialize database")?;
        
        // Compress with zstd
        let compressed = zstd::encode_all(&data[..], 3)
            .context("Failed to compress database")?;
        
        // Write to temporary file first
        let mut file = fs::File::create(&temp_path).await
            .context("Failed to create temporary database file")?;
        
        file.write_all(&compressed).await
            .context("Failed to write database file")?;
        
        file.sync_all().await
            .context("Failed to sync database file")?;
        
        // Atomically rename to final location
        fs::rename(&temp_path, &db_path).await
            .context("Failed to rename database file")?;
        
        info!("Saved {} hashes to database", self.hashes.len());
        Ok(())
    }

    /// Add or update a hash entry
    pub fn insert(&mut self, path: PathBuf, hash: String, size: u64, modified: i64) {
        // Remove from old hash index if updating
        if let Some(old_entry) = self.hashes.get(&path) {
            if let Some(paths) = self.hash_index.get_mut(&old_entry.hash) {
                paths.retain(|p| p != &path);
                if paths.is_empty() {
                    self.hash_index.remove(&old_entry.hash);
                }
            }
        }
        
        // Insert new entry
        let entry = HashEntry { hash: hash.clone(), size, modified };
        self.hashes.insert(path.clone(), entry);
        
        // Update hash index
        self.hash_index
            .entry(hash)
            .or_default()
            .push(path);
    }

    /// Get hash for a file path
    pub fn get(&self, path: &Path) -> Option<&HashEntry> {
        self.hashes.get(path)
    }

    /// Check if a hash exists in the database
    #[allow(dead_code)]
    pub fn contains_hash(&self, hash: &str) -> bool {
        self.hash_index.contains_key(hash)
    }

    /// Get all paths with a given hash
    #[allow(dead_code)]
    pub fn get_paths_by_hash(&self, hash: &str) -> Option<&Vec<PathBuf>> {
        self.hash_index.get(hash)
    }

    /// Remove entries for paths that no longer exist
    pub async fn cleanup(&mut self) -> Result<usize> {
        let mut removed = 0;
        let mut to_remove = Vec::new();
        
        for path in self.hashes.keys() {
            if !path.exists() {
                to_remove.push(path.clone());
            }
        }
        
        for path in to_remove {
            if let Some(entry) = self.hashes.remove(&path) {
                // Remove from hash index
                if let Some(paths) = self.hash_index.get_mut(&entry.hash) {
                    paths.retain(|p| p != &path);
                    if paths.is_empty() {
                        self.hash_index.remove(&entry.hash);
                    }
                }
                removed += 1;
            }
        }
        
        if removed > 0 {
            info!("Cleaned up {} obsolete entries from database", removed);
        }
        
        Ok(removed)
    }

    /// Get database statistics
    pub fn stats(&self) -> DatabaseStats {
        let unique_hashes = self.hash_index.len();
        let total_files = self.hashes.len();
        let duplicate_groups = self.hash_index.values()
            .filter(|paths| paths.len() > 1)
            .count();
        let total_duplicates = self.hash_index.values()
            .filter(|paths| paths.len() > 1)
            .map(|paths| paths.len() - 1)
            .sum();
        
        DatabaseStats {
            total_files,
            unique_hashes,
            duplicate_groups,
            total_duplicates,
        }
    }
}

#[derive(Debug)]
pub struct DatabaseStats {
    pub total_files: usize,
    pub unique_hashes: usize,
    pub duplicate_groups: usize,
    pub total_duplicates: usize,
}

impl Default for HashDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_database_save_load() {
        let dir = tempdir().unwrap();
        let mut db = HashDatabase::new();
        
        // Add some entries
        db.insert(
            PathBuf::from("/test/file1.jpg"),
            "hash1".to_string(),
            1000,
            1234567890,
        );
        db.insert(
            PathBuf::from("/test/file2.jpg"),
            "hash2".to_string(),
            2000,
            1234567891,
        );
        
        // Save
        db.save(dir.path()).await.unwrap();
        
        // Load
        let loaded = HashDatabase::load(dir.path()).await.unwrap();
        
        assert_eq!(loaded.hashes.len(), 2);
        assert_eq!(loaded.hash_index.len(), 2);
        assert_eq!(loaded.get(Path::new("/test/file1.jpg")).unwrap().hash, "hash1");
    }

    #[tokio::test]
    async fn test_duplicate_tracking() {
        let mut db = HashDatabase::new();
        
        // Add files with same hash
        db.insert(
            PathBuf::from("/test/file1.jpg"),
            "same_hash".to_string(),
            1000,
            1234567890,
        );
        db.insert(
            PathBuf::from("/test/file2.jpg"),
            "same_hash".to_string(),
            1000,
            1234567890,
        );
        
        assert!(db.contains_hash("same_hash"));
        assert_eq!(db.get_paths_by_hash("same_hash").unwrap().len(), 2);
        
        let stats = db.stats();
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.unique_hashes, 1);
        assert_eq!(stats.duplicate_groups, 1);
        assert_eq!(stats.total_duplicates, 1);
    }
}