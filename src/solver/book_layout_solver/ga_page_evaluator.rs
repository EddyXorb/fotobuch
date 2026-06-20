use super::local_search::PageLayoutEvaluator;
use crate::solver::page_layout_solver::{self, GaResult};
use crate::solver::prelude::*;

pub(super) struct GAPageEvaluator<'a> {
    pub(super) canvas: &'a Canvas,
    pub(super) ga_config: &'a PageLayoutSolverConfig,
}

impl<'a> GAPageEvaluator<'a> {
    pub(super) fn new(canvas: &'a Canvas, ga_config: &'a PageLayoutSolverConfig) -> Self {
        Self { canvas, ga_config }
    }
}

impl PageLayoutEvaluator for GAPageEvaluator<'_> {
    fn evaluate(&self, photos: &[Photo]) -> GaResult {
        page_layout_solver::run_ga(photos, self.canvas, self.ga_config)
    }
}
