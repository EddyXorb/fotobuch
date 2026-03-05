//! Photobook layout solver using slicing tree genetic algorithm.

use anyhow::{Context, Result};
use clap::Parser;
use photobook_solver::*;
use std::path::PathBuf;
use std::time::Instant;
use tracing::info;

/// Photobook layout solver: optimizes photo placement on a canvas using genetic algorithms.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Root directory containing photo subdirectories
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path (extension determines format: .json or .typ)
    #[arg(short, long, default_value = "layout.json")]
    output: PathBuf,

    // === Canvas Parameters ===
    /// Canvas width in mm
    #[arg(long, default_value_t = 297.0)]
    width: f64,

    /// Canvas height in mm
    #[arg(long, default_value_t = 210.0)]
    height: f64,

    /// Gap between photos in mm
    #[arg(long, default_value_t = 5.0)]
    beta: f64,

    /// Bleed over paper edge in mm
    #[arg(long, default_value_t = 0.0)]
    bleed: f64,

    // === GA Parameters ===
    /// Population size per island
    #[arg(long, default_value_t = 300)]
    population: usize,

    /// Maximum generations
    #[arg(long, default_value_t = 100)]
    generations: usize,

    /// Mutation rate (0.0-1.0)
    #[arg(long, default_value_t = 0.2)]
    mutation_rate: f64,

    /// Crossover rate (0.0-1.0)
    #[arg(long, default_value_t = 0.7)]
    crossover_rate: f64,

    // === Island Model Parameters ===
    /// Number of islands (default: number of CPU cores)
    #[arg(long)]
    islands: Option<usize>,

    /// Generations between migrations
    #[arg(long, default_value_t = 5)]
    migration_interval: usize,

    /// Number of migrants per migration
    #[arg(long, default_value_t = 2)]
    migrants: usize,

    /// Timeout in seconds (0 = no timeout)
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    /// Random seed for reproducibility
    #[arg(long)]
    seed: Option<u64>,

    // === Fitness Weights ===
    /// Weight for size distribution cost
    #[arg(long, default_value_t = 1.0)]
    w_size: f64,

    /// Weight for canvas coverage cost
    #[arg(long, default_value_t = 0.15)]
    w_coverage: f64,

    /// Weight for barycenter cost
    #[arg(long, default_value_t = 0.5)]
    w_barycenter: f64,

    /// Weight for reading order cost
    #[arg(long, default_value_t = 0.3)]
    w_order: f64,

    /// Verbose output (progress and fitness)
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    // Initialize logging
    let log_level = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    let args = Args::parse();

    // 1. Load photos
    info!("Loading photos from {:?}...", args.input);
    let photo_infos = load_photos_from_dir(&args.input)
        .context("Failed to load photos")?;

    if photo_infos.is_empty() {
        anyhow::bail!("No photos found in {:?}", args.input);
    }

    info!("Loaded {} photos", photo_infos.len());

    // Extract photos and paths
    let photos: Vec<Photo> = photo_infos.iter().map(|pi| pi.photo.clone()).collect();
    let photo_paths: Vec<String> = photo_infos
        .iter()
        .map(|pi| pi.path.to_string_lossy().to_string())
        .collect();

    // 2. Configure solver
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

    info!("Configuration:");
    info!("  Canvas: {}x{} mm, β={} mm", canvas.width, canvas.height, canvas.beta);
    info!("  Islands: {}, Population: {}/island, Generations: {}", 
        island_config.islands, ga_config.population, ga_config.generations);
    info!("  Weights: size={}, coverage={}, bary={}, order={}", 
        weights.w_size, weights.w_coverage, weights.w_barycenter, weights.w_order);
    info!("  Seed: {}", seed);

    // 3. Run solver
    info!("Running genetic algorithm...");
    let start = Instant::now();
    
    let (best_tree, best_layout, best_fitness) = run_island_ga(
        &photos,
        &canvas,
        &weights,
        &ga_config,
        &island_config,
        seed,
    );

    let elapsed = start.elapsed();
    info!("Optimization completed in {:.2}s", elapsed.as_secs_f64());
    info!("Best fitness: {:.6}", best_fitness);
    info!("Tree nodes: {}, Leaf count: {}", best_tree.len(), best_tree.leaf_count());

    // 4. Export result
    let output_ext = args.output.extension().and_then(|s| s.to_str());
    
    match output_ext {
        Some("json") => {
            info!("Exporting to JSON: {:?}", args.output);
            export_json(&best_layout, &args.output)
                .context("Failed to export JSON")?;
        }
        Some("typ") => {
            info!("Exporting to Typst: {:?}", args.output);
            export_typst(&best_layout, &photo_paths, &args.output)
                .context("Failed to export Typst")?;
        }
        _ => {
            anyhow::bail!(
                "Unsupported output format: {:?}. Use .json or .typ",
                output_ext
            );
        }
    }

    info!("Done!");
    Ok(())
}
