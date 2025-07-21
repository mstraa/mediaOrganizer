# Media Organizer

A high-performance CLI tool for organizing media files on macOS, optimized for Apple Silicon (M1/M2/M3) processors.

## Features

- 🚀 **High Performance**: Optimized for Apple Silicon with parallel processing
- 📁 **Smart Organization**: Multiple organization patterns (by date, type, or custom)
- 🔍 **Duplicate Detection**: Fast BLAKE3-based duplicate file detection
- 🧹 **Deduplication**: Remove duplicate files within directories (keeps oldest)
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

The media-organizer has three main commands: `organize`, `dedup`, and `init-db`.

### Organize Command

#### Basic Usage
```bash
media-organizer organize -i /path/to/input -o /path/to/output
```

#### Organization Patterns
```bash
# Organize by year/month (default)
media-organizer organize -i ~/Pictures -o ~/Organized -p year/month

# Organize by file type
media-organizer organize -i ~/Downloads -o ~/Media -p type

# Organize by year/month/day
media-organizer organize -i ~/Camera -o ~/Photos -p year/month/day
```

#### Advanced Options
```bash
# Move files instead of copying
media-organizer organize -i ~/Unsorted -o ~/Sorted -m move

# Enable duplicate detection with rename strategy
media-organizer organize -i ~/Photos -o ~/Clean -d --duplicate-strategy rename

# Dry run to preview changes
media-organizer organize -i ~/Media -o ~/Organized --dry-run

# Process only specific file types
media-organizer organize -i ~/Mixed -o ~/Images -t jpg,png,heic

# Parallel processing with 8 workers
media-organizer organize -i ~/Large -o ~/Processed -j 8
```

### Dedup Command (New!)

Remove duplicate files within a directory, keeping the oldest version of each file.

#### Basic Usage
```bash
# Preview duplicates (dry run)
media-organizer dedup -d /path/to/folder --dry-run

# Delete duplicates with confirmation
media-organizer dedup -d /path/to/folder

# Delete duplicates without confirmation (use with caution!)
media-organizer dedup -d /path/to/folder --force
```

#### Advanced Options
```bash
# Process only specific file types
media-organizer dedup -d ~/Photos -t jpg,png,heic

# Save list of deleted files
media-organizer dedup -d ~/Media --save-list deleted_files.txt

# Verbose output to see all duplicate groups
media-organizer dedup -d ~/Pictures -v

# Exclude certain patterns
media-organizer dedup -d ~/Documents -e "*.tmp" -e "backup/*"
```

### Init-DB Command (New!)

Pre-compute or update the hash database without organizing files. This significantly speeds up subsequent organization runs by caching file hashes.

#### Basic Usage
```bash
# Initialize database for a directory
media-organizer init-db -d /path/to/media -o /path/to/output

# Update existing database (only hashes new/modified files)
media-organizer init-db -d /path/to/media -o /path/to/output

# Clean up obsolete entries and update database
media-organizer init-db -d /path/to/media -o /path/to/output --cleanup
```

#### Advanced Options
```bash
# Output statistics in JSON format
media-organizer init-db -d ~/Photos -o ~/Organized --json

# Process only specific file types
media-organizer init-db -d ~/Media -o ~/Output -t jpg,png,mp4

# Verbose output to see progress
media-organizer init-db -d ~/Pictures -o ~/Backup -v

# Set custom number of workers
media-organizer init-db -d ~/Large -o ~/Storage -j 16 --hash-workers 8
```

## Command Line Options

### Organize Command Options

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

### Dedup Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --directory <DIR>` | Directory to scan | Required |
| `-t, --types <TYPES>` | File types to process | all |
| `--dry-run` | Preview without deleting | false |
| `-f, --force` | Skip confirmation prompt | false |
| `--save-list <FILE>` | Save deleted files list | none |
| `-j, --workers <NUM>` | Parallel workers (0=auto) | 0 |
| `-v, --verbose` | Verbose output | false |
| `-q, --quiet` | Suppress output | false |

### Init-DB Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --directory <DIR>` | Directory to scan | Required |
| `-o, --output <DIR>` | Output directory for database | Required |
| `-t, --types <TYPES>` | File types to process | all |
| `-j, --workers <NUM>` | Parallel workers (0=auto) | 0 |
| `--hash-workers <NUM>` | Hash computation workers | same as -j |
| `--cleanup` | Remove obsolete entries | false |
| `--json` | Output stats in JSON | false |
| `-v, --verbose` | Verbose output | false |
| `-e, --exclude <PATTERN>` | Exclude patterns | none |
| `--min-size <SIZE>` | Minimum file size | 0 |
| `--max-size <SIZE>` | Maximum file size | unlimited |
| `--follow-links` | Follow symbolic links | false |

Run `media-organizer <command> --help` for complete options for each command.

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