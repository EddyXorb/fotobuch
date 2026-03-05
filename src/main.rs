//! Photobook layout solver using slicing tree genetic algorithm.

mod cli;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use photobook_solver::*;
use tracing::info;

fn main() -> Result<()> {
    setup_logging();
    let args = Args::parse();

    let request = args.into_solver_request()?;
    let _book_layout = run_solver(&request)?;

    info!("Done!");
    Ok(())
}

/// Initialize logging system with environment variable support.
fn setup_logging() {
    let log_level = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();
}
