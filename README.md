# ImagesSorter

A high-performance Rust CLI application for organizing media files on macOS, optimized for Apple Silicon processors. Designed to process millions of files efficiently while detecting duplicates and organizing them by date.

## Features

- **High Performance**: Process 1,000+ files per second on Apple Silicon
- **Memory Efficient**: Uses under 500MB RAM regardless of folder size
- **Duplicate Detection**: BLAKE3-based fast hashing for finding duplicate files
- **Smart Organization**: Organize files by date into a structured directory hierarchy
- **Supported Formats**:
  - **Images**: JPEG, PNG, HEIC/HEIF, RAW (CR2, NEF, ARW, DNG), GIF, BMP, TIFF, WebP
  - **Videos**: MP4, MOV, AVI, MKV, WebM, FLV, WMV

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
- Debug: `target/debug/images-sorter`
- Release: `target/release/images-sorter`

## Usage

```bash
# Basic usage
images-sorter --input /path/to/source --output /path/to/destination

# Short form
images-sorter -i /source -o /dest

# Dry run (preview without making changes)
images-sorter -i /source -o /dest --dry-run

# Verbose output
images-sorter -i /source -o /dest --verbose

# Specify number of threads
images-sorter -i /source -o /dest --threads 8
```

### Command Line Options

- `-i, --input <PATH>` - Source directory containing media files
- `-o, --output <PATH>` - Destination directory for organized files
- `-d, --dry-run` - Preview operations without making changes
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

## Architecture

The application uses:
- **Tokio** for async runtime
- **Rayon** for parallel processing
- **BLAKE3** for fast, cryptographic hashing
- **Indicatif** for progress bars
- **Clap** for CLI parsing

## Performance

Optimized for Apple Silicon with:
- Streaming file processing (no full directory loading)
- Parallel scanning and hashing
- Efficient memory usage through iterators
- Native Apple Silicon optimizations

## License

[Your license here]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.