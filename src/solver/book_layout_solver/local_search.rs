//! Local search for book layout refinement.
//!
//! This module implements a Variable Neighborhood Search (VNS) variant
//! that improves a MIP solution by iteratively adjusting page boundaries.

mod improve;
mod perturbation;

use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig;
use crate::solver::page_layout_solver::GaResult;
use crate::solver::prelude::*;

pub use improve::LocalSearchResult;

/// Trait for evaluating single-page layouts.
///
/// Implementations return a full `GaResult` for a given slice of photos.
/// This abstraction allows testing local search logic with mock evaluators.
pub trait PageLayoutEvaluator {
    /// Evaluate the layout quality for a slice of photos.
    ///
    /// Returns the full GA result including layout, fitness, and cost breakdown.
    fn evaluate(&mut self, photos: &[Photo]) -> GaResult;
}

/// Improves an initial page assignment using local search.
///
/// Uses a Variable Neighborhood Search approach:
/// 1. Evaluates all pages and caches their GaResults
/// 2. Identifies cut points adjacent to poorly-covered pages
/// 3. Applies perturbations (shift cut by ±1, ±2, ...) in worst-first order
/// 4. Accepts first improving move, repeats until timeout or convergence
pub fn improve(
    assignment: PageAssignment,
    photos: &[Photo],
    groups: &GroupInfo,
    params: &BookLayoutSolverConfig,
    evaluator: &mut impl PageLayoutEvaluator,
) -> LocalSearchResult {
    improve::improve(assignment, photos, groups, params, evaluator)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::solver::data_models::Canvas;
    use crate::solver::page_layout_solver::{CostBreakdown, GaResult};

    /// Mock evaluator for testing that returns deterministic costs
    /// based on photo count deviation from an ideal value.
    #[allow(dead_code)]
    struct MockEvaluator {
        ideal_count: usize,
    }

    #[allow(dead_code)]
    fn make_mock_result(photos: &[Photo], ideal_count: usize) -> GaResult {
        let count = photos.len();
        let deviation = (count as i32 - ideal_count as i32).abs() as f64;
        let coverage = deviation * 0.1;
        let breakdown = CostBreakdown {
            total: coverage + 0.03,
            size: 0.01,
            coverage,
            barycenter: 0.01,
        };
        GaResult {
            layout: SolverPageLayout::new(vec![], Canvas::new(297.0, 210.0, 5.0)),
            fitness: breakdown.total,
            cost_breakdown: breakdown,
        }
    }

    impl PageLayoutEvaluator for MockEvaluator {
        fn evaluate(&mut self, photos: &[Photo]) -> GaResult {
            make_mock_result(photos, self.ideal_count)
        }
    }
}
