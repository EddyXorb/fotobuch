//! MIP solver for book layout page assignment.
//!
//! Solves the optimal page assignment problem using Mixed Integer Programming.

mod constraints;
mod objective;
mod var_map;
mod variables;

use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use good_lp::ProblemVariables;
use thiserror::Error;
use tracing::info;
use variables::MipVariables;

/// Error type for MIP solver.
#[derive(Debug, Error)]
pub enum MipError {
    #[error("MIP problem is infeasible")]
    Infeasible,

    #[error("MIP solver timeout")]
    Timeout,

    #[error("MIP solver error: {0}")]
    SolverError(String),
}

/// Solves the page assignment MIP.
///
/// Returns an optimal `PageAssignment` that satisfies all constraints
/// and minimizes the objective function.
///
/// # Arguments
///
/// * `groups` - Information about photo groups
/// * `params` - MIP parameters and constraints
///
/// # Returns
///
/// `Ok(PageAssignment)` with the optimal assignment, or an error if infeasible or solver fails.
pub fn solve_mip(groups: &GroupInfo, params: &Params) -> Result<PageAssignment, MipError> {
    use good_lp::{SolverModel, default_solver};

    // Create problem
    let mut problem = ProblemVariables::new();

    // Extract parameters
    let num_groups = groups.num_groups();
    let group_sizes: Vec<usize> = (0..num_groups).map(|l| groups.group_size(l)).collect();
    let b_max = params.page_max;

    // Create variables
    let vars = MipVariables::new(
        &mut problem,
        num_groups,
        &group_sizes,
        b_max,
        params.group_min_photos,
    );

    // Build objective function
    let objective = objective::build_objective(&vars, groups, params);

    // Build constraints
    let all_constraints = constraints::build_constraints(&vars, groups, params);

    // Build and solve
    let mut model = problem
        .minimise(objective)
        .using(default_solver)
        .set_parallel(good_lp::solvers::highs::HighsParallelType::On);

    info!(
        "Solving MIP with {} variables and {} constraints...",
        vars.len(),
        all_constraints.len()
    );

    for constraint in all_constraints {
        model = model.with(constraint);
    }

    let solution = model
        .solve()
        .map_err(|e| MipError::SolverError(e.to_string()))?;

    // Extract page assignment from solution
    extract_assignment(&solution, &vars, groups, b_max)
}

/// Extracts a `PageAssignment` from the MIP solution.
///
/// Reads the g_lj variables to determine cut points.
fn extract_assignment(
    solution: &impl good_lp::solvers::Solution,
    vars: &MipVariables,
    groups: &GroupInfo,
    b_max: usize,
) -> Result<PageAssignment, MipError> {
    let num_groups = groups.num_groups();
    let total_photos = groups.total_photos();

    // Determine active pages by checking a_j

    let nr_pages = (1..=b_max)
        .rev()
        .find(|p| solution.value(vars.a.get([*p])) > 0.5)
        .unwrap_or(0);

    if nr_pages == 0 {
        return Err(MipError::Infeasible);
    }

    // Compute page sizes from g_lj variables
    let mut cuts = vec![0]; // Always start at 0

    for j in 1..=nr_pages {
        // Page j contains sum_l n_lj photos
        // n_lj = g_lj - g_l(j-1)
        let mut page_size = 0;
        for l in 0..num_groups {
            let g_lj = solution.value(vars.g.get([l, j]));
            let g_lj_prev = if j > 0 {
                solution.value(vars.g.get([l, j - 1]))
            } else {
                0.0
            };
            let n_lj = g_lj - g_lj_prev;
            page_size += n_lj.round() as usize;
        }

        // Add cut point
        let next_cut = *cuts.last().unwrap() + page_size;
        if next_cut > total_photos {
            // Shouldn't happen if MIP is correctly formulated
            return Err(MipError::SolverError(format!(
                "Page {} cut point {} exceeds total photos {}",
                j, next_cut, total_photos
            )));
        }
        cuts.push(next_cut);
    }

    // Final cut should equal total_photos
    if *cuts.last().unwrap() != total_photos {
        return Err(MipError::SolverError(format!(
            "Final cut {} does not equal total photos {}",
            cuts.last().unwrap(),
            total_photos
        )));
    }

    Ok(PageAssignment::new(cuts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn default_params() -> Params {
        Params {
            page_target: 2,
            page_min: 1,
            page_max: 5,
            photos_per_page_min: 3,
            photos_per_page_max: 10,
            group_max_per_page: 3,
            group_min_photos: 3,
            weight_even: 1.0,
            weight_split: 1.0,
            weight_pages: 1.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        }
    }

    #[test]
    fn test_solve_mip_simple() {
        // 2 groups: 5 photos each, target 2 pages
        let groups = GroupInfo::new(&[5, 5]);
        let params = default_params();

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.num_pages(), 2);
        assert_eq!(assignment.total_photos(), 10);
    }

    #[test]
    fn test_solve_mip_three_groups() {
        // 3 groups: 4, 5, 6 photos
        let groups = GroupInfo::new(&[4, 5, 6]);
        let params = Params {
            page_target: 3,
            page_min: 2,
            page_max: 5,
            photos_per_page_min: 4,
            photos_per_page_max: 6,
            group_max_per_page: 2,
            group_min_photos: 3,
            weight_even: 1.0,
            weight_split: 10.0, // Discourage splitting
            weight_pages: 1.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.total_photos(), 15);
        assert!(assignment.num_pages() >= params.page_min);
        assert!(assignment.num_pages() <= params.page_max);
    }

    #[test]
    fn test_solve_mip_respects_g_min() {
        // Large group that can be split
        let groups = GroupInfo::new(&[8, 2]);
        let params = Params {
            page_target: 2,
            page_min: 2,
            page_max: 3,
            photos_per_page_min: 3,
            photos_per_page_max: 6,
            group_max_per_page: 2,
            group_min_photos: 3,
            weight_even: 1.0,
            weight_split: 0.1, // Allow splitting
            weight_pages: 1.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();

        // Check each page size
        for page_idx in 0..assignment.num_pages() {
            let page_size = assignment.page_size(page_idx);
            assert!(
                page_size >= params.photos_per_page_min,
                "Page {} size {} < min {}",
                page_idx,
                page_size,
                params.photos_per_page_min
            );
            assert!(
                page_size <= params.photos_per_page_max,
                "Page {} size {} > max {}",
                page_idx,
                page_size,
                params.photos_per_page_max
            );
        }
    }

    // --- Objective weight isolation tests ---

    /// D_even dominant: 9 photos in 3 equal groups, target 3 pages.
    /// With w1 very high and w2=w3=0, the solver minimises deviation from n̄=3.
    /// The unique optimum is three pages of exactly 3 photos each (D_even=0).
    #[test]
    fn test_weight_even_only_produces_equal_pages() {
        let groups = GroupInfo::new(&[3, 3, 3]);
        let params = Params {
            page_target: 3,
            page_min: 2,
            page_max: 5,
            photos_per_page_min: 1,
            photos_per_page_max: 9,
            group_max_per_page: 3,
            group_min_photos: 1,
            weight_even: 1000.0,
            weight_split: 0.0,
            weight_pages: 0.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.total_photos(), 9);
        assert_eq!(
            assignment.num_pages(),
            3,
            "High D_even should select 3 pages (D_even=0) over 2 pages (D_even=3)"
        );
        for i in 0..3 {
            assert_eq!(
                assignment.page_size(i),
                3,
                "Page {i} should have exactly 3 photos for perfect evenness"
            );
        }
    }

    /// D_even dominant: single group of 9, target 3 pages.
    /// Optimal split is 3×3 (D_even=0). Splitting adds D_split cost, but since
    /// w2=0 the solver ignores it and still picks 3×3 for evenness.
    #[test]
    fn test_weight_even_only_splits_single_group_evenly() {
        let groups = GroupInfo::new(&[9]);
        let params = Params {
            page_target: 3,
            page_min: 2,
            page_max: 5,
            photos_per_page_min: 2,
            photos_per_page_max: 5,
            group_max_per_page: 1,
            group_min_photos: 1,
            weight_even: 1000.0,
            weight_split: 0.0,
            weight_pages: 0.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.total_photos(), 9);
        // All active pages must have exactly 3 photos (n̄=3, D_even=0 is achievable)
        for i in 0..assignment.num_pages() {
            assert_eq!(
                assignment.page_size(i),
                3,
                "Page {i} should have 3 photos for minimum D_even"
            );
        }
    }

    /// D_split dominant: two groups [5, 4], target 2 pages.
    /// With w2 very high, splitting groups is expensive; optimal assigns
    /// each group to its own page (D_split=0), giving pages of sizes 5 and 4.
    #[test]
    fn test_weight_split_only_keeps_groups_together() {
        let groups = GroupInfo::new(&[5, 4]);
        let params = Params {
            page_target: 2,
            page_min: 1,
            page_max: 4,
            photos_per_page_min: 1,
            photos_per_page_max: 9,
            group_max_per_page: 2,
            group_min_photos: 2,
            weight_even: 0.0,
            weight_split: 1000.0,
            weight_pages: 0.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.total_photos(), 9);
        // Each group whole on one page → exactly two pages with sizes 5 and 4
        assert_eq!(
            assignment.num_pages(),
            2,
            "High D_split should keep each group on its own page"
        );
        let sizes: Vec<usize> = (0..assignment.num_pages())
            .map(|i| assignment.page_size(i))
            .collect();
        assert!(
            sizes.contains(&5) && sizes.contains(&4),
            "Expected pages of size 5 and 4, got {sizes:?}"
        );
    }

    /// D_pages dominant: 9 photos, target 2 pages.
    /// With w3 very high the solver minimises |num_pages - 2|, so exactly 2 pages
    /// are expected regardless of their sizes.
    #[test]
    fn test_weight_pages_only_hits_target_page_count() {
        let groups = GroupInfo::new(&[9]);
        let params = Params {
            page_target: 2,
            page_min: 1,
            page_max: 5,
            photos_per_page_min: 1,
            photos_per_page_max: 9,
            group_max_per_page: 1,
            group_min_photos: 1,
            weight_even: 0.0,
            weight_split: 0.0,
            weight_pages: 1000.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        let result = solve_mip(&groups, &params);
        assert!(result.is_ok(), "MIP should be feasible: {:?}", result);

        let assignment = result.unwrap();
        assert_eq!(assignment.total_photos(), 9);
        assert_eq!(
            assignment.num_pages(),
            2,
            "High D_pages should land exactly on target of 2 pages"
        );
    }

    /// Contrasting D_even vs D_split: groups [6, 2], 2 pages, n̄=4.
    ///
    /// * With w1=1000, w2=0: split group 1 as [4|2+2] → pages [4, 4], D_even=0.
    /// * With w2=1000, w1=0: keep groups intact as [6|2]  → D_split=0.
    #[test]
    fn test_weight_even_vs_split_tradeoff() {
        let base = Params {
            page_target: 2,
            page_min: 2,
            page_max: 3,
            photos_per_page_min: 1,
            photos_per_page_max: 8,
            group_max_per_page: 2,
            group_min_photos: 1,
            weight_even: 0.0,
            weight_split: 0.0,
            weight_pages: 0.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        // --- high D_even: prefer equal pages ---
        let even_result = solve_mip(
            &GroupInfo::new(&[6, 2]),
            &Params {
                weight_even: 1000.0,
                weight_split: 0.0,
                weight_pages: 1.0, // small nudge to use 2 pages
                ..base.clone()
            },
        );
        assert!(
            even_result.is_ok(),
            "even-dominant MIP failed: {:?}",
            even_result
        );
        let even_assignment = even_result.unwrap();
        assert_eq!(even_assignment.total_photos(), 8);
        // All pages must be equal (size 4) to achieve D_even=0
        for i in 0..even_assignment.num_pages() {
            assert_eq!(
                even_assignment.page_size(i),
                4,
                "D_even-dominant: page {i} should have 4 photos (equal pages)"
            );
        }

        // --- high D_split: keep groups together, accept uneven pages ---
        let split_result = solve_mip(
            &GroupInfo::new(&[6, 2]),
            &Params {
                weight_even: 0.0,
                weight_split: 1000.0,
                weight_pages: 1.0, // small nudge to use 2 pages
                ..base
            },
        );
        assert!(
            split_result.is_ok(),
            "split-dominant MIP failed: {:?}",
            split_result
        );
        let split_assignment = split_result.unwrap();
        assert_eq!(split_assignment.total_photos(), 8);
        assert_eq!(
            split_assignment.num_pages(),
            2,
            "D_split-dominant: should use 2 pages"
        );
        let sizes: Vec<usize> = (0..split_assignment.num_pages())
            .map(|i| split_assignment.page_size(i))
            .collect();
        assert!(
            sizes.contains(&6) && sizes.contains(&2),
            "D_split-dominant: expected pages [6, 2] to avoid splitting, got {sizes:?}"
        );
    }
}
