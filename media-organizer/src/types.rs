use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};

/// Supported media file types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    // Images
    Jpeg,
    Png,
    Heic,
    Raw,
    Gif,
    Bmp,
    Tiff,
    Webp,
    
    // Videos
    Mp4,
    Mov,
    Avi,
    Mkv,
    Webm,
    Flv,
    Wmv,
    
    // Other
    Unknown,
}

impl FileType {
    /// Determine file type from extension
    pub fn from_extension(path: &Path) -> Self {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        
        match ext.as_deref() {
            // Images
            Some("jpg") | Some("jpeg") => Self::Jpeg,
            Some("png") => Self::Png,
            Some("heic") | Some("heif") => Self::Heic,
            Some("raw") | Some("cr2") | Some("nef") | Some("arw") | Some("dng") => Self::Raw,
            Some("gif") => Self::Gif,
            Some("bmp") => Self::Bmp,
            Some("tiff") | Some("tif") => Self::Tiff,
            Some("webp") => Self::Webp,
            
            // Videos
            Some("mp4") | Some("m4v") => Self::Mp4,
            Some("mov") => Self::Mov,
            Some("avi") => Self::Avi,
            Some("mkv") => Self::Mkv,
            Some("webm") => Self::Webm,
            Some("flv") => Self::Flv,
            Some("wmv") => Self::Wmv,
            
            _ => Self::Unknown,
        }
    }
    
    /// Check if this is an image type
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Self::Jpeg
                | Self::Png
                | Self::Heic
                | Self::Raw
                | Self::Gif
                | Self::Bmp
                | Self::Tiff
                | Self::Webp
        )
    }
    
    /// Check if this is a video type
    pub fn is_video(&self) -> bool {
        matches!(
            self,
            Self::Mp4 | Self::Mov | Self::Avi | Self::Mkv | Self::Webm | Self::Flv | Self::Wmv
        )
    }
    
    /// Get the category name for organization
    pub fn category(&self) -> &'static str {
        if self.is_image() {
            "Images"
        } else if self.is_video() {
            "Videos"
        } else {
            "Other"
        }
    }
}

/// Information about a media file
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub modified: DateTime<Local>,
    pub created: Option<DateTime<Local>>,
    pub hash: Option<String>,
    pub metadata: MediaMetadata,
}

/// Media-specific metadata
#[derive(Debug, Clone, Default)]
pub struct MediaMetadata {
    pub date_taken: Option<DateTime<Local>>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<std::time::Duration>,
    pub location: Option<Location>,
}

/// GPS location information
#[derive(Debug, Clone)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
}

/// Organization pattern
#[derive(Debug, Clone)]
pub enum OrganizationPattern {
    Year,
    YearMonth,
    YearMonthDay,
    Type,
    TypeYearMonth,
    Custom(String),
}

impl OrganizationPattern {
    pub fn from_str(s: &str) -> Self {
        match s {
            "year" => Self::Year,
            "year/month" => Self::YearMonth,
            "year/month/day" => Self::YearMonthDay,
            "type" => Self::Type,
            "type/year/month" => Self::TypeYearMonth,
            pattern => Self::Custom(pattern.to_string()),
        }
    }
}

/// Result of a file operation
#[derive(Debug)]
pub struct OperationResult {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub success: bool,
    pub error: Option<String>,
    pub is_duplicate: bool,
}

/// Statistics for the operation
#[derive(Debug, Default)]
pub struct Statistics {
    pub files_scanned: usize,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub duplicates_found: usize,
    pub errors: usize,
    pub total_size: u64,
    pub processing_time: std::time::Duration,
}