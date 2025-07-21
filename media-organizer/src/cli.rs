use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A high-performance CLI tool for organizing media files on macOS
#[derive(Parser, Debug)]
#[command(
    name = "media-organizer",
    version,
    author,
    about,
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Organize media files from input to output directory
    Organize(OrganizeArgs),
    
    /// Remove duplicate files within a directory
    Dedup(DedupArgs),
    
    /// Initialize or update the hash database only
    InitDb(InitDbArgs),
}

/// Arguments for the organize command
#[derive(Parser, Debug)]
pub struct OrganizeArgs {
    /// Input directory containing media files to organize
    #[arg(short, long, value_name = "DIR")]
    pub input: PathBuf,

    /// Output directory where organized files will be placed
    #[arg(short, long, value_name = "DIR")]
    pub output: PathBuf,

    /// Organization pattern (default: year/month)
    #[arg(
        short = 'p',
        long,
        value_name = "PATTERN",
        default_value = "year/month",
        help = "Organization pattern: year, year/month, year/month/day, type, type/year/month"
    )]
    pub pattern: String,

    /// File operation mode
    #[arg(
        short = 'm',
        long,
        value_name = "MODE",
        default_value = "copy",
        help = "Operation mode: copy or move"
    )]
    pub mode: OperationMode,

    /// Enable duplicate detection
    #[arg(
        short = 'd',
        long,
        help = "Enable duplicate file detection and handling"
    )]
    pub detect_duplicates: bool,

    /// Duplicate handling strategy
    #[arg(
        long,
        value_name = "STRATEGY",
        default_value = "skip",
        help = "Duplicate handling: skip, rename, or replace",
        requires = "detect_duplicates"
    )]
    pub duplicate_strategy: DuplicateStrategy,

    /// File types to process (default: all supported types)
    #[arg(
        short = 't',
        long,
        value_name = "TYPES",
        value_delimiter = ',',
        help = "Comma-separated list of file types: jpg,png,mp4,mov,heic,raw"
    )]
    pub types: Option<Vec<String>>,

    /// Enable dry run mode (preview without making changes)
    #[arg(long, help = "Preview operations without making any changes")]
    pub dry_run: bool,

    /// Number of parallel workers
    #[arg(
        short = 'j',
        long,
        value_name = "NUM",
        default_value = "0",
        help = "Number of parallel workers (0 = auto-detect)"
    )]
    pub workers: usize,

    /// Number of parallel workers for hash computation
    #[arg(
        long,
        value_name = "NUM",
        help = "Number of parallel workers for hash computation (defaults to the value of --workers if not specified)"
    )]
    pub hash_workers: Option<usize>,

    /// Verbose output
    #[arg(short, long, help = "Enable verbose output")]
    pub verbose: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, help = "Suppress non-error output")]
    pub quiet: bool,

    /// Configuration file path
    #[arg(
        short = 'c',
        long,
        value_name = "FILE",
        help = "Path to configuration file"
    )]
    pub config: Option<PathBuf>,

    /// Exclude patterns
    #[arg(
        short = 'e',
        long,
        value_name = "PATTERN",
        help = "Patterns to exclude (can be specified multiple times)"
    )]
    pub exclude: Vec<String>,

    /// Minimum file size to process (in bytes)
    #[arg(
        long,
        value_name = "SIZE",
        default_value = "0",
        help = "Minimum file size to process"
    )]
    pub min_size: u64,

    /// Maximum file size to process (in bytes)
    #[arg(long, value_name = "SIZE", help = "Maximum file size to process")]
    pub max_size: Option<u64>,

    /// Preserve original file timestamps
    #[arg(long, help = "Preserve original file timestamps")]
    pub preserve_timestamps: bool,

    /// Follow symbolic links
    #[arg(long, help = "Follow symbolic links")]
    pub follow_links: bool,

    /// Log file path
    #[arg(long, value_name = "FILE", help = "Path to log file")]
    pub log_file: Option<PathBuf>,

    /// Generate summary report
    #[arg(long, help = "Generate a summary report after completion")]
    pub report: bool,

    /// Report output path
    #[arg(
        long,
        value_name = "FILE",
        help = "Path for the summary report",
        requires = "report"
    )]
    pub report_path: Option<PathBuf>,

    /// Output report in JSON format
    #[arg(long, help = "Output summary report in JSON format")]
    pub json: bool,
}

/// Arguments for the dedup command
#[derive(Parser, Debug)]
pub struct DedupArgs {
    /// Directory to scan for duplicates
    #[arg(short, long, value_name = "DIR")]
    pub directory: PathBuf,

    /// File types to process (default: all supported types)
    #[arg(
        short = 't',
        long,
        value_name = "TYPES",
        value_delimiter = ',',
        help = "Comma-separated list of file types: jpg,png,mp4,mov,heic,raw"
    )]
    pub types: Option<Vec<String>>,

    /// Enable dry run mode (preview without deleting files)
    #[arg(long, help = "Preview which files would be deleted without making changes")]
    pub dry_run: bool,

    /// Number of parallel workers
    #[arg(
        short = 'j',
        long,
        value_name = "NUM",
        default_value = "0",
        help = "Number of parallel workers (0 = auto-detect)"
    )]
    pub workers: usize,

    /// Number of parallel workers for hash computation
    #[arg(
        long,
        value_name = "NUM",
        help = "Number of parallel workers for hash computation (defaults to the value of --workers if not specified)"
    )]
    pub hash_workers: Option<usize>,

    /// Verbose output
    #[arg(short, long, help = "Enable verbose output")]
    pub verbose: bool,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, help = "Suppress non-error output")]
    pub quiet: bool,

    /// Exclude patterns
    #[arg(
        short = 'e',
        long,
        value_name = "PATTERN",
        help = "Patterns to exclude (can be specified multiple times)"
    )]
    pub exclude: Vec<String>,

    /// Minimum file size to process (in bytes)
    #[arg(
        long,
        value_name = "SIZE",
        default_value = "0",
        help = "Minimum file size to process"
    )]
    pub min_size: u64,

    /// Maximum file size to process (in bytes)
    #[arg(long, value_name = "SIZE", help = "Maximum file size to process")]
    pub max_size: Option<u64>,

    /// Follow symbolic links
    #[arg(long, help = "Follow symbolic links")]
    pub follow_links: bool,

    /// Force deletion without confirmation
    #[arg(short, long, help = "Skip confirmation prompt (use with caution!)")]
    pub force: bool,

    /// Output report in JSON format
    #[arg(long, help = "Output summary report in JSON format")]
    pub json: bool,

    /// Save list of deleted files to a file
    #[arg(long, value_name = "FILE", help = "Save list of deleted files to a file")]
    pub save_list: Option<PathBuf>,
}

/// Arguments for the init-db command
#[derive(Parser, Debug)]
pub struct InitDbArgs {
    /// Directory to scan for creating the hash database
    #[arg(short, long, value_name = "DIR")]
    pub directory: PathBuf,
    
    /// Output directory where the database will be saved
    #[arg(short, long, value_name = "DIR")]
    pub output: PathBuf,

    /// File types to process (default: all supported types)
    #[arg(
        short = 't',
        long,
        value_name = "TYPES",
        value_delimiter = ',',
        help = "Comma-separated list of file types: jpg,png,mp4,mov,heic,raw"
    )]
    pub types: Option<Vec<String>>,
    
    /// Number of parallel workers
    #[arg(
        short = 'j',
        long,
        value_name = "NUM",
        default_value = "0",
        help = "Number of parallel workers (0 = auto-detect)"
    )]
    pub workers: usize,
    
    /// Number of parallel workers for hash computation
    #[arg(
        long,
        value_name = "NUM",
        help = "Number of parallel workers for hash computation (defaults to the value of --workers if not specified)"
    )]
    pub hash_workers: Option<usize>,

    /// Verbose output
    #[arg(short, long, help = "Enable verbose output")]
    pub verbose: bool,

    /// Exclude patterns
    #[arg(
        short = 'e',
        long,
        value_name = "PATTERN",
        help = "Patterns to exclude (can be specified multiple times)"
    )]
    pub exclude: Vec<String>,

    /// Minimum file size to process (in bytes)
    #[arg(
        long,
        value_name = "SIZE",
        default_value = "0",
        help = "Minimum file size to process"
    )]
    pub min_size: u64,

    /// Maximum file size to process (in bytes)
    #[arg(long, value_name = "SIZE", help = "Maximum file size to process")]
    pub max_size: Option<u64>,

    /// Follow symbolic links
    #[arg(long, help = "Follow symbolic links")]
    pub follow_links: bool,
    
    /// Clean up obsolete entries from existing database
    #[arg(long, help = "Clean up entries for files that no longer exist")]
    pub cleanup: bool,

    /// Output database statistics in JSON format
    #[arg(long, help = "Output database statistics in JSON format")]
    pub json: bool,
}

// Keep the existing enums
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OperationMode {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq)]
pub enum DuplicateStrategy {
    Skip,
    Rename,
    Replace,
}

impl std::fmt::Display for OperationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationMode::Copy => write!(f, "copy"),
            OperationMode::Move => write!(f, "move"),
        }
    }
}

impl std::fmt::Display for DuplicateStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuplicateStrategy::Skip => write!(f, "skip"),
            DuplicateStrategy::Rename => write!(f, "rename"),
            DuplicateStrategy::Replace => write!(f, "replace"),
        }
    }
}

// Keep backward compatibility by providing an alias
#[allow(dead_code)]
pub type Args = OrganizeArgs;

impl OrganizeArgs {
    /// Validate command line arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check if input directory exists
        if !self.input.exists() {
            anyhow::bail!("Input directory does not exist: {}", self.input.display());
        }

        if !self.input.is_dir() {
            anyhow::bail!("Input path is not a directory: {}", self.input.display());
        }

        // Check if output directory parent exists (create output dir later)
        if let Some(parent) = self.output.parent() {
            if !parent.exists() {
                anyhow::bail!(
                    "Output directory parent does not exist: {}",
                    parent.display()
                );
            }
        }

        // Check for conflicting flags
        if self.verbose && self.quiet {
            anyhow::bail!("Cannot use both --verbose and --quiet flags");
        }

        // Validate file size constraints
        if let Some(max_size) = self.max_size {
            if max_size <= self.min_size {
                anyhow::bail!("Maximum file size must be greater than minimum file size");
            }
        }

        // Validate pattern
        match self.pattern.as_str() {
            "year" | "year/month" | "year/month/day" | "type" | "type/year/month" => {},
            _ => anyhow::bail!("Invalid organization pattern: {}", self.pattern),
        }

        // Validate worker count
        if self.workers > 1000 {
            anyhow::bail!("Worker count too high: {}", self.workers);
        }

        // Check config file if specified
        if let Some(config_path) = &self.config {
            if !config_path.exists() {
                anyhow::bail!(
                    "Configuration file does not exist: {}",
                    config_path.display()
                );
            }
        }

        Ok(())
    }

    /// Get the effective number of workers (0 means auto-detect)
    pub fn get_worker_count(&self) -> usize {
        if self.workers == 0 {
            // Auto-detect based on CPU cores
            num_cpus::get()
        } else {
            self.workers
        }
    }

    /// Get the organization pattern as the proper enum type
    pub fn get_organization_pattern(&self) -> crate::types::OrganizationPattern {
        use crate::types::OrganizationPattern;

        match self.pattern.as_str() {
            "year" => OrganizationPattern::Year,
            "year/month" => OrganizationPattern::YearMonth,
            "year/month/day" => OrganizationPattern::YearMonthDay,
            "type" => OrganizationPattern::Type,
            "type/year/month" => OrganizationPattern::TypeYearMonth,
            pattern => OrganizationPattern::Custom(pattern.to_string()),
        }
    }


    /// Get file types filter if specified
    pub fn get_file_types(&self) -> Option<Vec<crate::types::FileType>> {
        use crate::types::FileType;

        self.types.as_ref().map(|types| {
            types
                .iter()
                .filter_map(|t| match t.to_lowercase().as_str() {
                    "jpg" | "jpeg" => Some(FileType::Jpeg),
                    "png" => Some(FileType::Png),
                    "heic" | "heif" => Some(FileType::Heic),
                    "raw" | "cr2" | "nef" | "arw" | "dng" => Some(FileType::Raw),
                    "gif" => Some(FileType::Gif),
                    "bmp" => Some(FileType::Bmp),
                    "tiff" | "tif" => Some(FileType::Tiff),
                    "webp" => Some(FileType::Webp),
                    "mp4" | "m4v" => Some(FileType::Mp4),
                    "mov" => Some(FileType::Mov),
                    "avi" => Some(FileType::Avi),
                    "mkv" => Some(FileType::Mkv),
                    "webm" => Some(FileType::Webm),
                    "flv" => Some(FileType::Flv),
                    "wmv" => Some(FileType::Wmv),
                    _ => None,
                })
                .collect()
        })
    }

    /// Get size limits as a tuple
    pub fn get_size_limits(&self) -> Option<(u64, Option<u64>)> {
        if self.min_size > 0 || self.max_size.is_some() {
            Some((self.min_size, self.max_size))
        } else {
            None
        }
    }
}

impl DedupArgs {
    /// Validate dedup command arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check if directory exists
        if !self.directory.exists() {
            anyhow::bail!("Directory does not exist: {}", self.directory.display());
        }

        if !self.directory.is_dir() {
            anyhow::bail!("Path is not a directory: {}", self.directory.display());
        }

        // Check for conflicting flags
        if self.verbose && self.quiet {
            anyhow::bail!("Cannot use both --verbose and --quiet flags");
        }

        // Validate file size constraints
        if let Some(max_size) = self.max_size {
            if max_size <= self.min_size {
                anyhow::bail!("Maximum file size must be greater than minimum file size");
            }
        }

        // Validate worker count
        if self.workers > 1000 {
            anyhow::bail!("Worker count too high: {}", self.workers);
        }

        Ok(())
    }

    /// Get the effective number of workers (0 means auto-detect)
    pub fn get_worker_count(&self) -> usize {
        if self.workers == 0 {
            // Auto-detect based on CPU cores
            num_cpus::get()
        } else {
            self.workers
        }
    }

    /// Get file types filter if specified
    pub fn get_file_types(&self) -> Option<Vec<crate::types::FileType>> {
        use crate::types::FileType;

        self.types.as_ref().map(|types| {
            types
                .iter()
                .filter_map(|t| match t.to_lowercase().as_str() {
                    "jpg" | "jpeg" => Some(FileType::Jpeg),
                    "png" => Some(FileType::Png),
                    "heic" | "heif" => Some(FileType::Heic),
                    "raw" | "cr2" | "nef" | "arw" | "dng" => Some(FileType::Raw),
                    "gif" => Some(FileType::Gif),
                    "bmp" => Some(FileType::Bmp),
                    "tiff" | "tif" => Some(FileType::Tiff),
                    "webp" => Some(FileType::Webp),
                    "mp4" | "m4v" => Some(FileType::Mp4),
                    "mov" => Some(FileType::Mov),
                    "avi" => Some(FileType::Avi),
                    "mkv" => Some(FileType::Mkv),
                    "webm" => Some(FileType::Webm),
                    "flv" => Some(FileType::Flv),
                    "wmv" => Some(FileType::Wmv),
                    _ => None,
                })
                .collect()
        })
    }

    /// Get size limits as a tuple
    pub fn get_size_limits(&self) -> Option<(u64, Option<u64>)> {
        if self.min_size > 0 || self.max_size.is_some() {
            Some((self.min_size, self.max_size))
        } else {
            None
        }
    }
}

impl InitDbArgs {
    /// Validate init-db command arguments
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check if directory exists
        if !self.directory.exists() {
            anyhow::bail!("Directory does not exist: {}", self.directory.display());
        }

        if !self.directory.is_dir() {
            anyhow::bail!("Path is not a directory: {}", self.directory.display());
        }

        // Check if output directory exists (we'll create the database file there)
        if !self.output.exists() {
            anyhow::bail!("Output directory does not exist: {}", self.output.display());
        }

        if !self.output.is_dir() {
            anyhow::bail!("Output path is not a directory: {}", self.output.display());
        }

        // Validate file size constraints
        if let Some(max_size) = self.max_size {
            if max_size <= self.min_size {
                anyhow::bail!("Maximum file size must be greater than minimum file size");
            }
        }

        // Validate worker count
        if self.workers > 1000 {
            anyhow::bail!("Worker count too high: {}", self.workers);
        }

        Ok(())
    }

    /// Get the effective number of workers (0 means auto-detect)
    pub fn get_worker_count(&self) -> usize {
        if self.workers == 0 {
            // Auto-detect based on CPU cores
            num_cpus::get()
        } else {
            self.workers
        }
    }

    /// Get file types filter if specified
    pub fn get_file_types(&self) -> Option<Vec<crate::types::FileType>> {
        use crate::types::FileType;

        self.types.as_ref().map(|types| {
            types
                .iter()
                .filter_map(|t| match t.to_lowercase().as_str() {
                    "jpg" | "jpeg" => Some(FileType::Jpeg),
                    "png" => Some(FileType::Png),
                    "heic" | "heif" => Some(FileType::Heic),
                    "raw" | "cr2" | "nef" | "arw" | "dng" => Some(FileType::Raw),
                    "gif" => Some(FileType::Gif),
                    "bmp" => Some(FileType::Bmp),
                    "tiff" | "tif" => Some(FileType::Tiff),
                    "webp" => Some(FileType::Webp),
                    "mp4" | "m4v" => Some(FileType::Mp4),
                    "mov" => Some(FileType::Mov),
                    "avi" => Some(FileType::Avi),
                    "mkv" => Some(FileType::Mkv),
                    "webm" => Some(FileType::Webm),
                    "flv" => Some(FileType::Flv),
                    "wmv" => Some(FileType::Wmv),
                    _ => None,
                })
                .collect()
        })
    }

    /// Get size limits as a tuple
    pub fn get_size_limits(&self) -> Option<(u64, Option<u64>)> {
        if self.min_size > 0 || self.max_size.is_some() {
            Some((self.min_size, self.max_size))
        } else {
            None
        }
    }
}
