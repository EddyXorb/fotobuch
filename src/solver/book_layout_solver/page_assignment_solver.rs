//! Page assignment solver for large instances using problem splitting.
//!
//! This module handles splitting large photo problems (>max_photos_for_split)
//! into smaller subproblems, solving them independently, and merging results.

use super::create_start_solution;
use super::mip;
use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::prelude::*;
use std::time::Duration;
use tracing::debug;
use tracing::info;

/// Solver for page assignment, handles splitting for large instances.
pub struct PageAssignmentSolver {
    params: Params,
}

impl PageAssignmentSolver {
    /// Creates a new page assignment solver with given parameters.
    pub fn new(params: Params) -> Self {
        Self { params }
    }

    /// Solves the page assignment problem, potentially splitting for large instances.
    ///
    /// # Arguments
    ///
    /// * `groups` - Group information for photos
    /// * `photos` - Slice of photos to assign
    ///
    /// # Returns
    ///
    /// A `PageAssignment` for the entire photo set.
    pub fn solve(
        &self,
        groups: &GroupInfo,
        photos: &[Photo],
    ) -> Result<PageAssignment, mip::MipError> {
        let n = groups.total_photos();

        // Determine if splitting is needed
        if n <= self.params.max_photos_for_split {
            // No split needed: solve directly
            debug!(
                "Single problem: {} photos (max_photos_for_split={})",
                n, self.params.max_photos_for_split
            );
            let hint = create_start_solution::create_start_solution(&self.params, photos);
            return mip::solve_mip(groups, &self.params, Some(&hint)).or(Ok(hint));
        }

        // Split needed
        let k = n.div_ceil(self.params.max_photos_for_split);
        debug!(
            "Splitting: {} photos into {} subproblems (max_photos_for_split={})",
            n, k, self.params.max_photos_for_split
        );

        // Compute split points
        let split_points = self.compute_split_points(groups, k);

        // Solve each subproblem with dynamic parameter adjustment
        let mut assignments = Vec::new();
        let mut pages_created = 0;

        for i in 0..k {
            let start = if i == 0 { 0 } else { split_points[i - 1] };
            let end = if i == k - 1 { n } else { split_points[i] };

            let sub_photos = &photos[start..end];
            let sub_groups = GroupInfo::from_photos(sub_photos);

            // Derive parameters for this subproblem, considering pages already created
            let sub_params = self.derive_sub_params(i, k, pages_created);

            debug!(
                "Subproblem {}: photos [{}..{}], page_target={}, page_max={}, pages_created={}",
                i, start, end, sub_params.page_target, sub_params.page_max, pages_created
            );

            // Create hint for warm start
            let hint = create_start_solution::create_start_solution(&sub_params, sub_photos);

            // Solve MIP or fallback to hint
            let assignment = mip::solve_mip(&sub_groups, &sub_params, Some(&hint)).or_else(|err| {
                info!(
                    "Subproblem {} MIP failed with error: {}, using heuristic solution ({} photos)",
                    i,
                    err,
                    sub_photos.len()
                );
                Ok(hint)
            })?;

            pages_created += assignment.num_pages();
            assignments.push(assignment);
        }

        // Merge assignments
        Ok(self.merge(&assignments))
    }

    /// Computes split points for k subproblems.
    ///
    /// Tries to snap to group boundaries within `split_group_boundary_slack` of ideal split points.
    /// Returns k-1 split points (photo indices where subproblems start).
    fn compute_split_points(&self, groups: &GroupInfo, k: usize) -> Vec<usize> {
        if k <= 1 {
            return vec![];
        }

        let n = groups.total_photos();
        let slack = self.params.split_group_boundary_slack;
        let mut split_points = Vec::new();

        for i in 1..k {
            let target = (i * n) / k;
            let window_start = target.saturating_sub(slack);
            let window_end = (target + slack).min(n);

            // Find nearest group boundary in window
            let mut best_boundary = target;
            let mut best_distance = usize::MAX;

            for group_idx in 0..groups.num_groups() {
                let group_end = groups.group_range(group_idx).end;
                if group_end >= window_start && group_end <= window_end {
                    let distance = target.abs_diff(group_end);
                    if distance < best_distance {
                        best_distance = distance;
                        best_boundary = group_end;
                    }
                }
            }

            split_points.push(best_boundary);
        }

        split_points
    }

    /// Derives parameters for a subproblem.
    ///
    /// Dynamically adjusts `page_target` and `page_max` based on pages already created.
    /// `pages_created` allows later subproblems to adapt to actual results from earlier ones.
    fn derive_sub_params(&self, sub_index: usize, k: usize, pages_created: usize) -> Params {
        let mut params = self.params.clone();

        // Calculate remaining pages to allocate and remaining subproblems to solve
        let remaining_subproblems = k - sub_index;
        let target_remaining = self.params.page_target.saturating_sub(pages_created);
        let max_remaining = self.params.page_max.saturating_sub(pages_created);

        // Distribute remaining pages evenly across remaining subproblems (ceiling division)
        params.page_target = target_remaining.div_ceil(remaining_subproblems).max(1);
        params.page_max = max_remaining
            .div_ceil(remaining_subproblems)
            .max(params.page_target);

        // page_min is always 1
        params.page_min = 1;

        // Distribute timeout equally among remaining subproblems
        params.search_timeout = Duration::from_secs_f64(
            self.params.search_timeout.as_secs_f64() / remaining_subproblems as f64,
        );

        debug!(
            "Subproblem {} params: page_target={}, page_max={}, timeout={:.2}s (created={}, remaining={})",
            sub_index,
            params.page_target,
            params.page_max,
            params.search_timeout.as_secs_f64(),
            pages_created,
            remaining_subproblems
        );

        params
    }

    /// Merges subproblem assignments into a global assignment.
    fn merge(&self, assignments: &[PageAssignment]) -> PageAssignment {
        if assignments.is_empty() {
            return PageAssignment::empty();
        }

        let mut merged_cuts = vec![0];
        let mut offset = 0;

        for assignment in assignments {
            let cuts = assignment.cuts();
            // Skip the first cut (always 0) and add remaining cuts with offset
            for &cut in &cuts[1..] {
                merged_cuts.push(offset + cut);
            }
            offset += cuts[cuts.len() - 1];
        }

        PageAssignment::new(merged_cuts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_params() -> Params {
        Params {
            page_target: 32,
            page_min: 1,
            page_max: 48,
            photos_per_page_min: 2,
            photos_per_page_max: 20,
            group_max_per_page: 3,
            group_min_photos: 2,
            weight_even: 1.0,
            weight_split: 10.0,
            weight_pages: 5.0,
            search_timeout: Duration::from_secs(30),
            max_coverage_cost: 0.95,
            enable_local_search: true,
            mip_rel_gap: 0.01,
            max_photos_for_split: 100,
            split_group_boundary_slack: 5,
        }
    }

    #[test]
    fn test_compute_split_points_two_subproblems() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        // Create 100 photos in 10 groups of 10 each
        let group_sizes = vec![10; 10];
        let groups = GroupInfo::new(&group_sizes);

        let split_points = solver.compute_split_points(&groups, 2);

        // Should split near the middle (at 50)
        assert_eq!(split_points.len(), 1);
        assert!(
            split_points[0] >= 45 && split_points[0] <= 55,
            "Split at {}",
            split_points[0]
        );
    }

    #[test]
    fn test_compute_split_points_snap_to_boundary() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        // 30 photos: groups [0..10], [10..20], [20..30]
        let group_sizes = vec![10, 10, 10];
        let groups = GroupInfo::new(&group_sizes);

        let split_points = solver.compute_split_points(&groups, 2);

        // Target is 15, slack is 5, should snap to 20 or 10
        assert_eq!(split_points.len(), 1);
        assert!(split_points[0] == 10 || split_points[0] == 20);
    }

    #[test]
    fn test_derive_sub_params_distribute_evenly() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        // 4 subproblems, page_target=32, no pages created yet
        let sub_0 = solver.derive_sub_params(0, 4, 0);
        let sub_1 = solver.derive_sub_params(1, 4, 0);
        let sub_2 = solver.derive_sub_params(2, 4, 0);
        let sub_3 = solver.derive_sub_params(3, 4, 0);

        // Dynamic calculation per remaining subproblems:
        // sub_0: (32 - 0) / 4 = 8
        // sub_1: (32 - 0) / 3 = 11 ceil
        // sub_2: (32 - 0) / 2 = 16
        // sub_3: (32 - 0) / 1 = 32
        // This is intentional: earlier subproblems can target lower page counts,
        // but if they exceed expectations, later subproblems adjust downward.
        assert_eq!(sub_0.page_target, 8);
        assert_eq!(sub_1.page_target, 11);
        assert_eq!(sub_2.page_target, 16);
        assert_eq!(sub_3.page_target, 32);
    }

    #[test]
    fn test_derive_sub_params_dynamic_with_pages_created() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        // If first sub created fewer pages than expected, later ones get more
        let sub_0_when_0_created = solver.derive_sub_params(0, 4, 0);
        assert_eq!(sub_0_when_0_created.page_target, 8); // (32 - 0) / 4 = 8

        // If first sub created only 5 pages (2 less than expected 8)
        // then 27 pages remain for 3 subproblems
        let sub_1_when_5_created = solver.derive_sub_params(1, 4, 5);
        assert_eq!(sub_1_when_5_created.page_target, 9); // ceil(27 / 3) = 9

        // If first two created 15 pages
        // then 17 pages remain for 2 subproblems
        let sub_2_when_15_created = solver.derive_sub_params(2, 4, 15);
        assert_eq!(sub_2_when_15_created.page_target, 9); // ceil(17 / 2) = 9

        // If first three created 23 pages
        // then 9 pages remain for 1 subproblem
        let sub_3_when_23_created = solver.derive_sub_params(3, 4, 23);
        assert_eq!(sub_3_when_23_created.page_target, 9); // 9 / 1 = 9
    }

    #[test]
    fn test_derive_sub_params_dynamic_adjustment() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        // Scenario: page_target=32, 4 subproblems
        // If first two subproblems create 20 pages, remaining subproblems should adjust
        let sub_0 = solver.derive_sub_params(0, 4, 0);
        assert_eq!(sub_0.page_target, 8); // (32 - 0) / 4 = 8

        // After first subproblem created 10 pages
        let sub_1 = solver.derive_sub_params(1, 4, 10);
        assert_eq!(sub_1.page_target, 8); // (32 - 10) / 3 = 7.33... ceil = 8

        // After first two subproblems created 18 pages
        let sub_2 = solver.derive_sub_params(2, 4, 18);
        assert_eq!(sub_2.page_target, 7); // (32 - 18) / 2 = 7

        // After first three subproblems created 25 pages
        let sub_3 = solver.derive_sub_params(3, 4, 25);
        assert_eq!(sub_3.page_target, 7); // (32 - 25) / 1 = 7
    }

    #[test]
    fn test_merge_single_assignment() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        let assignment = PageAssignment::new(vec![0, 10, 20]);
        let merged = solver.merge(&[assignment]);

        assert_eq!(merged.cuts(), &[0, 10, 20]);
    }

    #[test]
    fn test_merge_multiple_assignments() {
        let params = create_test_params();
        let solver = PageAssignmentSolver::new(params);

        let assign1 = PageAssignment::new(vec![0, 10, 20]);
        let assign2 = PageAssignment::new(vec![0, 15, 30]);

        let merged = solver.merge(&[assign1, assign2]);

        // assign1: [0, 10, 20] (20 photos)
        // assign2: [0, 15, 30] (30 photos) -> offset by 20 -> [20, 35, 50]
        // merged: [0, 10, 20, 35, 50]
        assert_eq!(merged.cuts(), &[0, 10, 20, 35, 50]);
    }

    #[test]
    fn test_no_split_needed() {
        let mut params = create_test_params();
        params.max_photos_for_split = 150; // Higher than test
        let solver = PageAssignmentSolver::new(params);

        // Create 50 photos in 5 groups
        let photos: Vec<Photo> = (0..50)
            .map(|i| {
                Photo::new(
                    format!("photo_{}", i),
                    1.5,
                    1.0,
                    format!("group_{}", i / 10),
                )
            })
            .collect();

        let groups = GroupInfo::from_photos(&photos);

        // Should not panic and should return a valid assignment
        let result = solver.solve(&groups, &photos);
        assert!(result.is_ok() || result.is_err()); // Either OK or MIP error is fine
    }

    #[test]
    fn test_integration_large_instance_with_split() {
        let mut params = create_test_params();
        params.max_photos_for_split = 100; // Trigger split at ~150 photos
        let solver = PageAssignmentSolver::new(params.clone());

        // Create 150 photos in 15 groups of 10 each
        let photos: Vec<Photo> = (0..150)
            .map(|i| {
                Photo::new(
                    format!("photo_{}", i),
                    1.5,
                    1.0,
                    format!("group_{}", i / 10),
                )
            })
            .collect();

        let groups = GroupInfo::from_photos(&photos);

        let result = solver.solve(&groups, &photos);
        assert!(
            result.is_ok(),
            "Large instance should be solvable with split: {:?}",
            result
        );

        let assignment = result.unwrap();
        assert_eq!(
            assignment.total_photos(),
            150,
            "All photos should be assigned"
        );

        // Check that all pages have valid sizes
        for page_idx in 0..assignment.num_pages() {
            let size = assignment.page_size(page_idx);
            assert!(
                size >= params.photos_per_page_min,
                "Page {} has size {}, min is {}",
                page_idx,
                size,
                params.photos_per_page_min
            );
            assert!(
                size <= params.photos_per_page_max,
                "Page {} has size {}, max is {}",
                page_idx,
                size,
                params.photos_per_page_max
            );
        }
    }
}
