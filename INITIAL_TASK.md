# Set Up Basic Rust Project Structure

This command initializes the Media File Organizer CLI project with Rust (as recommended in the PRD).

## Task: Initialize Rust Project

### Objective
Set up the basic Rust project structure for the Media File Organizer CLI application targeting Apple Silicon (M1/M2/M3) macOS systems.

### Steps to execute:

1. **Create Rust project**
   ```bash
   cargo new media-organizer --bin
   cd media-organizer
   ```

2. **Configure Cargo.toml for Apple Silicon optimization**
   - Set package metadata (name, version, authors, description)
   - Add required dependencies:
     - `clap` (v4) - for CLI argument parsing
     - `tokio` - for async runtime and file operations
     - `walkdir` - for recursive directory traversal
     - `chrono` - for date/time handling
     - `blake3` - for fast hashing (duplicate detection)
     - `indicatif` - for progress bars
     - `tracing` - for logging
     - `anyhow` - for error handling

3. **Set up project structure**
   ```
   media-organizer/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs
   │   ├── cli.rs          # CLI argument parsing
   │   ├── scanner.rs      # File scanning logic
   │   ├── organizer.rs    # File organization logic
   │   ├── duplicate.rs    # Duplicate detection
   │   ├── progress.rs     # Progress tracking
   │   └── types.rs        # Data structures (FileInfo, FileType, etc.)
   ├── tests/
   │   └── integration_tests.rs
   └── README.md
   ```

4. **Configure for Apple Silicon**
   - Add `.cargo/config.toml` with arm64 target settings
   - Enable optimizations for M1/M2/M3 processors

5. **Implement basic CLI structure in main.rs**
   - Set up clap for parsing --input and --output arguments
   - Add help text and version information
   - Create placeholder functions for each major component

6. **Set up development environment**
   - Add `.gitignore` for Rust projects
   - Configure rustfmt.toml for consistent code formatting
   - Set up basic GitHub Actions workflow for CI

### Success Criteria
- Project compiles with `cargo build`
- CLI shows help with `cargo run -- --help`
- All dependencies resolve correctly for Apple Silicon
- Basic project structure is in place for further development

### Next Steps
After this task is complete, the next task will be to implement the CLI argument parsing module (cli.rs) with all required and optional arguments as specified in the PRD.