//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use super::book_layout_solver::solve_book_layout;
use crate::models::{BookLayout, GaConfig, Photo, SolverRequest};
use crate::{export_json, export_pdf, export_typst, load_photos_from_dir};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
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
    let book_layout = run_optimization(&photos, &request.canvas, &request.ga_config);
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
    canvas: &crate::models::Canvas,
    ga_config: &GaConfig,
) -> BookLayout {
    info!("Running book layout solver...");
    let start = Instant::now();

    let book_layout = solve_book_layout(photos, canvas, ga_config);

    let elapsed = start.elapsed();
    info!("Optimization completed in {:.2}s", elapsed.as_secs_f64());
    info!(
        "Generated {} page(s) with {} total photos",
        book_layout.page_count(),
        book_layout.total_photo_count()
    );

    book_layout
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
