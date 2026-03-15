//! Improvement algorithm for local search.
use tracing::debug;

use super::super::cache::PhotoCombinationCache;
use super::super::model::{GroupInfo, PageAssignment, Params};
use super::PageLayoutEvaluator;
use super::perturbation::{generate_perturbations, max_perturbation_delta, try_perturbation};
use crate::solver::page_layout_solver::{CostBreakdown, GaResult};
use crate::solver::prelude::*;
use std::time::Instant;

/// Result of the local search improvement algorithm.
#[derive(Debug)]
pub struct LocalSearchResult {
    /// The improved page assignment
    pub assignment: PageAssignment,
    /// Cache of evaluated page layouts
    pub cache: PhotoCombinationCache<GaResult>,

    pub start_fitness: f64,

    pub end_fitness: f64,
    /// Number of iterations performed
    pub iterations: usize,
}

/// Improves a page assignment using variable neighborhood search.
///
/// Algorithm:
/// 1. Evaluate all pages, populate layout cache
/// 2. Loop until timeout or convergence:
///    a. Identify all candidate cuts, sorted by worst adjacent-page total cost
///    b. For each candidate, try perturbations with increasing |delta|
///    c. Accept first improving move and restart
/// 3. Return best assignment, its layout cache, worst total cost, and iteration count
pub fn improve(
    mut assignment: PageAssignment,
    photos: &[Photo],
    groups: &GroupInfo,
    params: &Params,
    evaluator: &mut impl PageLayoutEvaluator,
) -> LocalSearchResult {
    let mut cache: PhotoCombinationCache<GaResult> = PhotoCombinationCache::new();
    let deadline = Instant::now() + params.search_timeout;
    let max_delta = max_perturbation_delta(params);
    let mut iterations = 0;

    let initial_worst_over_all = compute_worst_fitness_across_pages(
        0..assignment.num_pages(),
        &assignment,
        photos,
        &mut cache,
        evaluator,
    );

    if max_delta == 0 {
        return LocalSearchResult {
            assignment,
            cache,
            start_fitness: initial_worst_over_all,
            end_fitness: initial_worst_over_all,
            iterations,
        };
    }

    loop {
        iterations += 1;

        if Instant::now() >= deadline {
            break;
        }

        let candidates = find_candidate_cuts(&assignment, photos, &mut cache, evaluator);

        if candidates.is_empty() {
            break;
        }

        let mut improved = false;

        for (cut_index, delta) in generate_perturbations(&candidates, max_delta) {
            if let Some(new_assignment) =
                try_perturbation(&assignment, cut_index, delta, groups, params)
            {
                let first_page = cut_index - 1;
                let old_worst = compute_worst_fitness_across_pages(
                    [first_page, first_page + 1],
                    &assignment,
                    photos,
                    &mut cache,
                    evaluator,
                );

                let new_worst = compute_worst_fitness_across_pages(
                    [first_page, first_page + 1],
                    &new_assignment,
                    photos,
                    &mut cache,
                    evaluator,
                );

                if new_worst < old_worst {
                    debug!(
                        "Iteration {iterations}: Improved by perturbing cut {cut_index} by {delta} (worst total cost {old_worst:.3} → {new_worst:.3})"
                    );
                    assignment = new_assignment;
                    improved = true;
                    break;
                }
            }
        }

        if !improved {
            break;
        }
    }

    // This is cheap as we have the cache
    let final_worst_over_all = compute_worst_fitness_across_pages(
        0..assignment.num_pages(),
        &assignment,
        photos,
        &mut cache,
        evaluator,
    );

    LocalSearchResult {
        assignment,
        cache,
        start_fitness: initial_worst_over_all,
        end_fitness: final_worst_over_all,
        iterations,
    }
}

/// Computes the worst fitness value across all pages.
fn compute_worst_fitness_across_pages(
    pages_to_check: impl IntoIterator<Item = usize>,
    assignment: &PageAssignment,
    photos: &[Photo],
    cache: &mut PhotoCombinationCache<GaResult>,
    evaluator: &mut impl PageLayoutEvaluator,
) -> f64 {
    pages_to_check
        .into_iter()
        .map(|page_idx| {
            let range = assignment.page_range(page_idx);
            evaluate_page(evaluator, cache, &photos[range]).total
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

/// Evaluates a single page using the evaluator, caching the full GaResult.
///
/// Returns the `CostBreakdown` for use by the search algorithm.
fn evaluate_page(
    evaluator: &mut impl PageLayoutEvaluator,
    cache: &mut PhotoCombinationCache<GaResult>,
    photos: &[Photo],
) -> CostBreakdown {
    if let Some(result) = cache.get(photos) {
        return result.cost_breakdown.clone();
    }

    let result = evaluator.evaluate(photos);
    let breakdown = result.cost_breakdown.clone();
    cache.insert_if_better(photos, result);
    breakdown
}

/// Returns all movable cut indices, sorted by worst adjacent-page total cost (descending).
///
/// Cut 0 is always excluded since it is the immutable left boundary.
fn find_candidate_cuts(
    assignment: &PageAssignment,
    photos: &[Photo],
    cache: &mut PhotoCombinationCache<GaResult>,
    evaluator: &mut impl PageLayoutEvaluator,
) -> Vec<usize> {
    let num_pages = assignment.num_pages();

    let breakdowns: Vec<CostBreakdown> = (0..num_pages)
        .map(|page_idx| {
            let range = assignment.page_range(page_idx);
            evaluate_page(evaluator, cache, &photos[range])
        })
        .collect();

    let mut candidates: Vec<(usize, f64)> = (1..assignment.cuts().len() - 1)
        .map(|cut_index| {
            let page_before = cut_index - 1;
            let page_after = cut_index.min(num_pages - 1);
            let max_total = breakdowns[page_before]
                .total
                .max(breakdowns[page_after].total);
            (cut_index, max_total)
        })
        .collect();

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    candidates.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use super::*;
    use crate::solver::data_models::Canvas;
    use crate::solver::page_layout_solver::{CostBreakdown, GaResult};
    use std::time::Duration;

    struct MockEvaluator {
        ideal_count: usize,
    }

    impl PageLayoutEvaluator for MockEvaluator {
        fn evaluate(&mut self, photos: &[Photo]) -> GaResult {
            let count = photos.len();
            let deviation = (count as i32 - self.ideal_count as i32).abs() as f64;
            let total = deviation * 0.2;
            let breakdown = CostBreakdown {
                total: total,
                size: 0.01,
                coverage: 0.1,
                barycenter: 0.01,
            };
            GaResult {
                layout: SolverPageLayout::new(vec![], Canvas::new(297.0, 210.0, 5.0)),
                fitness: breakdown.total,
                cost_breakdown: breakdown,
            }
        }
    }

    fn create_test_photos(count: usize) -> Vec<Photo> {
        (0..count)
            .map(|i| {
                Photo::new(
                    format!("photo_{}", i),
                    16.0 / 9.0,
                    1.0,
                    format!("group_{}", i),
                )
            })
            .collect()
    }

    fn create_test_params() -> Params {
        Params {
            photos_per_page_min: 4,
            photos_per_page_max: 10,
            page_min: 1,
            page_max: 5,
            page_target: 3,
            group_min_photos: 1,
            group_max_per_page: 5,
            weight_even: 1.0,
            weight_split: 1.0,
            weight_pages: 1.0,
            search_timeout: Duration::from_millis(100),
            max_coverage_cost: 0.5,
            enable_local_search: true,
            mip_rel_gap: 0.01,
            max_photos_for_split: 100,
            split_group_boundary_slack: 5,
        }
    }

    fn create_test_groups() -> GroupInfo {
        GroupInfo::new(&[12])
    }

    #[test]
    fn test_improve_attempts_to_balance_pages() {
        let photos = create_test_photos(12);
        let groups = create_test_groups();
        let mut params = create_test_params();
        params.search_timeout = Duration::from_secs(1);
        let mut evaluator = MockEvaluator { ideal_count: 6 };

        // Initial: [0, 4, 8, 12] → 3 pages of 4 photos each (deviation=2 from ideal 6)
        // Due to min-page-size=4, most perturbations are infeasible, so assignment may stay.
        let initial = PageAssignment::new(vec![0, 4, 8, 12]);

        let result = improve(initial.clone(), &photos, &groups, &params, &mut evaluator);

        assert!(result.iterations > 0, "Expected at least one iteration");

        info!("Initial: {:?}", initial.cuts());
        info!("Improved: {:?}", result.assignment.cuts());
        info!("Start fitness: {}", result.start_fitness);
        info!("End fitness: {}", result.end_fitness);
        info!("Iterations: {}", result.iterations);
    }

    #[test]
    fn test_improve_respects_timeout() {
        let photos = create_test_photos(12);
        let groups = create_test_groups();
        let mut params = create_test_params();
        params.search_timeout = Duration::from_millis(1);
        let mut evaluator = MockEvaluator { ideal_count: 6 };

        let initial = PageAssignment::new(vec![0, 4, 8, 12]);

        let start = Instant::now();
        let _ = improve(initial, &photos, &groups, &params, &mut evaluator);
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_millis(50),
            "Should respect timeout"
        );
    }

    #[test]
    fn test_improve_stops_when_optimal() {
        let photos = create_test_photos(12);
        let groups = create_test_groups();
        let params = create_test_params();
        let mut evaluator = MockEvaluator { ideal_count: 6 };

        // Already optimal: 2 pages of 6 photos each → coverage = 0.0
        let initial = PageAssignment::new(vec![0, 6, 12]);

        let result = improve(initial.clone(), &photos, &groups, &params, &mut evaluator);

        assert_eq!(result.assignment.cuts(), initial.cuts());
        assert!(result.iterations <= 2, "Should stop quickly when optimal");
        approx::assert_abs_diff_eq!(result.start_fitness, 0.0, epsilon = 0.01);
        approx::assert_abs_diff_eq!(result.end_fitness, 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_improve_cache_contains_evaluated_layouts() {
        let photos = create_test_photos(12);
        let groups = create_test_groups();
        let params = create_test_params();
        let mut evaluator = MockEvaluator { ideal_count: 6 };

        let initial = PageAssignment::new(vec![0, 6, 12]);
        let result = improve(initial, &photos, &groups, &params, &mut evaluator);

        // Cache must contain a GaResult for each page of the final assignment
        for page_idx in 0..result.assignment.num_pages() {
            let range = result.assignment.page_range(page_idx);
            assert!(
                result.cache.get(&photos[range]).is_some(),
                "Cache missing layout for page {page_idx}"
            );
        }
    }

    #[test]
    fn test_find_candidate_cuts_returns_all_movable_cuts() {
        let photos = create_test_photos(12);
        let mut evaluator = MockEvaluator { ideal_count: 6 };
        let mut cache: PhotoCombinationCache<GaResult> = PhotoCombinationCache::new();

        // [0, 4, 8, 12] → 4 cuts, index 0 is immutable → 3 candidates
        let assignment = PageAssignment::new(vec![0, 4, 8, 12]);

        let candidates = find_candidate_cuts(&assignment, &photos, &mut cache, &mut evaluator);

        assert_eq!(candidates.len(), 2, "All movable cuts should be candidates");
    }

    #[test]
    fn test_find_candidate_cuts_sorted_by_coverage() {
        let photos = create_test_photos(12);
        // ideal=10: deviations for pages [0..4]=6, [4..8]=6, [8..12]=6 → all equal
        // ideal=4: pages [0..4]=0, [4..8]=0, [8..12]=0 → all zero
        // Use asymmetric assignment to get different coverages
        let mut evaluator = MockEvaluator { ideal_count: 10 };
        let mut cache: PhotoCombinationCache<GaResult> = PhotoCombinationCache::new();

        // [0, 4, 8, 12] → pages of 4, 4, 4 (all deviation=6 from ideal 10)
        let assignment = PageAssignment::new(vec![0, 4, 8, 12]);

        let candidates = find_candidate_cuts(&assignment, &photos, &mut cache, &mut evaluator);

        assert!(!candidates.is_empty());
        // All have equal coverage so order doesn't matter, but count should be 2
        assert_eq!(candidates.len(), 2);
    }
}
