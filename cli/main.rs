//! Photobook layout solver using slicing tree genetic algorithm.

mod cli;

use clap::Parser;
use cli::{Cli, Execute};
use std::process::ExitCode;

fn main() -> ExitCode {
    let _guard = setup_logging();

    let cli = Cli::parse();
    match cli.command.execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report_error(&err);
            ExitCode::FAILURE
        }
    }
}

/// Print the error, every cause behind it, and the hint for its condition.
///
/// `Display` on an `anyhow::Error` shows only the outermost message, so the
/// causes are walked explicitly — otherwise context such as which file failed
/// to parse would never reach the user.
fn report_error(err: &anyhow::Error) {
    eprintln!("error: {err}");
    for cause in err.chain().skip(1) {
        eprintln!("  caused by: {cause}");
    }
    if let Some(hint) = cli::hints::hint_for(err) {
        eprintln!("hint: {hint}");
    }
}

/// Initialize logging system with environment variable support.
/// Writes to stdout and to log/photobook-solver.log in the current directory.
fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
    use tracing_subscriber::prelude::*;

    let log_level = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    std::fs::create_dir_all("log").ok();
    let file_appender = tracing_appender::rolling::never("log", "photobook-solver.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::ChronoUtc::new(
            "%Y-%m-%d %H:%M:%S".to_string(),
        ));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    tracing_subscriber::registry()
        .with(log_level)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}
