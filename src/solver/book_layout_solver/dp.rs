//! Exact dynamic-programming solver for page assignment.
//!
//! Replaces the MIP formulation by the sequence-partitioning DP derived in
//! `docs/design/book_layout_solver_dp/dp.typ`. Same feasible set, same objective,
//! but exact, deterministic and polynomial in time.
//!
//! The problem is expressed in Bellman notation and handed to the generic
//! [`BellmanSolver`]:
//! - **State** `(i, m)` — the first `i` photos placed on `m` pages.
//! - **Decision** `p` — the size of the next page appended at the end.
//! - **Transition** `(i, m) -> (i + p, m + 1)`.
//! - **Cost** `c_page(i, i + p) + κ(i)` — page evenness plus the split penalty of
//!   the cut at position `i`.
//! - **Terminal cost** — `w3·|m - s|` once all photos are placed (`i == n`) on a
//!   valid page count, otherwise infinity.
//!
//! **Orientation.** Both this code and `dp.typ` build pages *forwards*: the
//! recursion starts at the single root `(0, 0)` and appends pages,
//! `(i, m) -> (i + p, m + 1)`, until all photos are placed at a terminal `(n, m)`.
//! The forward orientation is the natural one for the generic solver, which
//! computes `V(x_0)` from one root `x_0`: `(0, 0)` is that root, and the
//! page-count term `w3·|m - s|` falls out exactly as the terminal cost at
//! `(n, m)`.
//!
//! All indices are 0-based.

use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::dp_solver::{BellmanCache, BellmanSolver, StateValue};
use thiserror::Error;

/// Error type for the DP solver.
#[derive(Debug, Error)]
pub enum DpError {
    #[error("page assignment is infeasible")]
    Infeasible,
}

/// DP state: `(photos placed, pages used)`.
type State = (usize, usize);

/// Flat `Vec` cache for the dense `(i, m)` state grid (`i ∈ [0, n]`,
/// `m ∈ [0, page_max]`). The state indexes directly into the vector, so lookups
/// avoid hashing entirely.
struct GridCache {
    stride: usize,
    slots: Vec<Option<StateValue<usize>>>,
}

impl GridCache {
    fn new(n: usize, page_max: usize) -> Self {
        let stride = n + 1;
        Self {
            stride,
            slots: vec![None; stride * (page_max + 1)],
        }
    }

    fn index(&self, (i, m): State) -> usize {
        m * self.stride + i
    }
}

impl BellmanCache<State, usize> for GridCache {
    fn get(&self, state: &State) -> Option<&StateValue<usize>> {
        self.slots[self.index(*state)].as_ref()
    }

    fn insert(&mut self, state: State, value: StateValue<usize>) {
        let idx = self.index(state);
        self.slots[idx] = Some(value);
    }
}

/// Solves the page assignment problem exactly via dynamic programming.
///
/// Returns the optimal [`PageAssignment`] (minimal objective value) or
/// [`DpError::Infeasible`] if no assignment satisfies the constraints.
pub fn solve_dp(groups: &GroupInfo, params: &Params) -> Result<PageAssignment, DpError> {
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
        return Err(DpError::Infeasible);
    }

    // Decisions are page sizes front to back; accumulate them into cut points.
    let mut cuts = Vec::with_capacity(result.decisions.len() + 1);
    cuts.push(0);
    let mut acc = 0;
    for size in &result.decisions {
        acc += size;
        cuts.push(acc);
    }
    Ok(PageAssignment::new(cuts))
}

/// Precomputed instance data exposing the Bellman model (actions, cost, terminal
/// cost) for the page-assignment problem in O(1) per call.
struct PageProblem<'a> {
    params: &'a Params,
    groups: &'a GroupInfo,
    n: usize,
    /// `photo_group[i]` = group index of photo `i` (γ in dp.typ).
    photo_group: Vec<usize>,
    /// Photo-index range `[group_start[l], group_end[l])` of group `l`.
    group_start: Vec<usize>,
    group_end: Vec<usize>,
    /// Target photos per page, n̄ = n / page_target.
    n_bar: f64,
}

impl<'a> PageProblem<'a> {
    fn new(groups: &'a GroupInfo, params: &'a Params) -> Self {
        let n = groups.total_photos();
        let num_groups = groups.num_groups();

        let mut photo_group = vec![0usize; n];
        let mut group_start = vec![0usize; num_groups];
        let mut group_end = vec![0usize; num_groups];
        for l in 0..num_groups {
            let range = groups.group_range(l);
            group_start[l] = range.start;
            group_end[l] = range.end;
            for photo in range {
                photo_group[photo] = l;
            }
        }

        let n_bar = n as f64 / params.page_target as f64;

        Self {
            params,
            groups,
            n,
            photo_group,
            group_start,
            group_end,
            n_bar,
        }
    }

    /// Action set Γ(x): feasible sizes for the page appended at photo index `i`.
    /// Empty for a terminal or dead-end state.
    fn actions(&self, i: usize, m: usize) -> Vec<usize> {
        if i >= self.n || m >= self.params.page_max {
            return Vec::new();
        }
        let p_min = self.params.photos_per_page_min;
        let p_max = self.params.photos_per_page_max.min(self.n - i);
        (p_min..=p_max)
            .filter(|&p| self.interval_feasible(i, i + p))
            .collect()
    }

    /// Terminal cost: page-count deviation once all photos are placed on a valid
    /// page count, otherwise infinity (dead-end).
    fn terminal_cost(&self, i: usize, m: usize) -> f64 {
        if i == self.n && (self.params.page_min..=self.params.page_max).contains(&m) {
            self.params.weight_pages * (m as f64 - self.params.page_target as f64).abs()
        } else {
            f64::INFINITY
        }
    }

    /// Feasibility of a page covering photos `[u, v)` (φ(u, v) in dp.typ).
    ///
    /// The page-size bound `[p_min, p_max]` is already guaranteed by `actions`,
    /// so only the group constraints are checked here.
    fn interval_feasible(&self, u: usize, v: usize) -> bool {
        let first = self.photo_group[u];
        let last = self.photo_group[v - 1];

        // Max distinct groups per page (groups are contiguous).
        if last - first + 1 > self.params.group_max_per_page {
            return false;
        }

        // Splitting rule: only the boundary groups can be partial.
        if !self.split_ok(u, v, first) {
            return false;
        }
        if last != first && !self.split_ok(u, v, last) {
            return false;
        }
        true
    }

    /// Checks the splitting rule for group `l` on page `[u, v)`.
    fn split_ok(&self, u: usize, v: usize, l: usize) -> bool {
        let fragment = v.min(self.group_end[l]) - u.max(self.group_start[l]);
        let size = self.groups.group_size(l);
        if fragment < size {
            // Group is split here: must be splittable and the fragment big enough.
            size >= self.params.group_min_photos && fragment >= self.params.group_min_photos
        } else {
            true
        }
    }

    /// Even-distribution cost of a page covering `[u, v)`.
    fn page_cost(&self, u: usize, v: usize) -> f64 {
        self.params.weight_even * (((v - u) as f64) - self.n_bar).abs()
    }

    /// Split cost charged for the cut at position `c` (κ in dp.typ): `weight_split`
    /// iff the cut lies strictly inside a group, otherwise free.
    fn kappa(&self, c: usize) -> f64 {
        if c > 0 && self.photo_group[c - 1] == self.photo_group[c] {
            self.params.weight_split
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

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

    /// Independent reference implementation of the objective Z from dp.typ.
    /// Returns `None` if the cut vector is infeasible.
    fn reference_cost(cuts: &[usize], groups: &GroupInfo, p: &Params) -> Option<f64> {
        let n = groups.total_photos();
        let m = cuts.len() - 1;
        if m < p.page_min || m > p.page_max {
            return None;
        }
        let n_bar = n as f64 / p.page_target as f64;
        let pg = |i: usize| groups.group_of_photo(i);

        let mut total = 0.0;
        for j in 0..m {
            let (a, b) = (cuts[j], cuts[j + 1]);
            let size = b - a;
            if size < p.photos_per_page_min || size > p.photos_per_page_max {
                return None;
            }
            if pg(b - 1) - pg(a) + 1 > p.group_max_per_page {
                return None;
            }
            for l in [pg(a), pg(b - 1)] {
                let range = groups.group_range(l);
                let fragment = b.min(range.end) - a.max(range.start);
                let gs = groups.group_size(l);
                if fragment < gs && !(gs >= p.group_min_photos && fragment >= p.group_min_photos) {
                    return None;
                }
            }
            total += p.weight_even * (size as f64 - n_bar).abs();
        }
        for &c in &cuts[1..m] {
            if pg(c - 1) == pg(c) {
                total += p.weight_split;
            }
        }
        total += p.weight_pages * (m as f64 - p.page_target as f64).abs();
        Some(total)
    }

    /// Brute-force optimum over all cut vectors (only for small n).
    fn brute_force_min(groups: &GroupInfo, p: &Params) -> Option<f64> {
        let n = groups.total_photos();
        assert!(n <= 16, "brute force only for small instances");
        let mut best: Option<f64> = None;
        for mask in 0u32..(1u32 << (n - 1)) {
            let mut cuts = vec![0];
            for pos in 1..n {
                if mask & (1 << (pos - 1)) != 0 {
                    cuts.push(pos);
                }
            }
            cuts.push(n);
            if let Some(cost) = reference_cost(&cuts, groups, p) {
                best = Some(best.map_or(cost, |b| b.min(cost)));
            }
        }
        best
    }

    // --- Ported simple instances (from the former MIP tests) ---

    #[test]
    fn test_solve_simple_two_groups() {
        let groups = GroupInfo::new(&[5, 5]);
        let assignment = solve_dp(&groups, &default_params()).unwrap();
        assert_eq!(assignment.num_pages(), 2);
        assert_eq!(assignment.total_photos(), 10);
    }

    #[test]
    fn test_solve_three_groups() {
        let groups = GroupInfo::new(&[4, 5, 6]);
        let params = Params {
            page_target: 3,
            page_min: 2,
            page_max: 5,
            photos_per_page_min: 4,
            photos_per_page_max: 6,
            group_max_per_page: 2,
            group_min_photos: 3,
            weight_split: 10.0,
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        assert_eq!(assignment.total_photos(), 15);
        assert!(assignment.num_pages() >= params.page_min);
        assert!(assignment.num_pages() <= params.page_max);
    }

    #[test]
    fn test_solve_respects_page_sizes() {
        let groups = GroupInfo::new(&[8, 2]);
        let params = Params {
            page_target: 2,
            page_min: 2,
            page_max: 3,
            photos_per_page_min: 3,
            photos_per_page_max: 6,
            group_max_per_page: 2,
            group_min_photos: 3,
            weight_split: 0.1,
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        for page in 0..assignment.num_pages() {
            let size = assignment.page_size(page);
            assert!(size >= params.photos_per_page_min);
            assert!(size <= params.photos_per_page_max);
        }
    }

    // --- Weight isolation tests ---

    /// w1 dominant, three equal groups: unique optimum is 3 pages of 3 (D_even=0).
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
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        assert_eq!(assignment.num_pages(), 3);
        for i in 0..3 {
            assert_eq!(assignment.page_size(i), 3);
        }
    }

    /// w1 dominant, single group of 9: optimum splits evenly into 3×3.
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
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        assert_eq!(assignment.total_photos(), 9);
        for i in 0..assignment.num_pages() {
            assert_eq!(assignment.page_size(i), 3);
        }
    }

    /// w2 dominant: no group is ever split (every internal cut is a group boundary).
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
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        let cuts = assignment.cuts();
        for &c in &cuts[1..cuts.len() - 1] {
            assert_ne!(
                groups.group_of_photo(c - 1),
                groups.group_of_photo(c),
                "internal cut at {c} splits a group despite huge weight_split"
            );
        }
    }

    /// w3 dominant: page count lands exactly on the target.
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
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        assert_eq!(assignment.num_pages(), 2);
    }

    /// Even-vs-split tradeoff on groups [6, 2], n̄=4, with a small page nudge.
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
            ..default_params()
        };

        // High w1: split group 1 into [4 | 2+2] → pages [4, 4].
        let even = solve_dp(
            &GroupInfo::new(&[6, 2]),
            &Params {
                weight_even: 1000.0,
                weight_pages: 1.0,
                ..base.clone()
            },
        )
        .unwrap();
        for i in 0..even.num_pages() {
            assert_eq!(even.page_size(i), 4);
        }

        // High w2: keep groups intact → pages [6, 2].
        let split = solve_dp(
            &GroupInfo::new(&[6, 2]),
            &Params {
                weight_split: 1000.0,
                weight_pages: 1.0,
                ..base
            },
        )
        .unwrap();
        assert_eq!(split.num_pages(), 2);
        let sizes: Vec<usize> = (0..split.num_pages()).map(|i| split.page_size(i)).collect();
        assert!(sizes.contains(&6) && sizes.contains(&2), "got {sizes:?}");
    }

    // --- Exactness, infeasibility, determinism, performance ---

    #[test]
    fn test_exactness_against_brute_force() {
        let cases: Vec<(GroupInfo, Params)> = vec![
            (GroupInfo::new(&[9]), default_params()),
            (GroupInfo::new(&[4, 5]), default_params()),
            (
                GroupInfo::new(&[3, 3, 3]),
                Params {
                    page_target: 3,
                    page_min: 1,
                    page_max: 5,
                    photos_per_page_min: 1,
                    photos_per_page_max: 6,
                    group_max_per_page: 2,
                    group_min_photos: 1,
                    weight_even: 1.0,
                    weight_split: 7.0,
                    weight_pages: 2.0,
                    ..default_params()
                },
            ),
            (
                GroupInfo::new(&[6, 2, 4]),
                Params {
                    page_target: 3,
                    page_min: 2,
                    page_max: 6,
                    photos_per_page_min: 1,
                    photos_per_page_max: 5,
                    group_max_per_page: 2,
                    group_min_photos: 2,
                    weight_even: 3.0,
                    weight_split: 1.0,
                    weight_pages: 4.0,
                    ..default_params()
                },
            ),
        ];

        for (groups, params) in &cases {
            let assignment = solve_dp(groups, params).expect("instance should be feasible");
            let dp_cost =
                reference_cost(assignment.cuts(), groups, params).expect("dp solution feasible");
            let bf = brute_force_min(groups, params).expect("brute force found an optimum");
            assert!(
                (dp_cost - bf).abs() < 1e-9,
                "dp_cost={dp_cost}, brute_force={bf}"
            );
        }
    }

    #[test]
    fn test_infeasible_unsplittable_oversized_group() {
        // Group of 6 cannot fit (p_max=4) nor split (fragment must be >= g_min=6).
        let groups = GroupInfo::new(&[6]);
        let params = Params {
            page_target: 2,
            page_min: 1,
            page_max: 3,
            photos_per_page_min: 1,
            photos_per_page_max: 4,
            group_max_per_page: 1,
            group_min_photos: 6,
            ..default_params()
        };

        assert!(matches!(
            solve_dp(&groups, &params),
            Err(DpError::Infeasible)
        ));
    }

    #[test]
    fn test_infeasible_empty_instance() {
        let groups = GroupInfo::new(&[]);
        assert!(matches!(
            solve_dp(&groups, &default_params()),
            Err(DpError::Infeasible)
        ));
    }

    #[test]
    fn test_determinism() {
        let groups = GroupInfo::new(&[6, 2, 4]);
        let params = Params {
            page_target: 3,
            page_min: 2,
            page_max: 6,
            photos_per_page_min: 1,
            photos_per_page_max: 5,
            group_max_per_page: 2,
            group_min_photos: 1,
            weight_even: 2.0,
            weight_split: 3.0,
            weight_pages: 1.0,
            ..default_params()
        };

        let a = solve_dp(&groups, &params).unwrap();
        let b = solve_dp(&groups, &params).unwrap();
        assert_eq!(a.cuts(), b.cuts());
    }

    /// Cut exactly on a group boundary must not incur weight_split.
    #[test]
    fn test_boundary_cut_is_free() {
        let groups = GroupInfo::new(&[4, 4]);
        let params = Params {
            page_target: 2,
            page_min: 1,
            page_max: 4,
            photos_per_page_min: 1,
            photos_per_page_max: 8,
            group_max_per_page: 2,
            group_min_photos: 1,
            weight_even: 1.0,
            weight_split: 1000.0,
            weight_pages: 0.0,
            ..default_params()
        };

        let assignment = solve_dp(&groups, &params).unwrap();
        // Optimum splits at the boundary (index 4): even pages, zero split cost.
        assert_eq!(assignment.cuts(), &[0, 4, 8]);
        let cost = reference_cost(assignment.cuts(), &groups, &params).unwrap();
        assert!(cost < params.weight_split, "boundary cut wrongly penalised");
    }

    #[test]
    fn test_performance_large_instance() {
        // 1000 photos in 30 groups, up to 100 pages: must solve well under 1 s.
        // The flat GridCache (no hashing) keeps even the debug build comfortably
        // below the bound; optimized builds solve it in ~15 ms.
        let group_sizes: Vec<usize> = (0..30).map(|i| if i < 10 { 34 } else { 33 }).collect();
        let groups = GroupInfo::new(&group_sizes);
        assert_eq!(groups.total_photos(), 1000);

        let params = Params {
            page_target: 67,
            page_min: 50,
            page_max: 100,
            photos_per_page_min: 1,
            photos_per_page_max: 20,
            group_max_per_page: 5,
            group_min_photos: 1,
            weight_even: 1.0,
            weight_split: 10.0,
            weight_pages: 5.0,
            ..default_params()
        };

        let start = Instant::now();
        let assignment = solve_dp(&groups, &params).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(assignment.total_photos(), 1000);
        assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    }
}
