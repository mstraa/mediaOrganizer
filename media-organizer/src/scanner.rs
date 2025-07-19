use crate::types::{FileInfo, FileType};
use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use walkdir::WalkDir;

/// Scanner for finding and analyzing media files
pub struct Scanner {
    input_dir: PathBuf,
    file_types: Option<Vec<FileType>>,
    exclude_patterns: Vec<String>,
    min_size: u64,
    max_size: Option<u64>,
    follow_links: bool,
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
    
    /// Scan the input directory and send file information through the channel
    pub async fn scan(&self, tx: mpsc::Sender<FileInfo>) -> Result<()> {
        let walker = WalkDir::new(&self.input_dir)
            .follow_links(self.follow_links)
            .into_iter()
            .filter_entry(|entry| self.should_process_entry(entry));
        
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Error accessing path: {}", e);
                    continue;
                }
            };
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            let path = entry.path();
            
            // Check file type
            let file_type = FileType::from_extension(path);
            if file_type == FileType::Unknown {
                continue;
            }
            
            if let Some(ref types) = self.file_types {
                if !types.contains(&file_type) {
                    continue;
                }
            }
            
            // Check file size
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to get metadata for {:?}: {}", path, e);
                    continue;
                }
            };
            
            let size = metadata.len();
            if size < self.min_size {
                continue;
            }
            
            if let Some(max) = self.max_size {
                if size > max {
                    continue;
                }
            }
            
            // Create FileInfo
            match self.create_file_info(path, file_type, &metadata).await {
                Ok(info) => {
                    debug!("Found file: {:?}", info.path);
                    if tx.send(info).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(e) => {
                    warn!("Failed to process file {:?}: {}", path, e);
                }
            }
        }
        
        Ok(())
    }
    
    fn should_process_entry(&self, entry: &walkdir::DirEntry) -> bool {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        
        // Skip hidden files and directories
        if name.starts_with('.') {
            return false;
        }
        
        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if name.contains(pattern) || path.to_string_lossy().contains(pattern) {
                return false;
            }
        }
        
        true
    }
    
    async fn create_file_info(
        &self,
        path: &Path,
        file_type: FileType,
        metadata: &std::fs::Metadata,
    ) -> Result<FileInfo> {
        use chrono::{DateTime, Local};
        
        let modified = metadata.modified()?;
        let modified = DateTime::<Local>::from(modified);
        
        let created = metadata.created().ok().map(DateTime::<Local>::from);
        
        Ok(FileInfo {
            path: path.to_path_buf(),
            file_type,
            size: metadata.len(),
            modified,
            created,
            hash: None, // Will be computed later if duplicate detection is enabled
            metadata: Default::default(), // Will be populated by metadata extractor
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_type_detection() {
        let jpg_path = Path::new("test.jpg");
        assert_eq!(FileType::from_extension(jpg_path), FileType::Jpeg);
        
        let mov_path = Path::new("video.MOV");
        assert_eq!(FileType::from_extension(mov_path), FileType::Mov);
        
        let unknown_path = Path::new("document.txt");
        assert_eq!(FileType::from_extension(unknown_path), FileType::Unknown);
    }
}