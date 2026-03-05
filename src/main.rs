//! Photobook layout solver using slicing tree genetic algorithm.

use anyhow::Result;
use clap::Parser;
use photobook_solver::{run_solver, Args};
use tracing::info;

fn main() -> Result<()> {
    setup_logging();
    let args = Args::parse();

    run_solver(&args)?;

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
