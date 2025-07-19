# Task 2: Implement CLI Argument Parsing Module

## Objective
Implement a robust CLI argument parsing module using the `clap` crate that handles all required and optional arguments for the Media File Organizer, with proper validation and user-friendly error messages.

## Prerequisites
- Task 1 (Basic Rust Project Structure) completed
- `clap` v4 dependency added to Cargo.toml
- Basic project structure in place

## Implementation Steps

### 1. Create CLI Module Structure (`src/cli.rs`)
```rust
use clap::{Command, Arg, ArgMatches};
use std::path::PathBuf;

pub struct CliArgs {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub dry_run: bool,
    pub verbose: bool,
    pub exclude_duplicates: bool,
    pub min_file_size: Option<u64>,
    pub max_file_size: Option<u64>,
    pub file_types: Option<Vec<String>>,
    pub date_range: Option<(String, String)>,
    pub threads: Option<usize>,
}
```

### 2. Implement Argument Parser
- **Required arguments:**
  - `--input` / `-i`: Source directory path
  - `--output` / `-o`: Destination directory path

- **Optional arguments:**
  - `--dry-run`: Preview operations without moving files
  - `--verbose` / `-v`: Enable detailed logging
  - `--exclude-duplicates`: Skip duplicate files
  - `--min-size`: Minimum file size filter (e.g., "1MB")
  - `--max-size`: Maximum file size filter (e.g., "100MB")
  - `--file-types`: Comma-separated list of extensions (e.g., "jpg,mp4,heic")
  - `--date-range`: Date range filter (e.g., "2023-01-01:2023-12-31")
  - `--threads`: Number of parallel threads (default: CPU count)

### 3. Add Validation Logic
- Verify input directory exists
- Ensure output directory can be created
- Validate file size format (KB, MB, GB)
- Parse and validate date ranges
- Validate thread count (1 to max CPU cores)

### 4. Implement Help and Version Information
```rust
fn build_cli() -> Command {
    Command::new("ImagesSorter")
        .version("0.1.0")
        .author("Your Name")
        .about("High-performance media file organizer for macOS")
        .long_about("Organize millions of photos and videos by date...")
        // Add all arguments with descriptions
}
```

### 5. Create Unit Tests
- Test argument parsing with valid inputs
- Test validation error handling
- Test default values
- Test help and version display

## Success Criteria
- [x] CLI module compiles without errors
- [x] `cargo run -- --help` displays comprehensive help
- [x] All arguments are parsed correctly
- [x] Validation catches invalid inputs with clear error messages
- [ ] Unit tests pass with >90% coverage
- [ ] Performance: Argument parsing completes in <10ms

## Code Example Structure
```rust
// src/cli.rs
pub fn parse_args() -> Result<CliArgs, Box<dyn std::error::Error>> {
    let matches = build_cli().get_matches();
    
    // Parse and validate arguments
    let input_dir = parse_path(matches.value_of("input"))?;
    let output_dir = parse_path(matches.value_of("output"))?;
    
    // ... additional parsing ...
    
    Ok(CliArgs {
        input_dir,
        output_dir,
        // ... other fields ...
    })
}
```

## Integration Points
- Called from `main.rs` at startup
- Returns structured `CliArgs` for use by other modules
- Errors propagate to main for user-friendly display

## Next Task
After completing CLI parsing, proceed to Task 3: Implement File Scanning Module