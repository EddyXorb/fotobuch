//! Local search for book layout refinement.
//!
//! This module implements a Variable Neighborhood Search (VNS) variant
//! that improves a MIP solution by iteratively adjusting page boundaries.

mod improve;
mod perturbation;

use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig;
use crate::solver::page_layout_solver::CostBreakdown;
use crate::solver::prelude::*;

/// Trait for evaluating single-page layouts for testing purposes.
///
/// Implementations should return a `CostBreakdown` for a given slice of photos.
/// This abstraction allows testing local search logic with mock evaluators.
pub trait PageLayoutEvaluator {
    /// Evaluate the layout quality for a slice of photos.
    ///
    /// Returns cost breakdown (total, size, coverage, barycenter, order).
    fn evaluate(&mut self, photos: &[Photo]) -> CostBreakdown;
}

/// Improves an initial page assignment using local search.
///
/// Uses a Variable Neighborhood Search approach:
/// 1. Identifies candidate cut points (pages with poor coverage)
/// 2. Applies perturbations (shift cut by ±1, ±2, ...) in worst-first order
/// 3. Evaluates feasibility and cost improvement
/// 4. Accepts first improving move, repeats until timeout
///
/// # Arguments
/// * `assignment` - Initial assignment (typically from MIP solver)
/// * `photos` - All photos
/// * `groups` - Group information
/// * `params` - Solver parameters (includes search_timeout)
/// * `evaluator` - Page layout evaluator (for testing; production use evaluate_cached)
///
/// # Returns
/// Improved assignment, worst coverage value, and iteration count.
pub fn improve(
    assignment: PageAssignment,
    photos: &[Photo],
    groups: &GroupInfo,
    params: &BookLayoutSolverConfig,
    evaluator: &mut impl PageLayoutEvaluator,
) -> (PageAssignment, f64, usize) {
    improve::improve(assignment, photos, groups, params, evaluator)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::solver::page_layout_solver::CostBreakdown;

    /// Mock evaluator for testing that returns deterministic costs
    /// based on photo count deviation from an ideal value.
    #[allow(dead_code)]
    struct MockEvaluator {
        ideal_count: usize,
    }

    impl PageLayoutEvaluator for MockEvaluator {
        fn evaluate(&mut self, photos: &[Photo]) -> CostBreakdown {
            let count = photos.len();
            let deviation = (count as i32 - self.ideal_count as i32).abs() as f64;

            // Simple cost: coverage increases with deviation from ideal
            // Other components set to small values for testing
            let coverage = deviation * 0.1;
            CostBreakdown {
                total: coverage + 0.03,
                size: 0.01,
                coverage,
                barycenter: 0.01,
                order: 0.01,
            }
        }
    }

    // Note: evaluate_cached() now takes Canvas + GaConfig and runs real GA,
    // so we don't test it here with mock evaluator. It's tested via integration tests.
}
