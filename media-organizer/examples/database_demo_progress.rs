use media_organizer::duplicate::DuplicateDetector;
use media_organizer::cli::DuplicateStrategy;
use media_organizer::types::{FileType, FileInfo, MediaMetadata};
use media_organizer::progress::ProgressTracker;
use chrono::Local;
use std::path::Path;
use tokio::fs;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    media_organizer::setup_logging(false);
    
    // Create test directories
    let test_dir = Path::new("/tmp/media_organizer_test");
    let output_dir = test_dir.join("output");
    let input_dir = test_dir.join("input");
    
    fs::create_dir_all(&output_dir).await?;
    fs::create_dir_all(&input_dir).await?;
    
    println!("🔄 First run - Creating initial database with progress tracking\n");
    
    // Create some test files in output directory
    for i in 1..=100 {
        let file_path = output_dir.join(format!("existing_{:03}.jpg", i));
        fs::write(&file_path, format!("file content {}", i)).await?;
    }
    
    // Initialize progress tracker
    let mut progress = ProgressTracker::new(false);
    progress.enable_steady_tick();
    
    // Initialize duplicate detector with database and progress
    let mut detector = DuplicateDetector::with_database_and_progress(
        DuplicateStrategy::Skip,
        &output_dir,
        &progress
    ).await?;
    
    // Scan output directory to build initial database with progress
    let file_types = vec![FileType::Jpeg, FileType::Png];
    detector.scan_output_directory_with_progress(&output_dir, &file_types, Some(&progress)).await?;
    
    let stats = detector.get_statistics();
    println!("\n✅ Initial scan complete:");
    println!("   - Files in database: {}", stats.database_entries);
    println!("   - Files in output: {}", stats.existing_in_output);
    
    // Save database
    detector.save_database().await?;
    println!("💾 Database saved to: {}", output_dir.join("db.mediaorg").display());
    
    // Simulate second run with new files
    println!("\n🔄 Second run - Loading existing database");
    
    // Add new files to input directory
    for i in 1..=20 {
        let file_path = input_dir.join(format!("new_{:02}.jpg", i));
        fs::write(&file_path, format!("new content {}", i)).await?;
    }
    
    // Add some duplicate files
    for i in 1..=5 {
        let duplicate_path = input_dir.join(format!("duplicate_of_existing_{}.jpg", i));
        fs::write(&duplicate_path, format!("file content {}", i)).await?;
    }
    
    // Modify some existing files to test rehashing
    for i in 1..=10 {
        let file_path = output_dir.join(format!("existing_{:03}.jpg", i));
        fs::write(&file_path, format!("modified content {}", i)).await?;
    }
    
    // Create new detector that loads existing database with progress
    let mut detector2 = DuplicateDetector::with_database_and_progress(
        DuplicateStrategy::Skip,
        &output_dir,
        &progress
    ).await?;
    
    // Re-scan to verify database was loaded and hash modified files
    println!("\n📊 Scanning for new and modified files...");
    detector2.scan_output_directory_with_progress(&output_dir, &file_types, Some(&progress)).await?;
    
    let stats2 = detector2.get_statistics();
    println!("\n✅ Database updated:");
    println!("   - Files in database: {}", stats2.database_entries);
    println!("   - Files already hashed: {}", stats2.existing_in_output);
    
    // Check the duplicate files
    println!("\n🔍 Testing duplicate detection:");
    for i in 1..=5 {
        let duplicate_path = input_dir.join(format!("duplicate_of_existing_{}.jpg", i));
        let mut duplicate_info = FileInfo {
            path: duplicate_path.clone(),
            file_type: FileType::Jpeg,
            size: format!("file content {}", i).len() as u64,
            modified: Local::now(),
            created: Some(Local::now()),
            hash: None,
            metadata: MediaMetadata::default(),
        };
        
        let should_skip = detector2.check_duplicate(&mut duplicate_info).await?;
        println!("   - File: {} -> Duplicate: {}", duplicate_path.file_name().unwrap().to_string_lossy(), should_skip);
    }
    
    progress.finish("Demo complete");
    
    // Clean up
    fs::remove_dir_all(test_dir).await?;
    println!("\n🧹 Test directories cleaned up");
    
    Ok(())
}