//! Local search for book layout refinement.
//!
//! This module implements a Variable Neighborhood Search (VNS) variant
//! that improves a MIP solution by iteratively adjusting page boundaries.

mod improve;
mod perturbation;

use super::cache::LayoutCache;
use super::cost::{AssignmentCost, PageCost};
use super::model::{GroupInfo, PageAssignment, Params};
use crate::models::Photo;
use std::ops::Range;

/// Trait for evaluating single-page layouts without full GA overhead.
///
/// Implementations should return a `PageCost` for a given slice of photos.
/// The book layout solver uses this abstraction to test local search logic
/// with a mock evaluator before wiring up the real GA-based evaluator.
pub trait PageLayoutEvaluator {
    /// Evaluate the layout quality for a slice of photos.
    ///
    /// Returns cost breakdown (coverage, barycenter, order, size).
    fn evaluate(&mut self, photos: &[Photo]) -> PageCost;
}

/// Evaluates a page layout with caching.
///
/// First checks the cache for an existing result. If not found,
/// calls the evaluator and inserts the result into the cache
/// (using monotonic improvement logic: only insert if better than existing).
///
/// # Arguments
/// * `evaluator` - The layout evaluator (mock or GA-based)
/// * `cache` - Layout cache for memoization
/// * `photos` - Full photo array
/// * `range` - Photo range for the page to evaluate
fn evaluate_cached(
    evaluator: &mut impl PageLayoutEvaluator,
    cache: &mut LayoutCache,
    photos: &[Photo],
    range: Range<usize>,
) -> PageCost {
    if let Some(cached) = cache.get(range.clone()) {
        return cached.clone();
    }
    let cost = evaluator.evaluate(&photos[range.clone()]);
    cache.insert_if_better(range, cost.clone());
    cost
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
/// * `evaluator` - Page layout evaluator
///
/// # Returns
/// Improved assignment, its cost, and iteration count.
pub fn improve(
    assignment: PageAssignment,
    photos: &[Photo],
    groups: &GroupInfo,
    params: &Params,
    evaluator: &mut impl PageLayoutEvaluator,
) -> (PageAssignment, AssignmentCost, usize) {
    improve::improve(assignment, photos, groups, params, evaluator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Photo;

    /// Mock evaluator for testing that returns deterministic costs
    /// based on photo count deviation from an ideal value.
    struct MockEvaluator {
        ideal_count: usize,
    }

    impl PageLayoutEvaluator for MockEvaluator {
        fn evaluate(&mut self, photos: &[Photo]) -> PageCost {
            let count = photos.len();
            let deviation = (count as i32 - self.ideal_count as i32).abs() as f64;
            
            // Simple cost: coverage increases with deviation from ideal
            // Other components set to small values for testing
            PageCost {
                coverage: deviation * 0.1,
                barycenter: 0.01,
                order: 0.01,
                size: 0.01,
                total: deviation * 0.1 + 0.03,
            }
        }
    }

    fn create_test_photos(count: usize) -> Vec<Photo> {
        (0..count)
            .map(|i| Photo::new(
                16.0 / 9.0,
                1.0,
                format!("group_{}", i),
            ))
            .collect()
    }

    #[test]
    fn test_evaluate_cached_returns_cached_result() {
        let photos = create_test_photos(10);
        let mut evaluator = MockEvaluator { ideal_count: 5 };
        let mut cache = LayoutCache::new();

        // First call should compute
        let cost1 = evaluate_cached(&mut evaluator, &mut cache, &photos, 0..5);
        assert_eq!(cost1.coverage, 0.0); // 5 photos = ideal

        // Second call should return cached
        let cost2 = evaluate_cached(&mut evaluator, &mut cache, &photos, 0..5);
        assert_eq!(cost1, cost2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_evaluate_cached_computes_different_ranges() {
        let photos = create_test_photos(10);
        let mut evaluator = MockEvaluator { ideal_count: 5 };
        let mut cache = LayoutCache::new();

        let cost1 = evaluate_cached(&mut evaluator, &mut cache, &photos, 0..3);
        let cost2 = evaluate_cached(&mut evaluator, &mut cache, &photos, 3..8);

        // 3 photos: deviation = 2, coverage = 0.2
        approx::assert_abs_diff_eq!(cost1.coverage, 0.2, epsilon = 0.001);
        // 5 photos: deviation = 0, coverage = 0.0
        approx::assert_abs_diff_eq!(cost2.coverage, 0.0, epsilon = 0.001);
        assert_eq!(cache.len(), 2);
    }
}
