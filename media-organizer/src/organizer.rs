use crate::cli::OperationMode;
use crate::types::{FileInfo, OperationResult, OrganizationPattern};
use anyhow::Result;
use chrono::{Datelike, Timelike};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// File organizer that determines destination paths and executes operations
pub struct Organizer {
    output_dir: PathBuf,
    pattern: OrganizationPattern,
    mode: OperationMode,
    dry_run: bool,
    preserve_timestamps: bool,
}

impl Organizer {
    pub fn new(
        output_dir: PathBuf,
        pattern: OrganizationPattern,
        mode: OperationMode,
        dry_run: bool,
        preserve_timestamps: bool,
    ) -> Self {
        Self {
            output_dir,
            pattern,
            mode,
            dry_run,
            preserve_timestamps,
        }
    }
    
    /// Organize a single file
    pub async fn organize_file(&self, file_info: &FileInfo) -> Result<OperationResult> {
        let dest_path = self.determine_destination(file_info)?;
        
        debug!(
            "Organizing {:?} -> {:?}",
            file_info.path, dest_path
        );
        
        if self.dry_run {
            info!("[DRY RUN] Would {} {:?} to {:?}", self.mode, file_info.path, dest_path);
            return Ok(OperationResult {
                source: file_info.path.clone(),
                destination: dest_path,
                success: true,
                error: None,
                is_duplicate: false,
            });
        }
        
        // Create destination directory
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Execute the operation
        let result = match self.mode {
            OperationMode::Copy => self.copy_file(&file_info.path, &dest_path).await,
            OperationMode::Move => self.move_file(&file_info.path, &dest_path).await,
        };
        
        match result {
            Ok(()) => {
                if self.preserve_timestamps {
                    self.preserve_timestamps_for(&file_info.path, &dest_path).await?;
                }
                
                Ok(OperationResult {
                    source: file_info.path.clone(),
                    destination: dest_path,
                    success: true,
                    error: None,
                    is_duplicate: false,
                })
            }
            Err(e) => Ok(OperationResult {
                source: file_info.path.clone(),
                destination: dest_path,
                success: false,
                error: Some(e.to_string()),
                is_duplicate: false,
            }),
        }
    }
    
    /// Determine the destination path based on the organization pattern
    fn determine_destination(&self, file_info: &FileInfo) -> Result<PathBuf> {
        let date = file_info.metadata.date_taken.as_ref()
            .unwrap_or(&file_info.modified);
        
        let subdir = match &self.pattern {
            OrganizationPattern::Year => {
                format!("{}", date.year())
            }
            OrganizationPattern::YearMonth => {
                format!("{}/{:02}", date.year(), date.month())
            }
            OrganizationPattern::YearMonthDay => {
                format!("{}/{:02}/{:02}", date.year(), date.month(), date.day())
            }
            OrganizationPattern::Type => {
                file_info.file_type.category().to_string()
            }
            OrganizationPattern::TypeYearMonth => {
                format!(
                    "{}/{}/{:02}",
                    file_info.file_type.category(),
                    date.year(),
                    date.month()
                )
            }
            OrganizationPattern::Custom(pattern) => {
                self.format_custom_pattern(pattern, file_info, date)?
            }
        };
        
        let filename = file_info.path.file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
        
        Ok(self.output_dir.join(subdir).join(filename))
    }
    
    /// Format a custom organization pattern
    fn format_custom_pattern(
        &self,
        pattern: &str,
        file_info: &FileInfo,
        date: &chrono::DateTime<chrono::Local>,
    ) -> Result<String> {
        let mut result = pattern.to_string();
        
        // Replace placeholders
        result = result.replace("{year}", &date.year().to_string());
        result = result.replace("{month}", &format!("{:02}", date.month()));
        result = result.replace("{day}", &format!("{:02}", date.day()));
        result = result.replace("{hour}", &format!("{:02}", date.hour()));
        result = result.replace("{type}", file_info.file_type.category());
        
        if let Some(make) = &file_info.metadata.camera_make {
            result = result.replace("{camera_make}", make);
        }
        
        if let Some(model) = &file_info.metadata.camera_model {
            result = result.replace("{camera_model}", model);
        }
        
        Ok(result)
    }
    
    /// Copy a file to the destination
    async fn copy_file(&self, source: &Path, dest: &Path) -> Result<()> {
        fs::copy(source, dest).await?;
        Ok(())
    }
    
    /// Move a file to the destination
    async fn move_file(&self, source: &Path, dest: &Path) -> Result<()> {
        // Try to rename first (fast if on same filesystem)
        match fs::rename(source, dest).await {
            Ok(()) => Ok(()),
            Err(_) => {
                // If rename fails, copy and delete
                fs::copy(source, dest).await?;
                fs::remove_file(source).await?;
                Ok(())
            }
        }
    }
    
    /// Preserve timestamps from source to destination
    async fn preserve_timestamps_for(&self, source: &Path, _dest: &Path) -> Result<()> {
        let _metadata = fs::metadata(source).await?;
        
        // This would require platform-specific code to properly set timestamps
        // For now, this is a placeholder
        // TODO: Implement proper timestamp preservation using platform-specific APIs
        
        Ok(())
    }
}