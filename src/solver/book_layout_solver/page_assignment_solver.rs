//! Page assignment: exact DP with a heuristic fallback.
//!
//! The dynamic program solves any instance size directly, so no problem
//! splitting is needed. If the instance is infeasible, a greedy start solution
//! is used instead of failing.

use super::create_start_solution;
use super::dp;
use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::prelude::*;
use tracing::info;

/// Solver for page assignment.
pub struct PageAssignmentSolver {
    params: Params,
}

impl PageAssignmentSolver {
    /// Creates a new page assignment solver with given parameters.
    pub fn new(params: Params) -> Self {
        Self { params }
    }

    /// Solves the page assignment exactly via the DP.
    ///
    /// Falls back to a greedy start solution if the DP reports the instance as
    /// infeasible, preserving the previous "never hard-fail" behaviour.
    pub fn solve(
        &self,
        groups: &GroupInfo,
        photos: &[Photo],
    ) -> Result<PageAssignment, dp::DpError> {
        dp::solve_dp(groups, &self.params).or_else(|err| {
            info!("DP infeasible ({err}), using heuristic start solution");
            Ok(create_start_solution::create_start_solution(
                &self.params,
                photos,
            ))
        })
    }
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
            enable_local_search: true,
            mip_rel_gap: None,
            max_photos_for_split: None,
            split_group_boundary_slack: None,
        }
    }

    #[test]
    fn test_solve_returns_optimal_assignment() {
        let photos: Vec<Photo> = (0..10)
            .map(|i| Photo::new(format!("p{i}"), 1.5, 1.0, "g".to_string()))
            .collect();
        let groups = GroupInfo::from_photos(&photos);

        let assignment = PageAssignmentSolver::new(default_params())
            .solve(&groups, &photos)
            .unwrap();

        assert_eq!(assignment.total_photos(), 10);
    }

    #[test]
    fn test_solve_falls_back_to_heuristic_when_infeasible() {
        // Group of 6 cannot fit (p_max=4) nor split (fragment must be >= g_min=6):
        // the DP is infeasible, so the heuristic start solution is returned.
        let photos: Vec<Photo> = (0..6)
            .map(|i| Photo::new(format!("p{i}"), 1.5, 1.0, "g".to_string()))
            .collect();
        let groups = GroupInfo::from_photos(&photos);
        let params = Params {
            page_min: 1,
            page_max: 3,
            photos_per_page_min: 1,
            photos_per_page_max: 4,
            group_max_per_page: 1,
            group_min_photos: 6,
            ..default_params()
        };

        let assignment = PageAssignmentSolver::new(params)
            .solve(&groups, &photos)
            .unwrap();

        assert_eq!(assignment.total_photos(), 6);
    }
}
