use media_organizer::cli::{Args, DuplicateStrategy, OperationMode};
use std::fs;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_full_pipeline_with_duplicates() {
    // Create temporary directories
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    // Create test files
    let test_content = vec![0u8; 1024]; // 1KB of data
    fs::write(input_dir.path().join("photo1.jpg"), &test_content).unwrap();
    fs::write(input_dir.path().join("photo2.jpg"), &test_content).unwrap(); // Duplicate
    fs::write(input_dir.path().join("photo3.png"), b"unique content").unwrap();
    fs::write(input_dir.path().join("video.mp4"), b"video content").unwrap();

    // Create args for the test
    let args = Args {
        input: input_dir.path().to_path_buf(),
        output: output_dir.path().to_path_buf(),
        pattern: "year/month".to_string(),
        mode: OperationMode::Copy,
        workers: 0,
        verbose: true,
        quiet: true, // Suppress progress bars in tests
        dry_run: false,
        detect_duplicates: true,
        duplicate_strategy: DuplicateStrategy::Skip,
        types: None,
        exclude: vec![],
        min_size: 0,
        max_size: None,
        preserve_timestamps: false,
        config: None,
        report_path: None,
        follow_links: false,
        log_file: None,
        report: false,
        json: false,
    };

    // Run the main function
    let result = media_organizer::run_with_args(args).await;
    assert!(result.is_ok(), "Pipeline should complete successfully");

    // Verify files were organized (excluding database file)
    let organized_files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.file_name().to_str().unwrap().contains("db.mediaorg"))
        .collect();

    // Should have 3 files (1 duplicate was skipped)
    assert_eq!(organized_files.len(), 3);

    // Verify files are in year/month folders
    let has_year_month_structure = organized_files.iter().any(|entry| {
        let path = entry.path();
        let components: Vec<_> = path.components().collect();
        // Check if path contains year/month pattern
        components.windows(2).any(|w| {
            if let (Some(year), Some(month)) =
                (w[0].as_os_str().to_str(), w[1].as_os_str().to_str())
            {
                year.parse::<u32>().is_ok() && month.parse::<u32>().is_ok()
            } else {
                false
            }
        })
    });

    assert!(
        has_year_month_structure,
        "Files should be organized in year/month folders"
    );
}

#[tokio::test]
async fn test_dry_run_mode() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    // Create test files
    fs::write(input_dir.path().join("photo.jpg"), b"photo data").unwrap();
    fs::write(input_dir.path().join("video.mp4"), b"video data").unwrap();

    // Create args for dry run
    let args = Args {
        input: input_dir.path().to_path_buf(),
        output: output_dir.path().to_path_buf(),
        pattern: "type".to_string(),
        mode: OperationMode::Move,
        workers: 0,
        verbose: false,
        quiet: true,
        dry_run: true, // Dry run mode
        detect_duplicates: false,
        duplicate_strategy: DuplicateStrategy::Skip,
        types: None,
        exclude: vec![],
        min_size: 0,
        max_size: None,
        preserve_timestamps: false,
        config: None,
        report_path: None,
        follow_links: false,
        log_file: None,
        report: false,
        json: false,
    };

    // Run the main function
    let result = media_organizer::run_with_args(args).await;
    assert!(result.is_ok(), "Dry run should complete successfully");

    // Verify no files were actually moved (excluding database file)
    let output_files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.file_name().to_str().unwrap().contains("db.mediaorg"))
        .collect();

    assert_eq!(
        output_files.len(),
        0,
        "No files should be moved in dry run mode"
    );

    // Verify source files still exist
    assert!(input_dir.path().join("photo.jpg").exists());
    assert!(input_dir.path().join("video.mp4").exists());
}

#[tokio::test]
async fn test_duplicate_rename_strategy() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    // Create duplicate files
    let test_content = b"same content";
    fs::write(input_dir.path().join("photo1.jpg"), test_content).unwrap();
    fs::write(input_dir.path().join("photo2.jpg"), test_content).unwrap();
    fs::write(input_dir.path().join("photo3.jpg"), test_content).unwrap();

    // Create args with rename strategy
    let args = Args {
        input: input_dir.path().to_path_buf(),
        output: output_dir.path().to_path_buf(),
        pattern: "type".to_string(),
        mode: OperationMode::Copy,
        workers: 0,
        verbose: false,
        quiet: true,
        dry_run: false,
        detect_duplicates: true,
        duplicate_strategy: DuplicateStrategy::Rename, // Rename duplicates
        types: None,
        exclude: vec![],
        min_size: 0,
        max_size: None,
        preserve_timestamps: false,
        config: None,
        report_path: None,
        follow_links: false,
        log_file: None,
        report: false,
        json: false,
    };

    // Run the main function
    let result = media_organizer::run_with_args(args).await;
    assert!(result.is_ok(), "Pipeline should complete successfully");

    // Verify all files were copied (with renaming) excluding database file
    let output_files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.file_name().to_str().unwrap().contains("db.mediaorg"))
        .collect();

    assert_eq!(
        output_files.len(),
        3,
        "All files should be copied with rename strategy"
    );
}

#[tokio::test]
async fn test_cross_directory_duplicate_detection() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    // Create existing files in output directory
    let existing_content = b"existing file content";
    let output_images_dir = output_dir.path().join("images");
    fs::create_dir_all(&output_images_dir).unwrap();
    fs::write(output_images_dir.join("existing.jpg"), existing_content).unwrap();

    // Create input files - one duplicate of existing, one new
    fs::write(input_dir.path().join("duplicate.jpg"), existing_content).unwrap();
    fs::write(input_dir.path().join("new.jpg"), b"new file content").unwrap();

    // Create args with duplicate detection
    let args = Args {
        input: input_dir.path().to_path_buf(),
        output: output_dir.path().to_path_buf(),
        pattern: "type".to_string(),
        mode: OperationMode::Copy,
        workers: 0,
        verbose: true,
        quiet: true,
        dry_run: false,
        detect_duplicates: true,
        duplicate_strategy: DuplicateStrategy::Skip,
        types: None,
        exclude: vec![],
        min_size: 0,
        max_size: None,
        preserve_timestamps: false,
        config: None,
        report_path: None,
        follow_links: false,
        log_file: None,
        report: false,
        json: false,
    };

    // Run the main function
    let result = media_organizer::run_with_args(args).await;
    assert!(result.is_ok(), "Pipeline should complete successfully");

    // Count files in output after operation (excluding database file)
    let output_files: Vec<_> = walkdir::WalkDir::new(output_dir.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| !e.file_name().to_str().unwrap().contains("db.mediaorg"))
        .collect();

    // Should have 2 files total: existing.jpg + new.jpg (duplicate.jpg was skipped)
    assert_eq!(
        output_files.len(),
        2,
        "Should have 2 files: existing file + new file (duplicate was skipped)"
    );

    // Verify the new file was copied
    let new_file_exists = output_files.iter().any(|entry| {
        entry.file_name().to_str().unwrap().contains("new")
    });
    assert!(new_file_exists, "New file should have been copied");

    // Verify no duplicate of existing file was created
    let duplicate_count = output_files.iter().filter(|entry| {
        if let Ok(content) = fs::read(entry.path()) {
            content == existing_content
        } else {
            false
        }
    }).count();
    
    assert_eq!(
        duplicate_count,
        1,
        "Should only have one file with the existing content"
    );
}
