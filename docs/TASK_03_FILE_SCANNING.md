# Task 3: Implement File Scanning Module

## Objective
Create a high-performance file scanning module that can traverse large directory structures efficiently, identify media files, and extract metadata while maintaining low memory usage.

## Prerequisites
- Task 2 (CLI Parsing) completed
- `walkdir` and `tokio` dependencies available
- Basic type definitions in `types.rs`

## Implementation Steps

### 1. Define File Types and Structures (`src/types.rs`)
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Image(ImageFormat),
    Video(VideoFormat),
    Unknown,
}

#[derive(Debug, Clone)]
pub enum ImageFormat {
    Jpeg, Png, Heic, Heif, Raw(String), Gif, Bmp, Tiff, WebP,
}

#[derive(Debug, Clone)]
pub enum VideoFormat {
    Mp4, Mov, Avi, Mkv, WebM, Flv, Wmv,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub modified_date: DateTime<Utc>,
    pub creation_date: Option<DateTime<Utc>>,
    pub hash: Option<String>,
}
```

### 2. Implement Scanner Module (`src/scanner.rs`)
```rust
use walkdir::WalkDir;
use tokio::sync::mpsc;
use rayon::prelude::*;

pub struct Scanner {
    input_dir: PathBuf,
    file_type_filter: Option<Vec<FileType>>,
    size_filter: Option<(u64, u64)>,
    date_filter: Option<(DateTime<Utc>, DateTime<Utc>)>,
}
```

### 3. Add Parallel File Discovery
- Use `walkdir` for directory traversal
- Implement streaming with `mpsc` channels
- Process files in batches for memory efficiency
- Target: 1,000+ files/second scanning speed

### 4. Implement File Type Detection
```rust
fn detect_file_type(path: &Path) -> FileType {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("jpg") | Some("jpeg") => FileType::Image(ImageFormat::Jpeg),
        Some("png") => FileType::Image(ImageFormat::Png),
        Some("heic") | Some("heif") => FileType::Image(ImageFormat::Heic),
        Some("mp4") | Some("m4v") => FileType::Video(VideoFormat::Mp4),
        // ... other formats ...
        _ => FileType::Unknown,
    }
}
```

### 5. Extract File Metadata
- File size from filesystem
- Modified/creation dates
- EXIF data for images (using `kamadak-exif`)
- Video metadata (duration, resolution)

### 6. Implement Filtering Logic
- Apply file type filters
- Check size constraints
- Validate date ranges
- Skip hidden files and system directories

### 7. Memory-Efficient Streaming
```rust
pub async fn scan_files(
    &self,
    progress: Arc<ProgressTracker>,
) -> Result<mpsc::Receiver<FileInfo>> {
    let (tx, rx) = mpsc::channel(1000); // Buffer size
    
    tokio::spawn(async move {
        // Stream files without loading all into memory
    });
    
    Ok(rx)
}
```

## Performance Requirements
- Process 1,000+ files per second on Apple Silicon
- Memory usage under 100MB for scanning
- Support millions of files without OOM
- Utilize all CPU cores efficiently

## Error Handling
- Handle permission errors gracefully
- Skip corrupted files with logging
- Report inaccessible directories
- Continue scanning on errors

## Testing Strategy
- Unit tests for file type detection
- Integration test with temporary file system
- Benchmark scanning performance
- Test memory usage with large datasets

## Success Criteria
- [ ] Scanner compiles and passes all tests
- [ ] Achieves 1,000+ files/second scanning rate
- [ ] Memory usage stays under 100MB
- [ ] Correctly identifies all supported file types
- [ ] Filters work as expected
- [ ] Progress reporting is accurate

## Integration Points
- Receives configuration from CLI module
- Streams results to duplicate detector
- Reports progress to progress tracker
- Passes file info to organizer

## Next Task
After completing file scanning, proceed to Task 4: Implement Duplicate Detection Module