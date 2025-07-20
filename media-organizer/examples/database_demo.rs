use media_organizer::duplicate::DuplicateDetector;
use media_organizer::cli::DuplicateStrategy;
use media_organizer::types::{FileType, FileInfo, MediaMetadata};
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
    
    println!("🔄 First run - Creating initial database");
    
    // Create some test files in output directory
    for i in 1..=5 {
        let file_path = output_dir.join(format!("existing_{}.jpg", i));
        fs::write(&file_path, format!("file content {}", i)).await?;
    }
    
    // Initialize duplicate detector with database
    let mut detector = DuplicateDetector::with_database(
        DuplicateStrategy::Skip,
        &output_dir
    ).await?;
    
    // Scan output directory to build initial database
    let file_types = vec![FileType::Jpeg, FileType::Png];
    detector.scan_output_directory(&output_dir, &file_types).await?;
    
    let stats = detector.get_statistics();
    println!("✅ Initial scan complete:");
    println!("   - Files in database: {}", stats.database_entries);
    println!("   - Files in output: {}", stats.existing_in_output);
    
    // Save database
    detector.save_database().await?;
    println!("💾 Database saved to: {}", output_dir.join("db.mediaorg").display());
    
    // Simulate second run with new files
    println!("\n🔄 Second run - Loading existing database");
    
    // Add new files to input directory
    for i in 1..=3 {
        let file_path = input_dir.join(format!("new_{}.jpg", i));
        fs::write(&file_path, format!("new content {}", i)).await?;
    }
    
    // Add a duplicate file
    let duplicate_path = input_dir.join("duplicate_of_existing_1.jpg");
    fs::write(&duplicate_path, "file content 1").await?;
    
    // Create new detector that loads existing database
    let mut detector2 = DuplicateDetector::with_database(
        DuplicateStrategy::Skip,
        &output_dir
    ).await?;
    
    // Re-scan to verify database was loaded
    detector2.scan_output_directory(&output_dir, &file_types).await?;
    
    let stats2 = detector2.get_statistics();
    println!("✅ Database loaded:");
    println!("   - Files in database: {}", stats2.database_entries);
    println!("   - Files already hashed: {}", stats2.existing_in_output);
    
    // Check the duplicate file
    let mut duplicate_info = FileInfo {
        path: duplicate_path.clone(),
        file_type: FileType::Jpeg,
        size: 14,
        modified: Local::now(),
        created: Some(Local::now()),
        hash: None,
        metadata: MediaMetadata::default(),
    };
    
    let should_skip = detector2.check_duplicate(&mut duplicate_info).await?;
    println!("\n🔍 Duplicate detection test:");
    println!("   - File: {}", duplicate_path.display());
    println!("   - Is duplicate: {}", should_skip);
    println!("   - Hash: {:?}", duplicate_info.hash);
    
    // Clean up
    fs::remove_dir_all(test_dir).await?;
    println!("\n🧹 Test directories cleaned up");
    
    Ok(())
}