use anyhow::Result;
use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod cli;
mod duplicate;
mod organizer;
mod progress;
mod scanner;
mod types;

use cli::Args;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    // Parse command line arguments
    let args = Args::parse();
    
    // Validate arguments
    args.validate()?;

    info!("Starting Media Organizer");
    info!("Input directory: {}", args.input.display());
    info!("Output directory: {}", args.output.display());
    info!("Workers: {}", args.get_worker_count());

    // TODO: Implement main logic
    // 1. Scan input directory
    // 2. Detect duplicates if enabled
    // 3. Organize files
    // 4. Execute operations (copy/move)

    match run(args).await {
        Ok(()) => {
            info!("Media organization completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Error during media organization: {}", e);
            Err(e)
        }
    }
}

async fn run(args: Args) -> Result<()> {
    use crate::progress::ProgressTracker;
    use crate::scanner::Scanner;
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
        .with_worker_threads(args.get_worker_count());
    
    // Start scanning phase
    progress.start_scanning(None);
    progress.enable_steady_tick();
    
    // Create channel for streaming files
    let (tx, mut rx) = mpsc::channel(1000);
    
    // Start scanning in background
    let scan_handle = tokio::spawn(async move {
        scanner.scan(tx).await
    });
    
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
    
    // Start processing phase if there are files
    if !files.is_empty() {
        progress.start_processing(files.len() as u64);
        
        // TODO: Process files through duplicate detector and organizer
        for (idx, _file_info) in files.iter().enumerate() {
            progress.update_process(idx as u64 + 1, None);
            // TODO: Send to duplicate detector if enabled
            // TODO: Send to organizer
        }
        
        progress.finish_processing();
    }
    
    progress.finish(&format!("Processed {} files", file_count));
    
    if args.dry_run {
        info!("Dry run complete - no files were moved");
    }
    
    Ok(())
}