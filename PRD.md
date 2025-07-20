# Product Requirements Document (PRD)
# Media File Organizer CLI

## 1. Executive Summary

### Product Name
Media File Organizer CLI

### Product Description
A high-performance command-line interface (CLI) application designed to efficiently organize large collections of media files by date, handling millions of files with duplicate detection and smart categorization.

### Target Platform
Apple Silicon (M1/M2/M3) macOS systems

### Key Value Proposition
- Process millions of files efficiently on Apple Silicon
- Automatic date-based organization (YEAR/MONTH structure)
- Intelligent duplicate detection to save storage
- Smart file type categorization
- Minimal memory footprint for massive file operations

## 2. Product Goals & Objectives

### Primary Goals
1. **Performance**: Process millions of files without memory exhaustion
2. **Accuracy**: Correctly organize files by creation/modification date
3. **Efficiency**: Detect and skip duplicates to avoid redundant copying
4. **Simplicity**: Intuitive CLI interface with minimal configuration

### Success Metrics
- Process 1 million files in under 30 minutes on M1 Pro
- Memory usage stays under 500MB regardless of folder size
- 100% accurate duplicate detection
- Zero data loss during organization

## 3. User Stories

### As a photographer/videographer
- I want to organize my massive media library by date
- I want to avoid copying duplicate files
- I want to separate media files from other file types
- I want to clean up duplicate files within my existing library

### As a system administrator
- I want to process large file archives efficiently
- I want progress tracking for long-running operations
- I want detailed logs of what was processed

## 4. Functional Requirements

### 4.1 Core Features

#### Input/Output Management
- Accept source folder path as input parameter
- Accept destination folder path as output parameter
- Validate folder permissions before processing
- Create output directory structure automatically

#### File Scanning
- Recursively scan all files in input folder and subfolders
- Process files in streaming fashion to handle millions of files
- Extract file metadata (creation date, modification date, size, type)
- Handle symbolic links and aliases appropriately

#### Date Detection
- Primary: Use file creation date from metadata
- Fallback: Use modification date if creation date unavailable
- Parse EXIF data for images when filesystem dates are unreliable
- Parse video metadata for accurate creation dates

#### File Organization
- Create folder structure: `output/YYYY/MM/`
- Place images (jpg, jpeg, png, gif, bmp, raw, heic, webp) in month folder
- Place videos (mp4, mov, avi, mkv, webm, m4v) in month folder
- Create `output/YYYY/MM/others/` for non-media files
- Preserve original filenames

#### Duplicate Detection
- Calculate file hash (BLAKE3) for content-based comparison
- Maintain efficient in-memory or disk-based duplicate tracking
- Compare file size + hash for accurate duplicate detection
- Skip copying if identical file already exists in destination
- Log duplicate detections for user awareness
- Support for standalone deduplication within a directory
- **Persistent Hash Database**: Store computed hashes in `db.mediaorg` file at output root
  - Dramatically reduces processing time on subsequent runs
  - Only rehashes new or modified files (checks size and modification time)
  - Compressed binary format for efficient storage
  - Automatic cleanup of obsolete entries

#### Progress & Logging
- Display real-time progress (files processed, estimated time remaining)
- Provide detailed operation logs
- Summary report upon completion
- Error handling with graceful recovery

### 4.2 CLI Interface

#### Organize Command
```bash
# Basic usage
media-organizer organize --input /path/to/source --output /path/to/destination

# With options
media-organizer organize \
  --input /path/to/source \
  --output /path/to/destination \
  --detect-duplicates \
  --dry-run \
  --verbose \
  --workers 8
```

##### Required Arguments
- `--input, -i`: Source folder path
- `--output, -o`: Destination folder path

##### Optional Arguments
- `--detect-duplicates, -d`: Enable duplicate file detection
- `--duplicate-strategy`: How to handle duplicates (skip, rename, replace)
- `--dry-run`: Preview operations without copying files
- `--verbose, -v`: Enable detailed output
- `--quiet, -q`: Suppress non-error output
- `--workers, -j`: Number of parallel workers (default: CPU cores)
- `--types, -t`: Comma-separated list of file types to process
- `--pattern, -p`: Organization pattern (year, year/month, year/month/day)
- `--mode, -m`: Operation mode (copy or move)
- `--help, -h`: Display help information

#### Dedup Command (New Feature)
```bash
# Basic usage
media-organizer dedup --directory /path/to/folder

# With options
media-organizer dedup \
  --directory /path/to/folder \
  --dry-run \
  --verbose \
  --save-list deleted.txt
```

##### Required Arguments
- `--directory, -d`: Directory to scan for duplicates

##### Optional Arguments
- `--dry-run`: Preview which files would be deleted without making changes
- `--force, -f`: Skip confirmation prompt before deletion
- `--save-list`: Save list of deleted files to a file
- `--types, -t`: Comma-separated list of file types to process
- `--verbose, -v`: Enable detailed output
- `--quiet, -q`: Suppress non-error output
- `--workers, -j`: Number of parallel workers
- `--help, -h`: Display help information

##### Deduplication Strategy
- Keeps the oldest file based on creation date (or modification date if creation unavailable)
- Deletes newer duplicates to free up space
- Shows space savings before deletion

## 5. Non-Functional Requirements

### 5.1 Performance Requirements
- Process minimum 1,000 files per second on Apple Silicon
- Memory usage under 500MB for any folder size
- Utilize Apple Silicon's unified memory architecture
- Leverage multiple CPU cores for parallel processing
- Stream processing to avoid loading all files into memory
- **Hash Database Optimization**: Skip re-hashing unchanged files on subsequent runs
  - First run: Full hash computation for all files
  - Subsequent runs: Only hash new/modified files
  - Example: 1M files with 1K changes = 99.9% reduction in hash operations

### 5.2 Technical Requirements
- Native compilation for Apple Silicon (arm64)
- Minimal external dependencies
- Static binary distribution option
- macOS 12.0+ compatibility

### 5.3 Reliability Requirements
- Graceful handling of file system errors
- Resume capability for interrupted operations
- No data loss under any circumstances
- Atomic file operations (complete or rollback)

## 6. Technical Architecture

### 6.1 Recommended Technology Stack

#### Primary Option: Rust
- **Advantages**:
  - Excellent performance on Apple Silicon
  - Memory safety without garbage collection
  - Great concurrency primitives
  - Native arm64 compilation
  - Excellent file system libraries (tokio, walkdir)
  - Fast hashing libraries (blake3, xxhash)

#### Alternative Option: Go
- **Advantages**:
  - Good performance with simple concurrency
  - Easy cross-compilation
  - Built-in testing framework
  - Good standard library for file operations

### 6.2 Core Components

#### File Scanner
- Concurrent directory traversal
- Streaming file processing
- Metadata extraction pipeline

#### Duplicate Detector
- Efficient hash computation
- Bloom filter for initial screening
- Persistent hash cache option

#### File Organizer
- Date parser and normalizer
- Directory structure creator
- Concurrent file copying

#### Progress Tracker
- Real-time statistics collection
- ETA calculation
- Progress bar display

### 6.3 Data Structures

```rust
struct FileInfo {
    path: PathBuf,
    size: u64,
    created: DateTime,
    modified: DateTime,
    hash: Option<Hash>,
    file_type: FileType,
}

enum FileType {
    Image(ImageFormat),
    Video(VideoFormat),
    Other,
}

struct DuplicateTracker {
    seen_hashes: HashMap<Hash, PathBuf>,
    bloom_filter: BloomFilter,
}
```

## 7. User Interface Mockup

```
Media File Organizer v1.0.0

Scanning: /Users/alex/Photos
Output to: /Volumes/Backup/Organized

[████████████████████░░░░░░] 75% | 750,234/1,000,000 files
├─ Processing: IMG_2945.jpg
├─ Speed: 1,234 files/sec
├─ Time remaining: 3m 24s
├─ Duplicates found: 12,456
└─ Memory usage: 234 MB

Recent activity:
✓ Copied: IMG_2944.jpg → 2024/01/IMG_2944.jpg
✓ Skipped duplicate: IMG_2943.jpg (already exists)
✓ Copied: video_001.mp4 → 2024/01/video_001.mp4
```

## 8. Error Handling

### File System Errors
- Permission denied: Log and continue
- File not found: Log and continue
- Disk full: Pause and prompt user
- I/O errors: Retry with exponential backoff

### Data Integrity
- Verify file copy with size comparison
- Optional hash verification after copy
- Maintain operation log for recovery

## 9. Testing Strategy

### Unit Tests
- Hash calculation accuracy
- Date parsing edge cases
- Duplicate detection logic
- File type classification

### Integration Tests
- Large folder processing (1M+ files)
- Various file system types
- Unicode filename handling
- Symbolic link handling

### Performance Tests
- Benchmark with different file sizes
- Memory usage under load
- CPU utilization efficiency
- Disk I/O optimization

## 10. Future Enhancements

### Phase 2 Features
- Cloud storage support (iCloud, Google Drive)
- Smart album creation based on events
- Face/object recognition grouping
- Batch rename capabilities
- Undo/redo functionality

### Phase 3 Features
- GUI version for non-technical users
- Network folder support
- Real-time folder monitoring
- Integration with photo management tools
- Compression options for older files

## 11. Delivery & Deployment

### Distribution
- Homebrew formula for easy installation
- Direct binary download from GitHub releases
- Optional installer package (.pkg)

### Documentation
- Comprehensive README with examples
- Man page for Unix users
- Video tutorial for common use cases

### Support
- GitHub issues for bug reports
- Discord community for user support
- Regular update schedule (monthly)

## 12. Success Criteria

### Launch Metrics
- Process 10 million files without crashes
- 95% user satisfaction rating
- <0.01% data loss reports
- Active usage by 1,000+ users in first month

### Long-term Goals
- Become the standard tool for media organization on macOS
- Expand to other platforms while maintaining performance
- Build ecosystem of plugins and extensions