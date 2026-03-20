//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use crate::dto_models::{BookLayoutSolverConfig, CanvasConfig, LayoutPage, PhotoGroup};
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
#[derive(Debug)]
pub struct Request<'a, C: CanvasConfig> {
    /// Type of optimization to perform.
    pub request_type: RequestType,
    /// Photo groups (for both single and multi-page requests).
    pub groups: &'a [PhotoGroup],
    /// Book layout solver configuration.
    pub config: &'a BookLayoutSolverConfig,
    /// Genetic algorithm configuration.
    pub ga_config: &'a GaConfig,
    /// Canvas configuration (page size, margins, bleed, gap).
    pub canvas_config: &'a C,
}

/// The main entry point for running the photobook layout solver.
pub fn run_solver<C: CanvasConfig>(request: &Request<C>) -> Result<Vec<LayoutPage>, SolverError> {
    if request.groups.is_empty() {
        return Ok(vec![]);
    }

    let photos = Photo::from_photo_groups(request.groups);
    let canvas = Canvas::from_canvas_config(request.canvas_config);

    match request.request_type {
        RequestType::SinglePage => run_single_page(&photos, &canvas, request),
        RequestType::MultiPage => run_multi_page(&photos, &canvas, request),
    }
}

fn run_single_page<C: CanvasConfig>(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request<C>,
) -> Result<Vec<LayoutPage>, SolverError> {
    let ga_result = page_layout_solver::run_ga(photos, canvas, request.ga_config);
    let layout_page = ga_result
        .layout
        .to_layout_page(0, photos, request.canvas_config);
    Ok(vec![layout_page])
}

fn run_multi_page<C: CanvasConfig>(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request<C>,
) -> Result<Vec<LayoutPage>, SolverError> {
    let book_layout =
        book_layout_solver::solve_book_layout(photos, request.config, canvas, request.ga_config)?;

    let mut curr_idx = 0;
    let layout_pages: Vec<LayoutPage> = book_layout
        .pages
        .iter()
        .enumerate()
        .map(|(i, page)| {
            let layout_page = page.to_layout_page(
                i,
                &photos[curr_idx..curr_idx + page.placements.len()],
                request.canvas_config,
            );
            curr_idx += page.placements.len();
            layout_page
        })
        .collect();

    check_validity(photos, request, curr_idx, &layout_pages);

    Ok(layout_pages)
}

fn check_validity<C: CanvasConfig>(
    photos: &[Photo],
    request: &Request<'_, C>,
    curr_idx: usize,
    layout_pages: &[LayoutPage],
) {
    assert!(
        curr_idx == photos.len(),
        "All photos should be assigned to pages. RequestType: {:?}\nPhotos:\n{}\nPages:\n{}",
        request.request_type,
        photos
            .iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join("\n"),
        layout_pages
            .iter()
            .map(|p| format!("{:?}", p))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
