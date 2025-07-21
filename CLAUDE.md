# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MediaOrganizer is a high-performance Rust CLI application for organizing media files on macOS, specifically optimized for Apple Silicon processors. The project aims to process millions of files efficiently while detecting duplicates and organizing them by date.

## Architecture

### Core Modules
- `main.rs` - Async entry point using Tokio runtime, command handlers
- `cli.rs` - Command-line interface with clap (organize, dedup, init-db commands)
- `types.rs` - Core data structures (FileType, FileInfo, etc.)
- `scanner.rs` - File system scanning logic
- `duplicate.rs` - BLAKE3-based duplicate detection
- `organizer.rs` - File organization logic
- `progress.rs` - Progress tracking with indicatif
- `database.rs` - Persistent hash database for incremental processing
- `dedup.rs` - Standalone deduplication functionality

### Key Design Patterns
- **Streaming Processing**: Handle millions of files without loading all into memory
- **Parallel Processing**: Rayon for CPU-bound tasks, Tokio for async I/O
- **Error Handling**: anyhow for application errors, thiserror for custom error types

## Common Development Commands

### Building
```bash
# Debug build
cargo build

# Release build (optimized for Apple Silicon)
cargo build --release

# Check code without building
cargo check
```

### Testing
```bash
# Run all tests
cargo test

# Run integration tests only
cargo test --test integration_tests

# Run with verbose output
cargo test -- --nocapture

# Run a specific test
cargo test test_basic_functionality
```

### Benchmarking
```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench file_scanning
```

### Code Quality
```bash
# Format code (uses rustfmt.toml configuration)
cargo fmt

# Check formatting without changing files
cargo fmt -- --check

# Run clippy for linting
cargo clippy -- -D warnings

# Run clippy with all features
cargo clippy --all-features -- -D warnings
```

### Running the Application
```bash
# Organize files
cargo run -- organize -i /path/to/source -o /path/to/destination

# Remove duplicates
cargo run -- dedup -d /path/to/directory --dry-run

# Initialize/update hash database
cargo run -- init-db -d /path/to/folder

# Run with verbose logging
RUST_LOG=debug cargo run -- organize -i /source -o /dest --verbose

# Dry run to preview operations
cargo run -- organize -i /source -o /dest --dry-run
```

## Performance Targets

The application is designed to meet these requirements:
- Process 1,000+ files per second on Apple Silicon
- Memory usage under 500MB regardless of folder size
- Handle 1 million files in under 30 minutes

## Supported File Types

### Images
JPEG, PNG, HEIC/HEIF, RAW formats (CR2, NEF, ARW, DNG), GIF, BMP, TIFF, WebP

### Videos
MP4/M4V, MOV, AVI, MKV, WebM, FLV, WMV

## Current Implementation Status

The project is fully functional with three main commands:
1. `organize` - Organize media files by date/type with optional duplicate detection
2. `dedup` - Remove duplicate files within a directory (keeps oldest)
3. `init-db` - Pre-compute or update hash database for faster subsequent runs

### Key Features Implemented
- Parallel file scanning with configurable workers
- BLAKE3-based duplicate detection with persistent database
- Multiple organization patterns (year/month, type, custom)
- Real-time progress tracking with detailed statistics
- Dry-run mode for safe operation preview
- Comprehensive error handling and recovery

## Development Guidelines

### Error Handling
- Use `anyhow::Result` for functions that can fail
- Create custom error types with `thiserror` for domain-specific errors
- Always provide context with `.context()` when propagating errors

### Performance Considerations
- Use `rayon` for parallel file processing
- Stream files instead of collecting into memory
- Leverage BLAKE3 for fast hashing
- Use indicatif's `ProgressBar::set_draw_target` to reduce overhead

### Testing Strategy
- Unit tests for individual components (in module files)
- Integration tests in `tests/` directory
- Benchmarks in `benches/` for performance-critical code
- Use `tempfile` crate for test file system operations

### To do every time
- Update the README.md file with the latest information.
- Update the CLAUDE.md file with the latest information.
- Update the Cargo.toml file with the latest information.
- Update the Cargo.lock file with the latest information.
- Build and fix any errors and warnings.
- Don't allow dead code, either remove it or use it.