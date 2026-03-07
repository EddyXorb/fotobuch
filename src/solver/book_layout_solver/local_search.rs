//! Local search for book layout refinement.
//!
//! This module implements a Variable Neighborhood Search (VNS) variant
//! that improves a MIP solution by iteratively adjusting page boundaries.

mod improve;
mod perturbation;

use super::cache::LayoutCache;
use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig;
use crate::solver::page_layout_solver::{self, CostBreakdown};
use crate::solver::prelude::*;
use std::ops::Range;

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

/// Evaluates a page layout with caching using the real GA solver.
///
/// First checks the cache for an existing result. If not found,
/// runs the GA solver and inserts the full GaResult into the cache
/// (using monotonic improvement logic: only insert if better than existing).
///
/// # Arguments
/// * `cache` - Layout cache for memoization
/// * `photos` - Full photo array
/// * `range` - Photo range for the page to evaluate
/// * `canvas` - Canvas dimensions
/// * `ga_config` - GA configuration
///
/// # Returns
/// Cost breakdown for the page
pub fn evaluate_cached(
    cache: &mut LayoutCache,
    photos: &[Photo],
    range: Range<usize>,
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> CostBreakdown {
    if let Some(cached) = cache.get(range.clone()) {
        return cached.cost_breakdown.clone();
    }

    let result = page_layout_solver::run_ga(&photos[range.clone()], canvas, ga_config);
    let breakdown = result.cost_breakdown.clone();
    cache.insert_if_better(range, result);
    breakdown
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

    fn create_test_photos(count: usize) -> Vec<Photo> {
        (0..count)
            .map(|i| Photo::new(16.0 / 9.0, 1.0, format!("group_{}", i)))
            .collect()
    }

    // Note: evaluate_cached() now takes Canvas + GaConfig and runs real GA,
    // so we don't test it here with mock evaluator. It's tested via integration tests.
}
