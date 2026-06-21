use super::local_search::PageLayoutEvaluator;
use crate::solver::page_layout_solver::{self, PageLayoutResult};
use crate::solver::prelude::*;

pub(super) struct PageEvaluator<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) page_layout_config: &'a PageLayoutSolverConfig,
}

impl<'a> PageEvaluator<'a> {
    pub(super) fn new(canvas: &'a Canvas, page_layout_config: &'a PageLayoutSolverConfig) -> Self {
        Self {
            canvas,
            page_layout_config,
        }
    }
}

impl PageLayoutEvaluator for PageEvaluator<'_> {
    fn evaluate(&self, photos: &[Photo]) -> PageLayoutResult {
        page_layout_solver::solve_page_layout(photos, self.canvas, self.page_layout_config)
    }
}
