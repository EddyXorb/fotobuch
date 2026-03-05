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
/// Writes to stdout and to log/photobook-solver.log in the current directory.
fn setup_logging() {
    use tracing_subscriber::prelude::*;

    let log_level = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    std::fs::create_dir_all("log").ok();
    let file_appender = tracing_appender::rolling::never("log", "photobook-solver.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // _guard must be kept alive for the duration of the program
    std::mem::forget(_guard);

    let stdout_layer = tracing_subscriber::fmt::layer();
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    tracing_subscriber::registry()
        .with(log_level)
        .with(stdout_layer)
        .with(file_layer)
        .init();
}
