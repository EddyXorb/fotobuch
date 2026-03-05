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

    let request = build_solver_request(args)?;
    run_solver(&request)?;

    info!("Done!");
    Ok(())
}

/// Build a SolverRequest from command-line arguments.
fn build_solver_request(args: Args) -> Result<SolverRequest> {
    let canvas = Canvas::new(args.width, args.height, args.beta, args.bleed);
    
    let weights = FitnessWeights {
        w_size: args.w_size,
        w_coverage: args.w_coverage,
        w_barycenter: args.w_barycenter,
        w_order: args.w_order,
    };

    let ga_config = GaConfig {
        population: args.population,
        generations: args.generations,
        mutation_rate: args.mutation_rate,
        crossover_rate: args.crossover_rate,
        tournament_size: 3,
        elitism_ratio: 0.05,
    };

    let island_config = IslandConfig {
        islands: args.islands.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        }),
        migration_interval: args.migration_interval,
        migrants: args.migrants,
        timeout: if args.timeout > 0 {
            Some(std::time::Duration::from_secs(args.timeout))
        } else {
            None
        },
    };

    let seed = args.seed.unwrap_or_else(|| {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    Ok(SolverRequest::new(
        args.input,
        args.output,
        canvas,
        weights,
        ga_config,
        island_config,
        seed,
    ))
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
