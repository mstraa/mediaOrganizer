#![allow(dead_code)]

pub mod cli;
pub mod database;
pub mod dedup;
pub mod duplicate;
pub mod organizer;
pub mod progress;
pub mod scanner;
pub mod types;

use anyhow::Result;
use cli::Args;

// Re-export commonly used items
pub use progress::{setup_logging, ProgressReport, ProgressTracker};

/// Run the media organizer with the given arguments
/// This is exposed for testing purposes
pub async fn run_with_args(args: Args) -> Result<()> {
    // Import the run function from main.rs
    // For now, we'll duplicate the logic here
    use crate::duplicate::DuplicateDetector;
    use crate::organizer::Organizer;
    use crate::progress::ProgressTracker;
    use crate::scanner::Scanner;
    use crate::types::OperationResult;
    use tokio::sync::mpsc;
    use tracing::{error, info};

    // Initialize progress tracker
    let mut progress = ProgressTracker::new(args.quiet);

    // Create scanner
    let mut scanner = Scanner::new(args.input.clone());

    // Configure scanner based on arguments
    if let Some(file_types) = args.get_file_types() {
        scanner = scanner.with_file_types(file_types);
    }

    if let Some((min, max)) = args.get_size_limits() {
        scanner = scanner.with_size_limits(min, max);
    }

    scanner = scanner
        .with_batch_size(1000)
        .with_worker_threads(args.get_worker_count())
        .with_exclude_patterns(args.exclude.clone())
        .with_follow_links(args.follow_links);

    // Start scanning phase
    progress.start_scanning(None);
    progress.enable_steady_tick();

    // Create channel for streaming files
    let (tx, mut rx) = mpsc::channel(1000);

    // Start scanning in background
    let scan_handle = tokio::spawn(async move { scanner.scan(tx).await });

    // Collect files as they're discovered
    let mut files = Vec::new();
    let mut file_count = 0;

    while let Some(file_info) = rx.recv().await {
        file_count += 1;
        progress.update_scan(file_count);

        if args.verbose {
            info!("Found: {:?} ({} bytes)", file_info.path, file_info.size);
        }

        files.push(file_info);
    }

    // Wait for scanning to complete
    scan_handle.await??;
    progress.finish_scanning(file_count);

    // Process files if there are any
    if !files.is_empty() {
        let mut processed_files = Vec::new();
        let mut duplicate_detector = None;
        let mut skipped_count = 0;

        // Initialize duplicate detector if enabled
        if args.detect_duplicates {
            let hash_workers = args.hash_workers.or(Some(args.get_worker_count()));
            let mut detector = DuplicateDetector::new(args.duplicate_strategy);
            if let Some(workers) = hash_workers {
                detector = detector.with_hash_workers(workers);
            }
            duplicate_detector = Some(detector);
            
            // Pre-scan output directory for existing files
            if let Some(detector) = duplicate_detector.as_mut() {
                info!("Pre-scanning output directory for existing files...");
                let file_types = args.get_file_types().unwrap_or_else(|| {
                    // Include all image and video types by default
                    vec![
                        crate::types::FileType::Jpeg,
                        crate::types::FileType::Png,
                        crate::types::FileType::Heic,
                        crate::types::FileType::Raw,
                        crate::types::FileType::Gif,
                        crate::types::FileType::Bmp,
                        crate::types::FileType::Tiff,
                        crate::types::FileType::Webp,
                        crate::types::FileType::Mp4,
                        crate::types::FileType::Mov,
                        crate::types::FileType::Avi,
                        crate::types::FileType::Mkv,
                        crate::types::FileType::Webm,
                        crate::types::FileType::Flv,
                        crate::types::FileType::Wmv,
                    ]
                });
                
                match detector.scan_output_directory_with_progress(&args.output, &file_types, Some(&progress)).await {
                    Ok(_) => {
                        let stats = detector.get_statistics();
                        if stats.existing_in_output > 0 {
                            info!("Found {} existing files in output directory", stats.existing_in_output);
                        }
                    }
                    Err(e) => {
                        error!("Error pre-scanning output directory: {}", e);
                        // Continue anyway - we'll just miss existing duplicates
                    }
                }
            }
            
            progress.start_duplicate_detection(files.len() as u64);

            // Process files through duplicate detector
            for (idx, mut file_info) in files.into_iter().enumerate() {
                progress.update_duplicate_detection(idx as u64 + 1);

                if let Some(detector) = duplicate_detector.as_mut() {
                    match detector.check_duplicate(&mut file_info).await {
                        Ok(should_skip) => {
                            if should_skip {
                                skipped_count += 1;
                                if args.verbose {
                                    info!("Skipping duplicate: {:?}", file_info.path);
                                }
                            } else {
                                processed_files.push(file_info);
                            }
                        },
                        Err(e) => {
                            error!("Error checking duplicate for {:?}: {}", file_info.path, e);
                            processed_files.push(file_info); // Process anyway
                        },
                    }
                } else {
                    processed_files.push(file_info);
                }
            }

            progress.finish_duplicate_detection();

            // Report duplicate statistics
            if let Some(detector) = &duplicate_detector {
                let stats = detector.get_statistics();
                info!(
                    "Duplicates found: {} groups with {} total duplicates",
                    stats.duplicate_groups, stats.total_duplicates
                );
                if stats.existing_in_output > 0 {
                    info!(
                        "Files already in output directory: {} (these were skipped)",
                        stats.existing_in_output
                    );
                }
            }
        } else {
            processed_files = files;
        }

        // Start organization phase
        if !processed_files.is_empty() {
            progress.start_processing(processed_files.len() as u64);

            // Create organizer
            let organizer = Organizer::new(
                args.output.clone(),
                args.get_organization_pattern(),
                args.mode,
                args.dry_run,
                args.preserve_timestamps,
            );

            let mut success_count = 0;
            let mut error_count = 0;
            let mut operations: Vec<OperationResult> = Vec::new();

            // Process files through organizer
            for (idx, file_info) in processed_files.into_iter().enumerate() {
                progress
                    .update_process(idx as u64 + 1, Some(&file_info.path.display().to_string()));

                // Handle renamed duplicates if using rename strategy
                if args.duplicate_strategy == crate::cli::DuplicateStrategy::Rename {
                    if let Some(detector) = &duplicate_detector {
                        if file_info.hash.is_some() {
                            // This file has been hashed, so it might need renaming
                            let dest_path = organizer.determine_destination(&file_info).await?;
                            if dest_path.exists() {
                                let _renamed_path =
                                    detector.get_renamed_path(&file_info.path, &dest_path);
                                // Update the organizer's destination logic would go here
                                // For now, we'll let the organizer handle it
                            }
                        }
                    }
                }

                match organizer.organize_file(&file_info).await {
                    Ok(result) => {
                        if result.success {
                            success_count += 1;
                            if args.verbose && !args.dry_run {
                                info!("Organized: {:?} -> {:?}", result.source, result.destination);
                            }
                        } else {
                            error_count += 1;
                            if let Some(error) = &result.error {
                                error!("Failed to organize {:?}: {}", result.source, error);
                            }
                        }
                        operations.push(result);
                    },
                    Err(e) => {
                        error_count += 1;
                        error!("Error organizing {:?}: {}", file_info.path, e);
                    },
                }
            }

            progress.finish_processing();

            // Generate summary
            let total_processed = success_count + error_count;
            let summary = format!(
                "Processed {total_processed} files: {success_count} successful, {error_count} errors, {skipped_count} duplicates skipped"
            );
            progress.finish(&summary);

            // Print detailed summary if verbose
            if args.verbose && !operations.is_empty() {
                info!("\nOperation Summary:");
                info!("  Total files scanned: {}", file_count);
                info!("  Files processed: {}", total_processed);
                info!("  Successful operations: {}", success_count);
                info!("  Failed operations: {}", error_count);
                info!("  Duplicates skipped: {}", skipped_count);
            }
        } else {
            progress.finish("No files to organize after duplicate detection");
        }
    } else {
        progress.finish("No media files found");
    }

    if args.dry_run {
        info!("Dry run complete - no files were actually moved or copied");
    }

    Ok(())
}
