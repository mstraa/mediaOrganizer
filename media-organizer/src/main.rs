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
    // Placeholder for main application logic
    println!("Media Organizer - Work in Progress");
    println!("Input: {:?}", args.input);
    println!("Output: {:?}", args.output);
    
    // TODO: Implement the actual organization logic
    
    Ok(())
}