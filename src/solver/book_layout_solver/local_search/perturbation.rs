//! Perturbation operations for local search.

use super::super::feasibility::check_feasibility;
use super::super::model::{GroupInfo, PageAssignment, Params};

/// Attempts to apply a perturbation to a page assignment.
///
/// Shifts the cut at `cut_index` by `delta` positions and validates feasibility.
/// Returns `None` if the perturbation violates constraints.
///
/// # Arguments
/// * `assignment` - Current page assignment
/// * `cut_index` - Index of the cut to perturb (0-based, < assignment.num_pages())
/// * `delta` - Shift amount (positive = more photos on page, negative = fewer)
/// * `groups` - Group information
/// * `params` - Solver parameters
///
/// # Returns
/// New assignment if feasible, None otherwise.
pub fn try_perturbation(
    assignment: &PageAssignment,
    cut_index: usize,
    delta: i32,
    groups: &GroupInfo,
    params: &Params,
) -> Option<PageAssignment> {
    let cuts = assignment.cuts();

    // Bounds check
    if cut_index >= cuts.len() {
        return None;
    }

    // Compute new cut value
    let current_cut = cuts[cut_index];
    let new_cut_signed = current_cut as i32 + delta;

    // Must stay within photo bounds
    if new_cut_signed < 0 || new_cut_signed as usize > groups.total_photos() {
        return None;
    }

    let new_cut = new_cut_signed as usize;

    // Ensure monotonicity: cuts must be strictly increasing
    // cuts[cut_index-1] < new_cut < cuts[cut_index+1]
    if cut_index > 0 && new_cut <= cuts[cut_index - 1] {
        return None;
    }
    if cut_index + 1 < cuts.len() && new_cut >= cuts[cut_index + 1] {
        return None;
    }

    // Build new cuts array
    let mut new_cuts = cuts.to_vec();
    new_cuts[cut_index] = new_cut;

    // Create new assignment and validate
    let new_assignment = PageAssignment::new(new_cuts);

    // Check feasibility
    if check_feasibility(&new_assignment, groups, params).is_err() {
        return None;
    }

    Some(new_assignment)
}

/// Computes the maximum reasonable perturbation size.
///
/// Based on the difference between max and min photos per page.
/// No point trying perturbations larger than half that range.
pub fn max_perturbation_delta(params: &Params) -> usize {
    let range = params.photos_per_page_max - params.photos_per_page_min;
    (range / 2).max(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_params() -> Params {
        Params {
            photos_per_page_min: 4,
            photos_per_page_max: 10,
            page_min: 1,
            page_max: 5,
            page_target: 3,
            group_min_photos: 1, // Set to 1 to allow single-photo splits in tests
            group_max_per_page: 5,
            weight_even: 1.0,
            weight_split: 1.0,
            weight_pages: 1.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.5,
        }
    }

    fn create_test_groups() -> GroupInfo {
        GroupInfo::new(&[5, 5, 5]) // 3 groups, 15 photos total
    }

    #[test]
    fn test_try_perturbation_valid_positive_delta() {
        let groups = create_test_groups();
        let params = create_test_params();

        // Assignment: [0, 5, 10, 15] → pages with 5 photos each
        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Shift second cut (index 1) +1: [0, 6, 10, 15]
        let result = try_perturbation(&assignment, 1, 1, &groups, &params);
        assert!(result.is_some());
        assert_eq!(result.unwrap().cuts(), &[0, 6, 10, 15]);
    }

    #[test]
    fn test_try_perturbation_valid_negative_delta() {
        let groups = create_test_groups();
        let params = create_test_params();

        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Shift third cut (index 2) -1: [0, 5, 9, 15]
        // This gives: page 0: 5 photos, page 1: 4 photos, page 2: 6 photos
        // All pages >= photos_per_page_min = 4
        let result = try_perturbation(&assignment, 2, -1, &groups, &params);
        assert!(result.is_some());
        assert_eq!(result.unwrap().cuts(), &[0, 5, 9, 15]);
    }

    #[test]
    fn test_try_perturbation_violates_monotonicity_lower() {
        let groups = create_test_groups();
        let params = create_test_params();

        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Try to shift third cut below second: delta = -6 → 10 - 6 = 4 < 5
        let result = try_perturbation(&assignment, 2, -6, &groups, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_perturbation_violates_monotonicity_upper() {
        let groups = create_test_groups();
        let params = create_test_params();

        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Try to shift second cut above third: delta = +6 → 5 + 6 = 11 > 10
        let result = try_perturbation(&assignment, 1, 6, &groups, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_perturbation_out_of_bounds_negative() {
        let groups = create_test_groups();
        let params = create_test_params();

        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Try to shift second cut to negative: delta = -10 → 5 - 10 = -5 < 0
        // (can't shift first cut since it's always 0)
        let result = try_perturbation(&assignment, 1, -10, &groups, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_perturbation_out_of_bounds_positive() {
        let groups = create_test_groups();
        let params = create_test_params();

        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Try to shift last cut beyond total: delta = +10 → 15 + 10 = 25 > 15
        let result = try_perturbation(&assignment, 3, 10, &groups, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_perturbation_violates_page_size() {
        let groups = create_test_groups();
        let params = create_test_params();

        // Start with valid assignment: [0, 5, 10, 15]
        let assignment = PageAssignment::new(vec![0, 5, 10, 15]);

        // Shift second cut way down: delta = -3 → 5 - 3 = 2 photos on first page
        // This should violate photos_per_page_min = 4
        let result = try_perturbation(&assignment, 1, -3, &groups, &params);
        assert!(result.is_none(), "Should violate page size constraint");
    }

    #[test]
    fn test_max_perturbation_delta() {
        let params = create_test_params();
        let max_delta = max_perturbation_delta(&params);
        // (10 - 4) / 2 = 3
        assert_eq!(max_delta, 3);
    }

    #[test]
    fn test_max_perturbation_delta_minimum() {
        let mut params = create_test_params();
        params.photos_per_page_min = 9;
        params.photos_per_page_max = 10;
        let max_delta = max_perturbation_delta(&params);
        // (10 - 9) / 2 = 0, but min is 2
        assert_eq!(max_delta, 2);
    }
}
