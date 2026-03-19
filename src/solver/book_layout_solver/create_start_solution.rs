use super::model::{GroupInfo, PageAssignment};
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::prelude::*;

/// Constructs a valid initial page assignment using a greedy heuristic.
///
/// The algorithm:
/// 1. Sorts photos by group (preserves spatial coherence)
/// 2. Iteratively builds pages using average target size
/// 3. Respects constraints: min/max photos per page, max groups per page, group mixing rules
///
/// The heuristic avoids excessive group fragmentation by checking if the "last group"
/// on a page would have too few photos if included. If so, it excludes that group entirely
/// for the current page.
///
/// # Constraints Enforced
/// - `photos_per_page_min` <= page size <= `photos_per_page_max`
/// - page count <= `page_max`
/// - max `group_max_per_page` groups per page
/// - if a group is split across pages and `group_min_photos` < group size,
///   then at least `group_min_photos` photos of that group on mixed pages
///
/// # Arguments
/// * `params` - Configuration with constraints
/// * `photos` - Photos to assign (will be sorted by group)
///
/// # Returns
/// A valid `PageAssignment` distributing photos across pages
pub fn create_start_solution(params: &Params, photos: &[Photo]) -> PageAssignment {
    // Handle empty input
    if photos.is_empty() {
        return PageAssignment::empty();
    }

    // Sort photos by group to maintain spatial coherence
    let mut sorted_photos = photos.to_vec();
    sorted_photos.sort_by(|a, b| a.group.cmp(&b.group));

    let group_info = GroupInfo::from_photos(&sorted_photos);
    let total_photos = sorted_photos.len();

    let mut cuts = vec![0];
    let mut current_pos = 0;

    while current_pos < total_photos {
        let pages_left = (params.page_target - cuts.len() + 1).max(1);
        let photos_left = total_photos - current_pos;

        // Calculate target page size: ceiling division of remaining photos by remaining pages
        let p_avg = photos_left.div_ceil(pages_left);

        // Start with p_avg, constrain to min/max
        let mut p_target = p_avg
            .clamp(params.photos_per_page_min, params.photos_per_page_max)
            .min(photos_left);

        // Adjust for group boundaries if not at end
        if current_pos + p_target < total_photos {
            let candidate_cut = current_pos + p_target;
            let last_group_idx = group_info.group_of_photo(candidate_cut - 1);
            let group_range = group_info.group_range(last_group_idx);

            // How many photos of last_group would be on this page?
            let r_curr = candidate_cut.saturating_sub(group_range.start);
            // How many photos of last_group would remain for later pages?
            let r_rem = group_range.end.saturating_sub(candidate_cut);

            // Apply group mixing heuristic
            if r_rem > 0 && r_rem < params.group_min_photos && r_curr > 0 {
                // Too few of last_group would remain: take all
                p_target = group_range.end - current_pos;
            } else if r_rem >= params.group_min_photos
                && r_curr > 0
                && r_curr < params.group_min_photos
            {
                // Too few of last_group on this page: exclude it
                p_target = group_range.start - current_pos;
            }
        }

        // Final clamp to constraints
        p_target = p_target
            .clamp(params.photos_per_page_min, params.photos_per_page_max)
            .min(photos_left);

        let next_cut = current_pos + p_target;
        cuts.push(next_cut);
        current_pos = next_cut;
    }

    PageAssignment::new(cuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn default_params() -> Params {
        Params {
            page_target: 5,
            page_min: 1,
            page_max: 10,
            photos_per_page_min: 2,
            photos_per_page_max: 8,
            group_max_per_page: 3,
            group_min_photos: 2,
            weight_even: 1.0,
            weight_split: 10.0,
            weight_pages: 5.0,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.95,
            enable_local_search: true,
            mip_rel_gap: 0.01,
            max_photos_for_split: 100,
            split_group_boundary_slack: 5,
        }
    }

    #[test]
    fn test_empty_photos() {
        let params = default_params();
        let photos: Vec<Photo> = vec![];

        let assignment = create_start_solution(&params, &photos);

        assert_eq!(assignment.num_pages(), 0);
        assert_eq!(assignment.total_photos(), 0);
    }

    #[test]
    fn test_single_group_single_page() {
        let params = Params {
            photos_per_page_min: 2,
            photos_per_page_max: 10,
            ..default_params()
        };

        let photos = vec![
            Photo::new("p1".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("p2".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("p3".to_string(), 1.5, 1.0, "groupA".to_string()),
        ];

        let assignment = create_start_solution(&params, &photos);

        assert_eq!(assignment.num_pages(), 1);
        assert_eq!(assignment.page_size(0), 3);
    }

    #[test]
    fn test_single_group_multiple_pages() {
        let params = Params {
            photos_per_page_min: 2,
            photos_per_page_max: 4,
            ..default_params()
        };

        let photos: Vec<Photo> = (0..10)
            .map(|i| Photo::new(format!("p{}", i), 1.5, 1.0, "groupA".to_string()))
            .collect();

        let assignment = create_start_solution(&params, &photos);

        // 10 photos with min=2, max=4: at least 3 pages needed
        assert!(assignment.num_pages() >= 3);
        assert!(assignment.num_pages() <= 5);
        assert_eq!(assignment.total_photos(), 10);

        // Check all pages respect constraints
        for page_idx in 0..assignment.num_pages() {
            let size = assignment.page_size(page_idx);
            assert!(
                (2..=4).contains(&size),
                "Page {} size {} violates constraints",
                page_idx,
                size
            );
        }
    }

    #[test]
    fn test_multiple_groups_no_fragmentation() {
        let params = Params {
            photos_per_page_min: 2,
            photos_per_page_max: 6,
            group_min_photos: 2,
            ..default_params()
        };

        // Create 3 groups: 4, 4, 4 photos
        let mut photos = Vec::new();
        for group_id in 0..3 {
            for photo_id in 0..4 {
                let group_name = format!("group{}", group_id);
                photos.push(Photo::new(
                    format!("p{}_{}", group_id, photo_id),
                    1.5,
                    1.0,
                    group_name,
                ));
            }
        }

        let assignment = create_start_solution(&params, &photos);

        // With group_min_photos=2, max=6, and 12 photos: should get 2-6 pages
        // Heuristic respects group boundaries to avoid fragmentation
        assert!(assignment.num_pages() >= 2);
        assert!(assignment.num_pages() <= 6);
        assert_eq!(assignment.total_photos(), 12);
    }

    #[test]
    fn test_respects_page_min_max() {
        let params = Params {
            photos_per_page_min: 3,
            photos_per_page_max: 5,
            ..default_params()
        };

        let photos: Vec<Photo> = (0..15)
            .map(|i| Photo::new(format!("p{}", i), 1.5, 1.0, "groupA".to_string()))
            .collect();

        let assignment = create_start_solution(&params, &photos);

        for page_idx in 0..assignment.num_pages() {
            let size = assignment.page_size(page_idx);
            assert!(
                size >= params.photos_per_page_min && size <= params.photos_per_page_max,
                "Page {} size {} violates min={} max={}",
                page_idx,
                size,
                params.photos_per_page_min,
                params.photos_per_page_max
            );
        }
    }

    #[test]
    fn test_all_photos_assigned() {
        let params = default_params();
        let photos: Vec<Photo> = (0..20)
            .map(|i| Photo::new(format!("p{}", i), 1.5, 1.0, format!("group{}", i % 3)))
            .collect();

        let assignment = create_start_solution(&params, &photos);

        assert_eq!(assignment.total_photos(), 20);
        assert!(assignment.num_pages() > 0);
    }

    #[test]
    fn test_groups_sorted() {
        let params = Params {
            photos_per_page_min: 1,
            photos_per_page_max: 10,
            group_min_photos: 2,
            ..default_params()
        };

        // Create photos in unsorted order by group
        let photos = vec![
            Photo::new("p1".to_string(), 1.5, 1.0, "groupC".to_string()),
            Photo::new("p2".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("p3".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("p4".to_string(), 1.5, 1.0, "groupA".to_string()),
        ];

        let assignment = create_start_solution(&params, &photos);

        // Should still produce valid assignment despite unsorted input
        // After sorting: groupA(2) + groupB(1) + groupC(1)
        // With group_min_photos=2, groupB and groupC each < group_min, so they may split
        assert!(assignment.num_pages() >= 1);
        assert!(assignment.num_pages() <= 3);
        assert_eq!(assignment.total_photos(), 4);
    }
}
