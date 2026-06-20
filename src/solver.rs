//! Core solver for the photobook layout problem using slicing trees and genetic algorithms.
//!
//! This module contains:
//! - `ga_solver`: Generic genetic algorithm implementation
//! - `page_layout_solver`: Single-page layout optimization (tree, fitness)
//! - `book_layout_solver`: Multi-page book layout optimization

pub(crate) mod algorithms;
pub(crate) mod book_layout_solver;
pub(crate) mod conversion;
pub mod cover_solver;
pub(crate) mod data_models;
pub(crate) mod page_layout_solver;
pub(crate) mod prelude;

use crate::dto_models::{BookLayoutSolverConfig, CanvasConfig, LayoutPage, PhotoGroup};
pub use book_layout_solver::SolverError;
use prelude::*;

/// Solver mode, carrying any variant-specific configuration.
#[derive(Debug, Clone, Copy)]
pub enum RequestType<'a> {
    /// Single-page layout optimization; no grouping or multi-page logic applied.
    SinglePage,
    /// Multi-page book layout optimization with grouping and page assignment.
    MultiPage { config: &'a BookLayoutSolverConfig },
}

/// Request containing all data for running the solver.
#[derive(Debug)]
pub struct Request<'a, C: CanvasConfig> {
    /// Type of optimization to perform.
    pub request_type: RequestType<'a>,
    /// Photo groups (for both single and multi-page requests).
    pub groups: &'a [PhotoGroup],
    /// Genetic algorithm configuration.
    pub ga_config: &'a PageLayoutSolverConfig,
    /// Canvas configuration (page size, margins, bleed, gap).
    pub canvas_config: &'a C,
}

/// The main entry point for running the photobook layout solver.
pub fn run_solver<C: CanvasConfig>(request: &Request<C>) -> Result<Vec<LayoutPage>, SolverError> {
    if request.groups.is_empty() {
        return Ok(vec![]);
    }

    let photos = conversion::photos_from_groups(request.groups);
    let canvas = Canvas::from_canvas_config(request.canvas_config);

    match request.request_type {
        RequestType::SinglePage => run_single_page(&photos, &canvas, request),
        RequestType::MultiPage { config } => run_multi_page(&photos, &canvas, request, config),
    }
}

fn run_single_page<C: CanvasConfig>(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request<C>,
) -> Result<Vec<LayoutPage>, SolverError> {
    let ga_result = page_layout_solver::run_ga(photos, canvas, request.ga_config);
    let layout_page =
        conversion::to_layout_page(&ga_result.layout, 0, photos, request.canvas_config);
    Ok(vec![layout_page])
}

fn run_multi_page<C: CanvasConfig>(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request<C>,
    config: &BookLayoutSolverConfig,
) -> Result<Vec<LayoutPage>, SolverError> {
    let book_layout =
        book_layout_solver::solve_book_layout(photos, config, canvas, request.ga_config)?;

    let mut curr_idx = 0;
    let layout_pages: Vec<LayoutPage> = book_layout
        .pages
        .iter()
        .enumerate()
        .map(|(i, page)| {
            let layout_page = conversion::to_layout_page(
                page,
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
