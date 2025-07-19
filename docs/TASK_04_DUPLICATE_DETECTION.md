# Task 4: Implement Duplicate Detection Module

## Objective
Build a high-performance duplicate detection system using BLAKE3 hashing that can efficiently identify duplicate media files across millions of items while maintaining low memory usage.

## Prerequisites
- Task 3 (File Scanning) completed
- `blake3` crate dependency available
- File streaming infrastructure in place

## Implementation Steps

### 1. Create Duplicate Detector Structure (`src/duplicate.rs`)
```rust
use blake3::Hasher;
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct DuplicateDetector {
    hash_map: Arc<Mutex<HashMap<String, Vec<PathBuf>>>>,
    hash_cache: Arc<DashMap<PathBuf, String>>, // For performance
    partial_hash_size: usize, // For quick pre-filtering
}

pub struct DuplicateGroup {
    pub hash: String,
    pub files: Vec<FileInfo>,
    pub total_size: u64,
    pub keep_suggestion: PathBuf, // Suggested file to keep
}
```

### 2. Implement BLAKE3 Hashing
```rust
async fn calculate_hash(path: &Path) -> Result<String> {
    let mut hasher = Hasher::new();
    let mut file = tokio::fs::File::open(path).await?;
    let mut buffer = vec![0; 8192]; // 8KB buffer
    
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    
    Ok(hasher.finalize().to_hex().to_string())
}
```

### 3. Add Two-Phase Hashing Strategy
**Phase 1: Quick partial hash (first 1MB)**
- Rapidly eliminate obvious non-duplicates
- Group potential duplicates for full hashing

**Phase 2: Full file hash**
- Only for files with matching partial hashes
- Use BLAKE3 for speed (3GB/s on modern CPUs)

### 4. Implement Streaming Duplicate Detection
```rust
pub async fn process_file_stream(
    &self,
    file_stream: mpsc::Receiver<FileInfo>,
    progress: Arc<ProgressTracker>,
) -> mpsc::Receiver<DuplicateGroup> {
    let (tx, rx) = mpsc::channel(100);
    
    tokio::spawn(async move {
        // Process files as they arrive
        // Update hash map incrementally
        // Emit duplicate groups when found
    });
    
    Ok(rx)
}
```

### 5. Smart Duplicate Resolution
**Criteria for keeping files:**
- Prefer files in organized structure
- Keep higher resolution (for images/videos)
- Prefer files with complete metadata
- Consider file naming patterns
- Keep files with better quality

### 6. Memory Management
- Use memory-mapped files for large files
- Implement hash cache with LRU eviction
- Stream results instead of collecting
- Target: <200MB memory for millions of files

### 7. Parallel Processing
```rust
use rayon::prelude::*;

fn process_batch_parallel(files: Vec<FileInfo>) -> Vec<(PathBuf, String)> {
    files.par_iter()
        .map(|file| {
            let hash = calculate_hash(&file.path)?;
            Ok((file.path.clone(), hash))
        })
        .collect()
}
```

## Performance Optimizations
- Skip hashing for unique file sizes
- Use partial hashes for pre-filtering
- Cache hashes for moved files
- Process in parallel batches
- Target: 500MB/s hashing throughput

## Testing Strategy
- Unit tests with known duplicates
- Test various file sizes (1KB to 10GB)
- Verify hash consistency
- Benchmark hashing performance
- Test memory usage under load

## Error Handling
- Handle read errors gracefully
- Skip files being modified
- Report but continue on failures
- Maintain hash consistency

## Success Criteria
- [ ] Correctly identifies all duplicates
- [ ] No false positives in detection
- [ ] Processes 500MB/s or faster
- [ ] Memory usage under 200MB
- [ ] Handles millions of files
- [ ] Smart resolution suggestions work

## Integration Points
- Receives file stream from scanner
- Coordinates with progress tracker
- Passes results to organizer
- Saves duplicate report

## Output Format
```json
{
  "duplicate_groups": [
    {
      "hash": "blake3_hash_here",
      "files": ["path1", "path2"],
      "total_size": 10485760,
      "suggested_keep": "path1",
      "reason": "better_quality"
    }
  ],
  "summary": {
    "total_duplicates": 1234,
    "space_savings": 5368709120,
    "groups_found": 456
  }
}
```

## Next Task
After completing duplicate detection, proceed to Task 5: Implement File Organization Module