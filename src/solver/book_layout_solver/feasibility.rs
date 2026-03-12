//! Feasibility checking for page assignments.
//!
//! Validates that a `PageAssignment` satisfies all hard constraints.

use super::model::{GroupInfo, PageAssignment, Params};
use thiserror::Error;

/// Constraint violation error.
#[derive(Debug, Error, PartialEq)]
pub enum ConstraintViolation {
    #[error("page count {page_count} is outside [{page_min}, {page_max}]")]
    PageCountOutOfRange {
        page_count: usize,
        page_min: usize,
        page_max: usize,
    },

    #[error(
        "page {page_index} has {page_size} photos, outside [{photos_per_page_min}, {photos_per_page_max}]"
    )]
    PageSizeOutOfRange {
        page_index: usize,
        page_size: usize,
        photos_per_page_min: usize,
        photos_per_page_max: usize,
    },

    #[error("page {page_index} has {group_count} groups, exceeds maximum {group_max_per_page}")]
    TooManyGroupsOnPage {
        page_index: usize,
        group_count: usize,
        group_max_per_page: usize,
    },

    #[error(
        "page {page_index} has {photos_in_group} photos from group {group_index} (size {group_size}), violates g_min rule (min {group_min_photos})"
    )]
    GroupMinViolation {
        page_index: usize,
        group_index: usize,
        group_size: usize,
        photos_in_group: usize,
        group_min_photos: usize,
    },
}

/// Checks if a page assignment satisfies all hard constraints.
///
/// Returns `Ok(())` if feasible, or an error describing the first violation.
///
/// # Constraints checked
///
/// 1. Page count in [page_min, page_max]
/// 2. Each page size in [photos_per_page_min, photos_per_page_max]
/// 3. Max groups per page ≤ group_max_per_page
/// 4. g_min rule: if a group is split and group_size >= g_min, then each portion >= g_min
/// 5. Sequential ordering (implicit in PageAssignment representation)
pub fn check_feasibility(
    assignment: &PageAssignment,
    groups: &GroupInfo,
    params: &Params,
) -> Result<(), ConstraintViolation> {
    // 1. Check page count
    let page_count = assignment.num_pages();
    if page_count < params.page_min || page_count > params.page_max {
        return Err(ConstraintViolation::PageCountOutOfRange {
            page_count,
            page_min: params.page_min,
            page_max: params.page_max,
        });
    }

    // 2. Check each page size and group constraints
    for page_idx in 0..page_count {
        let page_size = assignment.page_size(page_idx);

        // 2a. Page size bounds
        if page_size < params.photos_per_page_min || page_size > params.photos_per_page_max {
            return Err(ConstraintViolation::PageSizeOutOfRange {
                page_index: page_idx,
                page_size,
                photos_per_page_min: params.photos_per_page_min,
                photos_per_page_max: params.photos_per_page_max,
            });
        }

        // Compute groups on this page
        let page_range = assignment.page_range(page_idx);
        let groups_on_page = groups_on_page_with_counts(page_range.clone(), groups);

        // 2b. Max groups per page
        if groups_on_page.len() > params.group_max_per_page {
            return Err(ConstraintViolation::TooManyGroupsOnPage {
                page_index: page_idx,
                group_count: groups_on_page.len(),
                group_max_per_page: params.group_max_per_page,
            });
        }

        // 2c. g_min rule
        for (group_idx, photos_in_group) in groups_on_page {
            let group_size = groups.group_size(group_idx);

            // Group is split if it doesn't contain all photos
            let is_split = photos_in_group < group_size;

            // If group is large enough and split, each portion must satisfy g_min
            if is_split
                && group_size >= params.group_min_photos
                && photos_in_group < params.group_min_photos
            {
                return Err(ConstraintViolation::GroupMinViolation {
                    page_index: page_idx,
                    group_index: group_idx,
                    group_size,
                    photos_in_group,
                    group_min_photos: params.group_min_photos,
                });
            }
        }
    }

    Ok(())
}

/// Returns groups present on a page with their photo counts.
///
/// Returns a vector of (group_index, photo_count) pairs.
fn groups_on_page_with_counts(
    page_range: std::ops::Range<usize>,
    groups: &GroupInfo,
) -> Vec<(usize, usize)> {
    if page_range.is_empty() {
        return vec![];
    }

    let first_photo = page_range.start;
    let last_photo = page_range.end - 1;

    let first_group = groups.group_of_photo(first_photo);
    let last_group = groups.group_of_photo(last_photo);

    let mut result = Vec::new();

    for group_idx in first_group..=last_group {
        let group_range = groups.group_range(group_idx);

        // Count overlap between page_range and group_range
        let overlap_start = page_range.start.max(group_range.start);
        let overlap_end = page_range.end.min(group_range.end);
        let count = overlap_end.saturating_sub(overlap_start);

        if count > 0 {
            result.push((group_idx, count));
        }
    }

    result
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
    fn test_feasibility_valid_assignment() {
        // Groups: [5, 4, 6]
        // Photos: 0-4 (group 0), 5-8 (group 1), 9-14 (group 2)
        // Pages: [0-4], [5-8], [9-14]
        // Each page contains exactly one complete group
        let groups = GroupInfo::new(&[5, 4, 6]);
        let assignment = PageAssignment::new(vec![0, 5, 9, 15]);
        let params = default_params();

        let result = check_feasibility(&assignment, &groups, &params);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    }

    #[test]
    fn test_feasibility_page_count_too_small() {
        let groups = GroupInfo::new(&[5, 5]);
        let assignment = PageAssignment::new(vec![0, 10]); // 1 page
        let params = Params {
            page_min: 2,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::PageCountOutOfRange {
                page_count: 1,
                page_min: 2,
                page_max: 5
            })
        );
    }

    #[test]
    fn test_feasibility_page_count_too_large() {
        let groups = GroupInfo::new(&[3, 3, 3, 3, 3, 3]);
        let assignment = PageAssignment::new(vec![0, 3, 6, 9, 12, 15, 18]); // 6 pages
        let params = Params {
            page_max: 5,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::PageCountOutOfRange {
                page_count: 6,
                page_min: 1,
                page_max: 5
            })
        );
    }

    #[test]
    fn test_feasibility_page_size_too_small() {
        let groups = GroupInfo::new(&[5, 5]);
        let assignment = PageAssignment::new(vec![0, 2, 10]); // page 0 has 2 photos
        let params = Params {
            photos_per_page_min: 3,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::PageSizeOutOfRange {
                page_index: 0,
                page_size: 2,
                photos_per_page_min: 3,
                photos_per_page_max: 10
            })
        );
    }

    #[test]
    fn test_feasibility_page_size_too_large() {
        let groups = GroupInfo::new(&[15]);
        let assignment = PageAssignment::new(vec![0, 15]); // page 0 has 15 photos
        let params = Params {
            photos_per_page_max: 10,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::PageSizeOutOfRange {
                page_index: 0,
                page_size: 15,
                photos_per_page_min: 3,
                photos_per_page_max: 10
            })
        );
    }

    #[test]
    fn test_feasibility_too_many_groups() {
        let groups = GroupInfo::new(&[2, 2, 2, 2]);
        let assignment = PageAssignment::new(vec![0, 8]); // 4 groups on one page
        let params = Params {
            group_max_per_page: 3,
            photos_per_page_max: 10,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::TooManyGroupsOnPage {
                page_index: 0,
                group_count: 4,
                group_max_per_page: 3
            })
        );
    }

    #[test]
    fn test_feasibility_group_min_violation() {
        let groups = GroupInfo::new(&[5, 5]);
        // Split group 0: 2 photos on page 0, 3 photos on page 1
        // Group size (5) >= g_min (3), but portion (2) < g_min
        let assignment = PageAssignment::new(vec![0, 2, 10]);
        let params = Params {
            photos_per_page_min: 2,
            group_min_photos: 3,
            ..default_params()
        };

        assert_eq!(
            check_feasibility(&assignment, &groups, &params),
            Err(ConstraintViolation::GroupMinViolation {
                page_index: 0,
                group_index: 0,
                group_size: 5,
                photos_in_group: 2,
                group_min_photos: 3
            })
        );
    }

    #[test]
    fn test_feasibility_small_group_split_allowed() {
        let groups = GroupInfo::new(&[2, 2]);
        // Split small groups (size < g_min) is allowed
        let assignment = PageAssignment::new(vec![0, 1, 4]);
        let params = Params {
            photos_per_page_min: 1,
            group_min_photos: 3,
            ..default_params()
        };

        // Should be OK because group size (2) < g_min (3), so g_min doesn't apply
        assert!(check_feasibility(&assignment, &groups, &params).is_ok());
    }

    #[test]
    fn test_groups_on_page_with_counts_single_group() {
        let groups = GroupInfo::new(&[10]);
        let counts = groups_on_page_with_counts(2..7, &groups);
        assert_eq!(counts, vec![(0, 5)]);
    }

    #[test]
    fn test_groups_on_page_with_counts_multiple_groups() {
        let groups = GroupInfo::new(&[3, 4, 5]);
        // Page covers photos 2..9 (group 0: photo 2, group 1: photos 3-6, group 2: photos 7-8)
        let counts = groups_on_page_with_counts(2..9, &groups);
        assert_eq!(counts, vec![(0, 1), (1, 4), (2, 2)]);
    }

    #[test]
    fn test_groups_on_page_with_counts_empty_range() {
        let groups = GroupInfo::new(&[5, 5]);
        let counts = groups_on_page_with_counts(0..0, &groups);
        assert_eq!(counts, vec![]);
    }
}
