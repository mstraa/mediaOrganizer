# Media Organizer

A high-performance CLI tool for organizing media files on macOS, optimized for Apple Silicon (M1/M2/M3) processors.

## Features

- 🚀 **High Performance**: Optimized for Apple Silicon with parallel processing
- 📁 **Smart Organization**: Multiple organization patterns (by date, type, or custom)
- 🔍 **Duplicate Detection**: Fast BLAKE3-based duplicate file detection
- 📊 **Progress Tracking**: Real-time progress bars and statistics
- 🎯 **Flexible Options**: Copy or move operations with extensive configuration
- 🏃 **Dry Run Mode**: Preview changes before execution
- 📝 **Detailed Reporting**: Comprehensive operation summaries

## Supported File Types

### Images
- JPEG/JPG
- PNG
- HEIC/HEIF
- RAW formats (CR2, NEF, ARW, DNG)
- GIF, BMP, TIFF, WebP

### Videos
- MP4/M4V
- MOV
- AVI, MKV, WebM
- FLV, WMV

## Installation

### Prerequisites
- macOS 13.0 or later
- Rust 1.70 or later

### Building from Source
```bash
git clone https://github.com/yourusername/media-organizer.git
cd media-organizer
cargo build --release
```

The optimized binary will be available at `target/release/media-organizer`.

## Usage

### Basic Usage
```bash
media-organizer -i /path/to/input -o /path/to/output
```

### Organization Patterns
```bash
# Organize by year/month (default)
media-organizer -i ~/Pictures -o ~/Organized -p year/month

# Organize by file type
media-organizer -i ~/Downloads -o ~/Media -p type

# Custom pattern
media-organizer -i ~/Camera -o ~/Photos -p "{type}/{year}/{month}/{day}"
```

### Advanced Options
```bash
# Move files instead of copying
media-organizer -i ~/Unsorted -o ~/Sorted -m move

# Enable duplicate detection with rename strategy
media-organizer -i ~/Photos -o ~/Clean -d --duplicate-strategy rename

# Dry run to preview changes
media-organizer -i ~/Media -o ~/Organized --dry-run

# Process only specific file types
media-organizer -i ~/Mixed -o ~/Images -t jpg,png,heic

# Parallel processing with 8 workers
media-organizer -i ~/Large -o ~/Processed -j 8
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input <DIR>` | Input directory | Required |
| `-o, --output <DIR>` | Output directory | Required |
| `-p, --pattern <PATTERN>` | Organization pattern | year/month |
| `-m, --mode <MODE>` | Operation mode (copy/move) | copy |
| `-d, --detect-duplicates` | Enable duplicate detection | false |
| `--duplicate-strategy <STRATEGY>` | Duplicate handling (skip/rename/replace) | skip |
| `-t, --types <TYPES>` | File types to process | all |
| `--dry-run` | Preview without changes | false |
| `-j, --workers <NUM>` | Parallel workers (0=auto) | 0 |
| `-v, --verbose` | Verbose output | false |
| `-q, --quiet` | Suppress output | false |

Run `media-organizer --help` for a complete list of options.

## Configuration File

You can use a configuration file to set default options:

```toml
# ~/.config/media-organizer/config.toml
pattern = "type/year/month"
mode = "copy"
detect_duplicates = true
duplicate_strategy = "rename"
workers = 4
preserve_timestamps = true
```

## Performance

The Media Organizer is optimized for Apple Silicon and uses:
- BLAKE3 for fast cryptographic hashing
- Parallel processing with Rayon
- Async I/O with Tokio
- Zero-copy operations where possible

## Development

### Running Tests
```bash
cargo test
```

### Running Benchmarks
```bash
cargo bench
```

### Code Formatting
```bash
cargo fmt
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.