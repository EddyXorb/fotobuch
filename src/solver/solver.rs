//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use crate::cli::Args;
use crate::models::{Canvas, FitnessWeights, LayoutResult, Photo};
use crate::solver::{run_island_ga, GaConfig, IslandConfig};
use crate::{export_json, export_pdf, export_typst, load_photos_from_dir};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;

/// Run the complete photobook solver workflow from command-line arguments.
///
/// This is the main entry point that coordinates:
/// 1. Loading and validating photos from the input directory
/// 2. Configuring solver parameters from command-line arguments
/// 3. Running the genetic algorithm optimization
/// 4. Exporting the result in the requested format (JSON/Typst/PDF)
pub fn run_solver(args: &Args) -> Result<()> {
    let (photos, photo_paths) = load_and_validate_photos(&args.input)?;
    let (canvas, weights, ga_config, island_config, seed) = configure_solver(args);
    let centered_layout = run_optimization(&photos, &canvas, &weights, &ga_config, &island_config, seed);
    export_result(&centered_layout, &photo_paths, &args.input, &args.output)?;
    Ok(())
}

/// Load photos from directory and validate that at least one photo exists.
///
/// Returns tuple of (photos for solver, photo paths for export).
fn load_and_validate_photos(input_dir: &Path) -> Result<(Vec<Photo>, Vec<String>)> {
    info!("Loading photos from {:?}...", input_dir);
    let photo_infos = load_photos_from_dir(input_dir)
        .context("Failed to load photos")?;

    if photo_infos.is_empty() {
        anyhow::bail!("No photos found in {:?}", input_dir);
    }

    info!("Loaded {} photos", photo_infos.len());

    let photos: Vec<Photo> = photo_infos.iter().map(|pi| pi.photo.clone()).collect();
    let photo_paths: Vec<String> = photo_infos
        .iter()
        .map(|pi| pi.path.to_string_lossy().to_string())
        .collect();

    Ok((photos, photo_paths))
}

/// Configure all solver parameters from command line arguments.
///
/// Returns tuple of (Canvas, FitnessWeights, GaConfig, IslandConfig, seed).
fn configure_solver(args: &Args) -> (Canvas, FitnessWeights, GaConfig, IslandConfig, u64) {
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

    (canvas, weights, ga_config, island_config, seed)
}

/// Run genetic algorithm optimization and return centered layout.
fn run_optimization(
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    ga_config: &GaConfig,
    island_config: &IslandConfig,
    seed: u64,
) -> LayoutResult {
    info!("Running genetic algorithm...");
    let start = Instant::now();
    
    let (_best_tree, best_layout, best_fitness) = run_island_ga(
        photos,
        canvas,
        weights,
        ga_config,
        island_config,
        seed,
    );

    let elapsed = start.elapsed();
    info!("Optimization completed in {:.2}s", elapsed.as_secs_f64());
    info!("Best fitness: {:.6}", best_fitness);

    best_layout.centered()
}

/// Export layout result based on output file extension.
fn export_result(
    layout: &LayoutResult,
    photo_paths: &[String],
    input_dir: &Path,
    output_path: &PathBuf,
) -> Result<()> {
    let output_ext = output_path.extension().and_then(|s| s.to_str());
    
    match output_ext {
        Some("json") => {
            info!("Exporting to JSON: {:?}", output_path);
            export_json(layout, photo_paths, output_path)
                .context("Failed to export JSON")?;
        }
        Some("typ") => {
            info!("Exporting to Typst: {:?}", output_path);
            export_typst(layout, photo_paths, output_path)
                .context("Failed to export Typst")?;
        }
        Some("pdf") => {
            info!("Compiling to PDF: {:?}", output_path);
            export_pdf(layout, photo_paths, input_dir, output_path)
                .context("Failed to compile PDF")?;
        }
        _ => {
            anyhow::bail!(
                "Unsupported output format: {:?}. Use .json, .typ, or .pdf",
                output_ext
            );
        }
    }

    Ok(())
}
