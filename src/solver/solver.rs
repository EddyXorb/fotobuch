//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use super::book_layout_solver;
use crate::dto_models::BookLayoutSolverConfig;
use crate::load_photos_from_dir;
use crate::solver::prelude::*;
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

    // TODO: Re-enable I/O once input/output fields are added back to SolverRequest
    // let (photos, photo_paths) = load_and_validate_photos(&request.input)?;
    // let book_layout = run_optimization(&photos, &request.canvas, &request.ga_config)?;
    // export_result(&book_layout, &photo_paths, &request.input, &request.output)?;

    // For now, return an empty layout
    Ok(BookLayout::new(vec![]))
}

/// Log the solver configuration for user visibility.
fn log_configuration(request: &SolverRequest) {
    let islands = request.ga_config.islands_nr;

    info!("Configuration:");
    info!(
        "  Canvas: {}x{} mm, β={} mm",
        request.canvas.width, request.canvas.height, request.canvas.beta
    );
    info!(
        "  Islands: {}, Population: {}/island, Generations: {}",
        islands, request.ga_config.population_size, request.ga_config.max_generations
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
    let photo_files = load_photos_from_dir(input_dir).context("Failed to load photos")?;

    if photo_files.is_empty() {
        anyhow::bail!("No photos found in {:?}", input_dir);
    }

    for (idx, file) in photo_files.iter().enumerate() {
        let aspect_ratio = file.aspect_ratio();
        info!(
            "  Photo {}: {} (aspect ratio {:.2}, dimensions: {}x{})",
            idx, file.id, aspect_ratio, file.width_px, file.height_px
        );
    }

    info!("Loaded {} photos", photo_files.len());

    // Convert PhotoFile (dto_models) to Photo (solver's internal model)
    let photos: Vec<Photo> = photo_files
        .iter()
        .map(|pf| Photo {
            aspect_ratio: pf.aspect_ratio(),
            area_weight: pf.area_weight,
            group: pf.id.clone(), // Use photo ID as group for now
            timestamp: Some(pf.timestamp),
            dimensions: Some((pf.width_px, pf.height_px)),
        })
        .collect();

    let photo_paths: Vec<String> = photo_files.iter().map(|pf| pf.source.clone()).collect();

    Ok((photos, photo_paths))
}

/// Run book layout optimization and return the result.
fn run_optimization(photos: &[Photo], canvas: &Canvas, ga_config: &GaConfig) -> Result<BookLayout> {
    info!("Running book layout solver...");
    let start = Instant::now();

    // Create default parameters for the MIP+LocalSearch solver
    // TODO: Make these configurable via SolverRequest
    let params = BookLayoutSolverConfig::default();

    let book_layout = book_layout_solver::solve_book_layout(photos, &params, canvas, ga_config)
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
    _photo_paths: &[String],
    _input_dir: &Path,
    output_path: &PathBuf,
) -> Result<()> {
    // Export only the first page (multi-page export planned for future release)
    // See: book_layout_solver.rs for multi-page layout implementation status
    let _first_page = book_layout
        .pages
        .first()
        .context("Book layout has no pages")?;

    let _output_ext = output_path.extension().and_then(|s| s.to_str());

    // match output_ext {
    //     Some("json") => {
    //         info!("Exporting to JSON: {:?}", output_path);
    //         export_json(first_page, photo_paths, output_path).context("Failed to export JSON")?;
    //     }
    //     Some("typ") => {
    //         info!("Exporting to Typst: {:?}", output_path);
    //         export_typst(first_page, photo_paths, output_path).context("Failed to export Typst")?;
    //     }
    //     Some("pdf") => {
    //         info!("Compiling to PDF: {:?}", output_path);
    //         export_pdf(first_page, photo_paths, input_dir, output_path)
    //             .context("Failed to compile PDF")?;
    //     }
    //     _ => {
    //         anyhow::bail!(
    //             "Unsupported output format: {:?}. Use .json, .typ, or .pdf",
    //             output_ext
    //         );
    //     }
    // }

    Ok(())
}
