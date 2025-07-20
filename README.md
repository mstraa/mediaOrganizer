# ImagesSorter

A high-performance Rust CLI application for organizing and managing media files on macOS, optimized for Apple Silicon processors. Designed to process millions of files efficiently while detecting duplicates and organizing them by date.

## Features

- **High Performance**: Process 1,000+ files per second on Apple Silicon
- **Memory Efficient**: Uses under 500MB RAM regardless of folder size
- **Duplicate Detection**: BLAKE3-based fast hashing for finding duplicate files
- **Smart Organization**: Organize files by date into a structured directory hierarchy (YYYY/MM/DD)
- **Deduplication**: Remove duplicate files with safety checks and detailed reporting
- **Enhanced Progress Tracking**: Real-time progress bars with detailed statistics
- **Supported Formats**:
  - **Images**: JPEG, PNG, HEIC/HEIF, RAW (CR2, NEF, ARW, DNG), GIF, BMP, TIFF, WebP
  - **Videos**: MP4, MOV, AVI, MKV, WebM, FLV, WMV
- **Safety Features**: Dry-run mode, duplicate backup verification, detailed reports

## Prerequisites

- Rust 1.70 or later
- macOS (optimized for Apple Silicon)
- Cargo (comes with Rust)

## Installation

### Install Rust

If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Build from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/ImagesSorter.git
cd ImagesSorter
```

2. Build the project:
```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized for performance)
cargo build --release
```

The binary will be located at:
- Debug: `target/debug/media-organizer`
- Release: `target/release/media-organizer`

3. (Optional) Install globally:
```bash
cargo install --path .
```

## Usage

### Organize Command

Organize media files from a source directory into a date-based structure:

```bash
# Basic usage
media-organizer organize --input /path/to/source --output /path/to/destination

# Short form
media-organizer organize -i /source -o /dest

# Dry run (preview without making changes)
media-organizer organize -i /source -o /dest --dry-run

# Verbose output
media-organizer organize -i /source -o /dest --verbose

# Specify number of threads
media-organizer organize -i /source -o /dest --threads 8
```

### Deduplicate Command

Find and remove duplicate files:

```bash
# Basic deduplication
media-organizer deduplicate --directory /path/to/media

# Short form
media-organizer deduplicate -d /media

# Dry run (preview what would be deleted)
media-organizer deduplicate -d /media --dry-run

# Generate a report of duplicates
media-organizer deduplicate -d /media --report /path/to/report.txt

# With verbose output
media-organizer deduplicate -d /media --verbose
```

### Command Line Options

#### Organize Command
- `-i, --input <PATH>` - Source directory containing media files
- `-o, --output <PATH>` - Destination directory for organized files
- `-d, --dry-run` - Preview operations without making changes
- `-v, --verbose` - Enable verbose logging
- `-t, --threads <NUM>` - Number of threads for parallel processing (default: CPU count)

#### Deduplicate Command
- `-d, --directory <PATH>` - Directory to scan for duplicates
- `--dry-run` - Preview what would be deleted without making changes
- `-r, --report <PATH>` - Generate a detailed report of duplicates
- `-v, --verbose` - Enable verbose logging
- `-t, --threads <NUM>` - Number of threads for parallel processing (default: CPU count)

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_basic_functionality
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings
```

### Benchmarks

```bash
# Run performance benchmarks
cargo bench
```

## Output Structure

When organizing files, ImagesSorter creates a date-based directory structure:

```
output/
├── 2024/
│   ├── 01/
│   │   ├── 15/
│   │   │   ├── IMG_1234.jpg
│   │   │   └── VID_5678.mp4
│   │   └── 16/
│   │       └── DSC_9012.raw
│   └── 02/
│       └── ...
└── Unknown/
    └── files_without_dates.jpg
```

## Architecture

The application uses:
- **Tokio** for async runtime
- **Rayon** for parallel processing
- **BLAKE3** for fast, cryptographic hashing
- **Indicatif** for progress bars with real-time statistics
- **Clap** for CLI parsing
- **Chrono** for date/time handling

## Performance

Optimized for Apple Silicon with:
- Streaming file processing (no full directory loading)
- Parallel scanning and hashing
- Efficient memory usage through iterators
- Native Apple Silicon optimizations
- Progress tracking with minimal overhead

## License

[Your license here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.