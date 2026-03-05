//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use crate::models::{Canvas, FitnessWeights, LayoutResult, Photo, SolverRequest};
use crate::solver::{GaConfig, IslandConfig};
use super::page_layout::ga::run_island_ga;
use crate::{export_json, export_pdf, export_typst, load_photos_from_dir};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;

/// Run the complete photobook solver workflow from a solver request.
///
/// This is the main entry point that coordinates:
/// 1. Loading and validating photos from the input directory
/// 2. Running the genetic algorithm optimization with the provided configuration
/// 3. Exporting the result in the requested format (JSON/Typst/PDF)
pub fn run_solver(request: &SolverRequest) -> Result<()> {
    log_configuration(request);
    
    let (photos, photo_paths) = load_and_validate_photos(&request.input)?;
    let centered_layout = run_optimization(
        &photos,
        &request.canvas,
        &request.weights,
        &request.ga_config,
        &request.island_config,
        request.seed,
    );
    export_result(&centered_layout, &photo_paths, &request.input, &request.output)?;
    
    Ok(())
}

/// Log the solver configuration for user visibility.
fn log_configuration(request: &SolverRequest) {
    info!("Configuration:");
    info!("  Canvas: {}x{} mm, β={} mm", 
        request.canvas.width, request.canvas.height, request.canvas.beta);
    info!("  Islands: {}, Population: {}/island, Generations: {}", 
        request.island_config.islands, 
        request.ga_config.population, 
        request.ga_config.generations);
    info!("  Weights: size={}, coverage={}, bary={}, order={}", 
        request.weights.w_size, 
        request.weights.w_coverage, 
        request.weights.w_barycenter, 
        request.weights.w_order);
    info!("  Seed: {}", request.seed);
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
