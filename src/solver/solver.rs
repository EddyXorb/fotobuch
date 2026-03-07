//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use super::book_layout_solver;
use crate::solver::prelude::*;
use crate::{load_photos_from_dir};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::info;
/// Run the complete photobook solver workflow from a solver request.
///
/// This is the main entry point that coordinates:
/// 1. Loading and validating photos from the input directory
/// 2. Running the book layout solver to distribute photos across pages
/// 3. Exporting the result in the requested format (JSON/Typst/PDF)
///
/// Returns the generated `BookLayout` for inspection and testing.
pub fn run_solver(request: &SolverRequest) -> Result<BookLayout> {
    log_configuration(request);

    let (photos, photo_paths) = load_and_validate_photos(&request.input)?;
    let book_layout = run_optimization(&photos, &request.canvas, &request.ga_config)?;
    export_result(&book_layout, &photo_paths, &request.input, &request.output)?;

    Ok(book_layout)
}

/// Log the solver configuration for user visibility.
fn log_configuration(request: &SolverRequest) {
    let islands = request.ga_config.island_config.islands;

    info!("Configuration:");
    info!(
        "  Canvas: {}x{} mm, β={} mm",
        request.canvas.width, request.canvas.height, request.canvas.beta
    );
    info!(
        "  Islands: {}, Population: {}/island, Generations: {}",
        islands, request.ga_config.population, request.ga_config.generations
    );
    info!(
        "  Weights: size={}, coverage={}, bary={}, order={}",
        request.ga_config.weights.w_size,
        request.ga_config.weights.w_coverage,
        request.ga_config.weights.w_barycenter,
        request.ga_config.weights.w_order
    );
    info!("  Seed: {}", request.ga_config.seed);
}

/// Load photos from directory and validate that at least one photo exists.
///
/// Returns tuple of (photos for solver, photo paths for export).
fn load_and_validate_photos(input_dir: &Path) -> Result<(Vec<Photo>, Vec<String>)> {
    info!("Loading photos from {:?}...", input_dir);
    let photo_infos = load_photos_from_dir(input_dir).context("Failed to load photos")?;

    if photo_infos.is_empty() {
        anyhow::bail!("No photos found in {:?}", input_dir);
    }

    for (idx, info) in photo_infos.iter().enumerate() {
        info!(
            "  Photo {}: {} (aspect ratio {:.2}, dimensions: {:?})",
            idx,
            info.path.file_name().unwrap_or_default().to_string_lossy(),
            info.photo.aspect_ratio,
            info.photo.dimensions
        );
    }

    info!("Loaded {} photos", photo_infos.len());

    let photos: Vec<Photo> = photo_infos.iter().map(|pi| pi.photo.clone()).collect();
    let photo_paths: Vec<String> = photo_infos
        .iter()
        .map(|pi| pi.path.to_string_lossy().to_string())
        .collect();

    Ok((photos, photo_paths))
}

/// Run book layout optimization and return the result.
fn run_optimization(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> Result<BookLayout> {
    info!("Running book layout solver...");
    let start = Instant::now();

    // Create default parameters for the MIP+LocalSearch solver
    // TODO: Make these configurable via SolverRequest
    let params = book_layout_solver::Params {
        page_target: 5,
        page_min: 3,
        page_max: 10,
        photos_per_page_min: 5,
        photos_per_page_max: 15,
        group_max_per_page: 3,
        group_min_photos: 3,
        weight_even: 1.0,
        weight_split: 2.0,
        weight_pages: 0.5,
        search_timeout: Duration::from_secs(10),
        max_coverage_cost: 0.1,
    };

    let book_layout = book_layout_solver::solve(photos, &params, canvas, ga_config)
        .context("Book layout solver failed")?;

    let elapsed = start.elapsed();
    info!("Optimization completed in {:.2}s", elapsed.as_secs_f64());
    info!(
        "Generated {} page(s) with {} total photos",
        book_layout.page_count(),
        book_layout.total_photo_count()
    );

    Ok(book_layout)
}

/// Export book layout result based on output file extension.
///
/// Currently exports only the first page. Future versions will support
/// multi-page export for PDF/Typst formats.
fn export_result(
    book_layout: &BookLayout,
    photo_paths: &[String],
    input_dir: &Path,
    output_path: &PathBuf,
) -> Result<()> {
    // Export only the first page (multi-page export planned for future release)
    // See: book_layout_solver.rs for multi-page layout implementation status
    let first_page = book_layout
        .pages
        .first()
        .context("Book layout has no pages")?;

    let output_ext = output_path.extension().and_then(|s| s.to_str());

    match output_ext {
        Some("json") => {
            info!("Exporting to JSON: {:?}", output_path);
            export_json(first_page, photo_paths, output_path).context("Failed to export JSON")?;
        }
        Some("typ") => {
            info!("Exporting to Typst: {:?}", output_path);
            export_typst(first_page, photo_paths, output_path).context("Failed to export Typst")?;
        }
        Some("pdf") => {
            info!("Compiling to PDF: {:?}", output_path);
            export_pdf(first_page, photo_paths, input_dir, output_path)
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
