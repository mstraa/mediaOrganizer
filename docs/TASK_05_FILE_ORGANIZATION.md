# Task 5: Implement File Organization Module

## Objective
Create a file organization module that efficiently moves or copies media files into a date-based directory structure, handles naming conflicts, preserves metadata, and provides detailed operation logging.

## Prerequisites
- Task 4 (Duplicate Detection) completed
- File scanning and duplicate detection working
- Basic file operations available via `tokio::fs`

## Implementation Steps

### 1. Define Organization Structure (`src/organizer.rs`)
```rust
use chrono::{DateTime, Utc, Datelike};
use std::path::{Path, PathBuf};

pub struct Organizer {
    output_dir: PathBuf,
    operation_mode: OperationMode,
    conflict_strategy: ConflictStrategy,
    preserve_metadata: bool,
}

#[derive(Debug, Clone)]
pub enum OperationMode {
    Move,
    Copy,
    Hardlink, // macOS supports hardlinks
}

#[derive(Debug, Clone)]
pub enum ConflictStrategy {
    Skip,
    Rename,      // Add number suffix
    Overwrite,   // If different file
    Compare,     // Check if same via hash
}

pub struct OrganizeResult {
    pub processed: usize,
    pub moved: usize,
    pub skipped: usize,
    pub errors: Vec<OrganizeError>,
    pub space_saved: u64,
}
```

### 2. Implement Date-Based Directory Structure
```rust
fn generate_output_path(
    &self,
    file_info: &FileInfo,
    base_dir: &Path,
) -> PathBuf {
    let date = file_info.creation_date
        .or(file_info.modified_date)
        .unwrap_or_else(|| Utc::now());
    
    // Structure: output/2024/2024-03/2024-03-15/filename.jpg
    base_dir
        .join(date.year().to_string())
        .join(format!("{}-{:02}", date.year(), date.month()))
        .join(format!("{}-{:02}-{:02}", 
            date.year(), date.month(), date.day()))
        .join(&file_info.filename)
}
```

### 3. Add Intelligent Conflict Resolution
```rust
async fn resolve_conflict(
    &self,
    source: &Path,
    destination: &Path,
    file_info: &FileInfo,
) -> Result<PathBuf> {
    match self.conflict_strategy {
        ConflictStrategy::Skip => Ok(destination.to_path_buf()),
        ConflictStrategy::Rename => {
            // Find available name: file.jpg -> file_2.jpg
            find_available_filename(destination).await
        },
        ConflictStrategy::Compare => {
            // Compare hashes, skip if identical
            if are_files_identical(source, destination).await? {
                Ok(destination.to_path_buf())
            } else {
                find_available_filename(destination).await
            }
        },
        ConflictStrategy::Overwrite => Ok(destination.to_path_buf()),
    }
}
```

### 4. Implement Batch Operations
```rust
pub async fn organize_files(
    &self,
    file_stream: mpsc::Receiver<FileInfo>,
    duplicate_decisions: HashMap<String, PathBuf>,
    progress: Arc<ProgressTracker>,
) -> Result<OrganizeResult> {
    let semaphore = Arc::new(Semaphore::new(10)); // Limit concurrent ops
    
    while let Some(file_info) = file_stream.recv().await {
        let permit = semaphore.acquire().await?;
        
        tokio::spawn(async move {
            self.process_single_file(file_info).await;
            drop(permit);
        });
    }
}
```

### 5. Add Metadata Preservation
```rust
#[cfg(target_os = "macos")]
async fn preserve_file_metadata(
    source: &Path,
    destination: &Path,
) -> Result<()> {
    // Preserve timestamps
    let metadata = tokio::fs::metadata(source).await?;
    filetime::set_file_times(
        destination,
        metadata.accessed().ok(),
        metadata.modified().ok(),
    )?;
    
    // Preserve extended attributes (xattr)
    copy_extended_attributes(source, destination)?;
    
    // Preserve permissions
    tokio::fs::set_permissions(
        destination,
        metadata.permissions(),
    ).await?;
    
    Ok(())
}
```

### 6. Implement Transaction-Like Operations
```rust
pub struct FileOperation {
    source: PathBuf,
    destination: PathBuf,
    operation: OperationType,
    status: OperationStatus,
}

impl FileOperation {
    async fn execute(&mut self) -> Result<()> {
        // Create parent directories
        tokio::fs::create_dir_all(
            self.destination.parent().unwrap()
        ).await?;
        
        // Perform operation
        match self.operation {
            OperationType::Move => {
                tokio::fs::rename(&self.source, &self.destination).await?
            },
            OperationType::Copy => {
                tokio::fs::copy(&self.source, &self.destination).await?
            },
            OperationType::Hardlink => {
                std::fs::hard_link(&self.source, &self.destination)?
            },
        }
        
        self.status = OperationStatus::Completed;
        Ok(())
    }
    
    async fn rollback(&mut self) -> Result<()> {
        // Undo operation if needed
    }
}
```

### 7. Add Dry Run Support
```rust
pub async fn dry_run(
    &self,
    files: Vec<FileInfo>,
) -> Result<Vec<PlannedOperation>> {
    // Generate operation plan without executing
    // Show what would happen
    // Calculate space requirements
    // Identify potential conflicts
}
```

## Performance Requirements
- Move/copy at disk speed limits
- Batch operations for efficiency
- Parallel processing with limits
- Minimal memory overhead
- Handle millions of files

## Error Handling
- Atomic operations where possible
- Rollback on critical failures
- Continue on non-critical errors
- Detailed error reporting
- Preserve source files on failure

## Testing Strategy
- Unit tests for path generation
- Test conflict resolution strategies
- Verify metadata preservation
- Test with various file systems
- Benchmark operation speed

## Success Criteria
- [ ] Correctly organizes files by date
- [ ] Handles conflicts intelligently
- [ ] Preserves all metadata
- [ ] Achieves disk-speed operations
- [ ] Dry run accurately predicts operations
- [ ] No data loss under any condition

## Integration Points
- Receives processed files from scanner
- Uses duplicate decisions from detector
- Reports progress continuously
- Generates detailed operation log

## Operation Log Format
```json
{
  "operations": [
    {
      "source": "/input/IMG_1234.jpg",
      "destination": "/output/2024/2024-03/2024-03-15/IMG_1234.jpg",
      "operation": "move",
      "status": "success",
      "size": 2048576,
      "timestamp": "2024-03-15T10:30:00Z"
    }
  ],
  "summary": {
    "total_processed": 10000,
    "successful": 9998,
    "failed": 2,
    "space_saved": 5368709120,
    "duration_seconds": 300
  }
}
```

## Next Task
After completing file organization, proceed to Task 6: Implement Progress Tracking and Reporting