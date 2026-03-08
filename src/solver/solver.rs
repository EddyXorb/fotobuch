//! High-level solver orchestration.
//!
//! This module provides the main entry point for running the photobook solver,
//! coordinating input loading, solver configuration, optimization, and export.

use crate::dto_models::{BookConfig, BookLayoutSolverConfig, LayoutPage, PhotoFile, PhotoGroup};
use crate::solver::prelude::*;
use anyhow::Result;

pub enum RequestType<'a> {
    /// Single-page layout optimization; no grouping or multi-page logic applied.
    /// uses page_layout_solver directly on the full photo set.
    SinglePageOptimization(&'a [PhotoFile]),
    /// Multi-page book layout optimization with grouping and page assignment.
    /// uses the full MIP + local search pipeline to distribute photos across pages
    /// creates layouts for each page.
    MultiPageOptimization(&'a [PhotoGroup]),
}

pub struct Request<'a> {
    pub request_type: RequestType<'a>,
    pub config: &'a BookLayoutSolverConfig,
    pub ga_config: &'a GaConfig,
    pub book_config: &'a BookConfig,
}
/// The main entry point for running the photobook layout solver.
pub fn run_solver(_request: &Request) -> Result<Vec<LayoutPage>> {
    Ok(vec![])
}
