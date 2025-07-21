use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

mod cli;
mod database;
mod dedup;
mod duplicate;
mod organizer;
mod progress;
mod scanner;
mod types;

use cli::{Cli, Commands, InitDbArgs};
use progress::setup_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    match cli.command {
        Commands::Organize(args) => {
            // Initialize logging based on verbose flag
            setup_logging(args.verbose);

            // Validate arguments
            args.validate()?;

            info!("Starting Media Organizer");
            info!("Input directory: {}", args.input.display());
            info!("Output directory: {}", args.output.display());
            info!("Workers: {}", args.get_worker_count());

            match run_organize(args).await {
                Ok(()) => {
                    info!("Media organization completed successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Error during media organization: {}", e);
                    Err(e)
                },
            }
        },
        Commands::Dedup(args) => {
            // Initialize logging based on verbose flag
            setup_logging(args.verbose);

            // Validate arguments
            args.validate()?;

            info!("Starting Deduplication");
            info!("Directory: {}", args.directory.display());
            info!("Workers: {}", args.get_worker_count());

            let mut deduplicator = dedup::Deduplicator::new(args);
            match deduplicator.run().await {
                Ok(()) => {
                    info!("Deduplication completed successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Error during deduplication: {}", e);
                    Err(e)
                },
            }
        },
        Commands::InitDb(args) => {
            // Initialize logging based on verbose flag
            setup_logging(args.verbose);

            // Validate arguments
            args.validate()?;

            info!("Starting Database Initialization");
            info!("Directory: {}", args.directory.display());
            info!("Workers: {}", args.get_worker_count());

            match run_init_db(args).await {
                Ok(()) => {
                    info!("Database initialization completed successfully");
                    Ok(())
                },
                Err(e) => {
                    error!("Error during database initialization: {}", e);
                    Err(e)
                },
            }
        },
    }
}

async fn run_organize(args: cli::OrganizeArgs) -> Result<()> {
    use crate::duplicate::DuplicateDetector;
    use crate::organizer::Organizer;
    use crate::progress::ProgressTracker;
    use crate::scanner::Scanner;
    use crate::types::OperationResult;
    use tokio::sync::mpsc;

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
        progress.increment_files_scanned(1);
        progress.add_bytes_processed(file_info.size);

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
                
                match detector.scan_output_directory(&args.output, &file_types).await {
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
                            progress.increment_files_hashed(1);
                            if should_skip {
                                skipped_count += 1;
                                progress.increment_duplicates(1);
                                if args.verbose {
                                    info!("Skipping duplicate: {:?}", file_info.path);
                                }
                            } else {
                                processed_files.push(file_info);
                            }
                        },
                        Err(e) => {
                            error!("Error checking duplicate for {:?}: {}", file_info.path, e);
                            progress.increment_errors();
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
                            progress.increment_files_organized(1);
                            progress.report_success(&format!("Organized {:?}", result.source.file_name().unwrap_or_default()));
                            if args.verbose && !args.dry_run {
                                info!("Organized: {:?} -> {:?}", result.source, result.destination);
                            }
                        } else {
                            error_count += 1;
                            progress.increment_errors();
                            if let Some(error) = &result.error {
                                progress.report_error(&format!("{:?}: {}", result.source.file_name().unwrap_or_default(), error));
                                error!("Failed to organize {:?}: {}", result.source, error);
                            }
                        }
                        operations.push(result);
                    },
                    Err(e) => {
                        error_count += 1;
                        progress.increment_errors();
                        progress.report_error(&format!("{:?}: {}", file_info.path.file_name().unwrap_or_default(), e));
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

            // Print comprehensive summary
            progress.print_summary();

            // If requested, output JSON report
            if args.json {
                let report = progress.generate_report();
                if let Ok(json) = progress::report_as_json(&report) {
                    println!("\n{json}");
                }
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

async fn run_init_db(args: InitDbArgs) -> Result<()> {
    use crate::database::HashDatabase;
    use crate::duplicate::DuplicateDetector;
    use crate::progress::ProgressTracker;
    use crate::scanner::Scanner;
    use serde_json;
    use tokio::sync::mpsc;

    // Initialize progress tracker
    let mut progress = ProgressTracker::new(false);

    // Load existing database if present
    let mut db = HashDatabase::load(&args.directory).await?;
    
    if args.cleanup {
        info!("Cleaning up obsolete entries from database...");
        let removed = db.cleanup().await?;
        info!("Removed {} obsolete entries", removed);
    }

    // Create scanner
    let mut scanner = Scanner::new(args.directory.clone());

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
        progress.increment_files_scanned(1);
        progress.add_bytes_processed(file_info.size);

        if args.verbose {
            info!("Found: {:?} ({} bytes)", file_info.path, file_info.size);
        }

        files.push(file_info);
    }

    // Wait for scanning to complete
    scan_handle.await??;
    progress.finish_scanning(file_count);

    // Process files for hashing
    if !files.is_empty() {
        progress.start_duplicate_detection(files.len() as u64);

        // Create a duplicate detector just for hashing
        let hash_workers = args.hash_workers.unwrap_or_else(|| args.get_worker_count());
        let mut detector = DuplicateDetector::new(crate::cli::DuplicateStrategy::Skip)
            .with_hash_workers(hash_workers);

        let mut new_hashes = 0;
        let mut updated_hashes = 0;

        // Process each file
        for (idx, file_info) in files.into_iter().enumerate() {
            progress.update_duplicate_detection(idx as u64 + 1);

            // Check if we already have a hash for this file
            let needs_update = if let Some(existing) = db.get(&file_info.path) {
                // Check if file has been modified
                let file_modified = file_info.modified.timestamp();
                existing.modified != file_modified || existing.size != file_info.size
            } else {
                true
            };

            if needs_update {
                // Compute hash
                match detector.compute_hash(&file_info.path).await {
                    Ok(hash_value) => {
                            // Check if this is an update or new entry
                            if db.get(&file_info.path).is_some() {
                                updated_hashes += 1;
                            } else {
                                new_hashes += 1;
                            }
                            
                            // Insert into database
                            db.insert(
                                file_info.path.clone(),
                                hash_value,
                                file_info.size,
                                file_info.modified.timestamp(),
                            );
                            
                            progress.increment_files_hashed(1);
                    }
                    Err(e) => {
                        error!("Error computing hash for {:?}: {}", file_info.path, e);
                        progress.increment_errors();
                    }
                }
            } else {
                // File hasn't changed, skip it
                if args.verbose {
                    info!("Skipping unchanged file: {:?}", file_info.path);
                }
            }
        }

        progress.finish_duplicate_detection();

        // Save the database
        info!("Saving database to {:?}", args.directory);
        db.save(&args.directory).await?;

        // Report statistics
        let stats = db.stats();
        info!("Database statistics:");
        info!("  Total files: {}", stats.total_files);
        info!("  Unique hashes: {}", stats.unique_hashes);
        info!("  Duplicate groups: {}", stats.duplicate_groups);
        info!("  Total duplicates: {}", stats.total_duplicates);
        info!("  New hashes added: {}", new_hashes);
        info!("  Existing hashes updated: {}", updated_hashes);

        if args.json {
            let json_stats = serde_json::json!({
                "total_files": stats.total_files,
                "unique_hashes": stats.unique_hashes,
                "duplicate_groups": stats.duplicate_groups,
                "total_duplicates": stats.total_duplicates,
                "new_hashes": new_hashes,
                "updated_hashes": updated_hashes,
            });
            println!("{}", serde_json::to_string_pretty(&json_stats)?);
        }

        progress.print_summary();
    } else {
        progress.finish("No media files found");
    }

    Ok(())
}
