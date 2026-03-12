//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use crate::dto_models::{BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoGroup};
use crate::solver::book_layout_solver::{self, SolverError};
use crate::solver::page_layout_solver;
use crate::solver::prelude::*;

/// Simple switch enum to select solver mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestType {
    /// Single-page layout optimization; no grouping or multi-page logic applied.
    /// Uses page_layout_solver directly on the full photo set.
    SinglePage,
    /// Multi-page book layout optimization with grouping and page assignment.
    /// Uses the full MIP + local search pipeline to distribute photos across pages
    /// and creates layouts for each page.
    MultiPage,
}

/// Request containing all data for running the solver.
pub struct Request<'a> {
    /// Type of optimization to perform.
    pub request_type: RequestType,
    /// Photo groups (for both single and multi-page requests).
    pub groups: &'a [PhotoGroup],
    /// Book layout solver configuration.
    pub config: &'a BookLayoutSolverConfig,
    /// Genetic algorithm configuration.
    pub ga_config: &'a GaConfig,
    /// Book configuration (page size, margins, etc.).
    pub book_config: &'a BookConfig,
}

/// The main entry point for running the photobook layout solver.
///
/// # Algorithm
/// 1. Validates that input is not empty
/// 2. Converts DTOs (PhotoGroup, BookConfig) to internal models (Photo, Canvas)
/// 3. Dispatches to single-page or multi-page solver based on request_type
/// 4. Converts results back to DTO (LayoutPage)
///
/// # Returns
/// Vector of LayoutPage containing photo IDs and slot positions for each page.
pub fn run_solver(request: &Request) -> Result<Vec<LayoutPage>, SolverError> {
    // 1. Validate request
    if request.groups.is_empty() {
        return Ok(vec![]);
    }

    // 2. Convert DTOs to internal models
    let photos = Photo::from_photo_groups(request.groups);
    let canvas = Canvas::from_book_config(request.book_config);

    // 3. Dispatch based on request type
    match request.request_type {
        RequestType::SinglePage => run_single_page(&photos, &canvas, request),
        RequestType::MultiPage => run_multi_page(&photos, &canvas, request),
    }
}

/// Runs single-page layout optimization.
fn run_single_page(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request,
) -> Result<Vec<LayoutPage>, SolverError> {
    // Run single-page GA solver
    let ga_result = page_layout_solver::run_ga(photos, canvas, request.ga_config);

    // Convert to DTO
    let layout_page = ga_result
        .layout
        .centered()
        .to_layout_page(1, photos, &request.book_config);

    Ok(vec![layout_page])
}

/// Runs multi-page book layout optimization.
fn run_multi_page(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request,
) -> Result<Vec<LayoutPage>, SolverError> {
    // Run book layout solver (MIP + local search)
    let book_layout =
        book_layout_solver::solve_book_layout(photos, request.config, canvas, request.ga_config)?;

    // Convert each page to DTO
    let layout_pages: Vec<LayoutPage> = book_layout
        .pages
        .iter()
        .enumerate()
        .map(|(i, page)| {
            page.centered()
                .to_layout_page(i + 1, photos, &request.book_config)
        })
        .collect();

    Ok(layout_pages)
}
