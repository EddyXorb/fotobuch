//! Precomputed instance data exposing the Bellman model for page assignment.
//!
//! All indices are 0-based. See `docs/design/book_layout_solver_dp/dp.typ` for
//! the formal derivation; the symbol names below mirror that document.

use super::super::model::GroupInfo;
use crate::dto_models::BookLayoutSolverConfig as Params;

/// DP state: `(photos placed, pages used)`.
pub(super) type State = (usize, usize);

/// Precomputed instance data exposing the Bellman model (actions, cost, terminal
/// cost) for the page-assignment problem in O(1) per call.
pub(super) struct PageProblem<'a> {
    params: &'a Params,
    groups: &'a GroupInfo,
    pub(super) n: usize,
    /// `photo_group[i]` = group index of photo `i` (γ in dp.typ).
    photo_group: Vec<usize>,
    /// Photo-index range `[group_start[l], group_end[l])` of group `l`.
    group_start: Vec<usize>,
    group_end: Vec<usize>,
    /// Target photos per page, n̄ = n / page_target.
    n_bar: f64,
}

impl<'a> PageProblem<'a> {
    pub(super) fn new(groups: &'a GroupInfo, params: &'a Params) -> Self {
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
    pub(super) fn actions(&self, i: usize, m: usize) -> Vec<usize> {
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
    pub(super) fn terminal_cost(&self, i: usize, m: usize) -> f64 {
        if i == self.n && (self.params.page_min..=self.params.page_max).contains(&m) {
            self.params.weight_pages * (m as f64 - self.params.page_target as f64).abs()
        } else {
            f64::INFINITY
        }
    }

    /// Even-distribution cost of a page covering `[u, v)`.
    pub(super) fn page_cost(&self, u: usize, v: usize) -> f64 {
        self.params.weight_even * (((v - u) as f64) - self.n_bar).abs()
    }

    /// Split cost charged for the cut at position `c` (κ in dp.typ): `weight_split`
    /// iff the cut lies strictly inside a group, otherwise free.
    pub(super) fn kappa(&self, c: usize) -> f64 {
        if c > 0 && self.photo_group[c - 1] == self.photo_group[c] {
            self.params.weight_split
        } else {
            0.0
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
}
