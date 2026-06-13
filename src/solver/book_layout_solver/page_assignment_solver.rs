//! Page assignment: exact dynamic program with a heuristic fallback.
//!
//! This is *the* page-assignment solver. Internally it expresses the problem in
//! Bellman notation (the sequence-partitioning DP derived in
//! `docs/design/book_layout_solver_dp/dp.typ`) and hands it to the generic
//! [`BellmanSolver`]. Same feasible set, same objective as the former MIP, but
//! exact, deterministic and polynomial in time:
//! - **State** `(i, m)` — the first `i` photos placed on `m` pages.
//! - **Decision** `p` — the size of the next page appended at the end.
//! - **Transition** `(i, m) -> (i + p, m + 1)`.
//! - **Cost** `c_page(i, i + p) + κ(i)` — page evenness plus the split penalty of
//!   the cut at position `i`.
//! - **Terminal cost** — `w3·|m - s|` once all photos are placed (`i == n`) on a
//!   valid page count, otherwise infinity.
//!
//! **Orientation.** Pages are built *forwards*: the recursion starts at the
//! single root `(0, 0)` and appends pages, `(i, m) -> (i + p, m + 1)`, until all
//! photos are placed at a terminal `(n, m)`. The page-count term `w3·|m - s|`
//! falls out exactly as the terminal cost at `(n, m)`.
//!
//! The DP solves any instance size directly, so no problem splitting is needed.
//! If the instance is infeasible, a greedy start solution is used instead of
//! failing. All indices are 0-based. The precomputed model lives in [`problem`],
//! the dense memoization cache in [`cache`].

mod cache;
mod problem;

#[cfg(test)]
mod exact_tests;

use super::create_start_solution;
use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::dp_solver::BellmanSolver;
use crate::solver::prelude::*;
use cache::GridCache;
use problem::{PageProblem, State};
use thiserror::Error;
use tracing::{debug, info};

/// Error type for page assignment.
#[derive(Debug, Error)]
pub enum PageAssignmentError {
    #[error("page assignment is infeasible")]
    Infeasible,
}

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
    /// infeasible, preserving the "never hard-fail" behaviour.
    pub fn solve(
        &self,
        groups: &GroupInfo,
        photos: &[Photo],
    ) -> Result<PageAssignment, PageAssignmentError> {
        solve_exact(groups, &self.params).or_else(|err| {
            info!("exact page assignment infeasible ({err}), using heuristic start solution");
            Ok(create_start_solution::create_start_solution(
                &self.params,
                photos,
            ))
        })
    }
}

/// Solves the page assignment problem exactly via dynamic programming.
///
/// Returns the optimal [`PageAssignment`] (minimal objective value) or
/// [`PageAssignmentError::Infeasible`] if no assignment satisfies the constraints.
fn solve_exact(groups: &GroupInfo, params: &Params) -> Result<PageAssignment, PageAssignmentError> {
    let ctx = PageProblem::new(groups, params);
    let cache = GridCache::new(ctx.n, params.page_max);

    // Bellman model components (named for readability), see the module docs.
    let initial_state: State = (0, 0);
    let transition = |x: &State, page_size: &usize| (x.0 + page_size, x.1 + 1);
    let cost = |x: &State, page_size: &usize| ctx.page_cost(x.0, x.0 + page_size) + ctx.kappa(x.0);
    let actions = |x: &State| ctx.actions(x.0, x.1);
    let terminal_cost = |x: &State| ctx.terminal_cost(x.0, x.1);

    let mut solver = BellmanSolver::with_cache(
        initial_state,
        transition,
        cost,
        actions,
        terminal_cost,
        cache,
    );

    let result = solver.solve();
    if !result.objective.is_finite() {
        return Err(PageAssignmentError::Infeasible);
    }

    // Decisions are page sizes front to back; accumulate them into cut points.
    let mut cuts = Vec::with_capacity(result.decisions.len() + 1);
    cuts.push(0);
    let mut acc = 0;
    for size in &result.decisions {
        acc += size;
        cuts.push(acc);
    }

    info!(
        "DP page assignment: cost {:.3}, {} pages",
        result.objective,
        result.decisions.len()
    );
    debug!("DP page cuts: {:?}", cuts);

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
