//! Photobook layout solver using slicing tree genetic algorithm.

mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Args;
use photobook_solver::*;
use std::time::Instant;
use tracing::info;

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
    
    let (_best_tree, best_layout, best_fitness) = run_island_ga(
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

    // 4. Center layout on canvas
    let centered_layout = best_layout.centered();

    // 5. Export result
    let output_ext = args.output.extension().and_then(|s| s.to_str());
    
    match output_ext {
        Some("json") => {
            info!("Exporting to JSON: {:?}", args.output);
            export_json(&centered_layout, &photo_paths, &args.output)
                .context("Failed to export JSON")?;
        }
        Some("typ") => {
            info!("Exporting to Typst: {:?}", args.output);
            export_typst(&centered_layout, &photo_paths, &args.output)
                .context("Failed to export Typst")?;
        }
        Some("pdf") => {
            info!("Compiling to PDF: {:?}", args.output);
            export_pdf(&centered_layout, &photo_paths, &args.input, &args.output)
                .context("Failed to compile PDF")?;
        }
        _ => {
            anyhow::bail!(
                "Unsupported output format: {:?}. Use .json, .typ, or .pdf",
                output_ext
            );
        }
    }

    info!("Done!");
    Ok(())
}
