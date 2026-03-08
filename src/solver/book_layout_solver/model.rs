//! Data model for book layout solver.
//!
//! This module defines:
//! - `Params`: configuration parameters for the book layout solver
//! - `ValidationError`: parameter validation errors
//! - `PageAssignment`: partitioning of photos into pages
//! - `GroupInfo`: information about photo groups
//! - `PageCost`, `AssignmentCost`: cost structures for evaluating layouts

pub use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::prelude::*;
use std::ops::Range;

/// Information about photo groups.
///
/// Stores cumulative group sizes for efficient lookup.
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// Cumulative group sizes: group_ends[i] = sum of sizes of groups 0..=i.
    /// group_ends[0] = size of group 0, group_ends[1] = size of groups 0+1, etc.
    group_ends: Vec<usize>,
}

impl GroupInfo {
    /// Creates a new `GroupInfo` from group sizes.
    ///
    /// # Arguments
    ///
    /// * `group_sizes` - Sizes of each group (number of photos per group)
    pub fn new(group_sizes: &[usize]) -> Self {
        let mut cumulative = 0;
        let group_ends = group_sizes
            .iter()
            .map(|&size| {
                cumulative += size;
                cumulative
            })
            .collect();
        Self { group_ends }
    }

    /// Creates a `GroupInfo` from a slice of photos.
    ///
    /// Groups photos by their `group` field, counting consecutive photos
    /// with the same group identifier. Photos must be pre-sorted by group.
    ///
    /// # Arguments
    ///
    /// * `photos` - Photos sorted by group (and optionally by timestamp within group)
    ///
    /// # Returns
    ///
    /// A new `GroupInfo` with cumulative group sizes.
    pub fn from_photos(photos: &[Photo]) -> Self {
        if photos.is_empty() {
            return Self { group_ends: vec![] };
        }

        let mut group_sizes = Vec::new();
        let mut current_group = &photos[0].group;
        let mut current_count = 0;

        for photo in photos {
            if &photo.group == current_group {
                current_count += 1;
            } else {
                group_sizes.push(current_count);
                current_group = &photo.group;
                current_count = 1;
            }
        }

        // Push the last group
        group_sizes.push(current_count);

        Self::new(&group_sizes)
    }

    /// Returns the number of groups.
    pub fn num_groups(&self) -> usize {
        self.group_ends.len()
    }

    /// Returns the size of a specific group.
    ///
    /// # Panics
    ///
    /// Panics if `group_index` is out of bounds.
    pub fn group_size(&self, group_index: usize) -> usize {
        if group_index == 0 {
            self.group_ends[0]
        } else {
            self.group_ends[group_index] - self.group_ends[group_index - 1]
        }
    }

    /// Returns the group index of a specific photo.
    ///
    /// # Panics
    ///
    /// Panics if `photo_index` is out of bounds.
    pub fn group_of_photo(&self, photo_index: usize) -> usize {
        // Binary search for the first group_ends[i] > photo_index
        self.group_ends
            .iter()
            .position(|&end| photo_index < end)
            .expect("photo_index out of bounds")
    }

    /// Returns the range of photo indices for a specific group.
    ///
    /// # Panics
    ///
    /// Panics if `group_index` is out of bounds.
    pub fn group_range(&self, group_index: usize) -> Range<usize> {
        let start = if group_index == 0 {
            0
        } else {
            self.group_ends[group_index - 1]
        };
        let end = self.group_ends[group_index];
        start..end
    }

    /// Returns the total number of photos.
    pub fn total_photos(&self) -> usize {
        self.group_ends.last().copied().unwrap_or(0)
    }
}

/// Page assignment: partitions a sequence of photos into pages.
///
/// Represented by cut points: page j contains photos [cuts[j]..cuts[j+1]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAssignment {
    /// Cut points: cuts[0] = 0, cuts[last] = total_photos.
    /// Page j contains photos [cuts[j]..cuts[j+1]).
    /// Length = num_pages + 1.
    cuts: Vec<usize>,
}

impl PageAssignment {
    /// Creates a new `PageAssignment` from cut points.
    ///
    /// # Panics
    ///
    /// Panics if cuts are not strictly increasing or if cuts[0] != 0.
    pub fn new(cuts: Vec<usize>) -> Self {
        assert!(!cuts.is_empty(), "cuts must not be empty");
        assert_eq!(cuts[0], 0, "cuts[0] must be 0");
        for i in 1..cuts.len() {
            assert!(cuts[i] > cuts[i - 1], "cuts must be strictly increasing");
        }
        Self { cuts }
    }

    /// Creates an empty assignment (zero pages, zero photos).
    pub fn empty() -> Self {
        Self { cuts: vec![0] }
    }

    /// Creates an assignment with a single page containing all photos.
    pub fn single_page(total_photos: usize) -> Self {
        Self {
            cuts: vec![0, total_photos],
        }
    }

    /// Returns the number of pages.
    pub fn num_pages(&self) -> usize {
        self.cuts.len() - 1
    }

    /// Returns the number of photos on a specific page.
    ///
    /// # Panics
    ///
    /// Panics if `page_index` is out of bounds.
    pub fn page_size(&self, page_index: usize) -> usize {
        self.cuts[page_index + 1] - self.cuts[page_index]
    }

    /// Returns the range of photo indices for a specific page.
    ///
    /// # Panics
    ///
    /// Panics if `page_index` is out of bounds.
    pub fn page_range(&self, page_index: usize) -> Range<usize> {
        self.cuts[page_index]..self.cuts[page_index + 1]
    }

    /// Returns the indices of the two pages adjacent to a cut point.
    ///
    /// Returns `(left_page, right_page)` where `left_page` ends at `cut_index`
    /// and `right_page` starts at `cut_index`.
    ///
    /// # Panics
    ///
    /// Panics if `cut_index` is 0 or >= cuts.len() - 1 (boundary cuts have no adjacent pages on both sides).
    pub fn affected_pages(&self, cut_index: usize) -> (usize, usize) {
        assert!(cut_index > 0 && cut_index < self.cuts.len() - 1);
        (cut_index - 1, cut_index)
    }

    /// Returns the total number of photos.
    pub fn total_photos(&self) -> usize {
        *self.cuts.last().unwrap()
    }

    /// Returns all cut points.
    pub fn cuts(&self) -> &[usize] {
        &self.cuts
    }
}

#[cfg(test)]
mod tests {
    use crate::dto_models::ValidationError;

    use super::*;
    use std::time::Duration;

    #[test]
    fn test_params_validate_valid() {
        let params = Params {
            page_target: 5,
            page_min: 3,
            page_max: 10,
            photos_per_page_min: 5,
            photos_per_page_max: 15,
            group_max_per_page: 3,
            group_min_photos: 3,
            weight_even: 1.0,
            weight_split: 2.0,
            weight_pages: 0.5,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        };

        assert!(params.validate(50).is_ok());
    }

    #[test]
    fn test_params_validate_page_min_max_invalid() {
        let params = Params {
            page_min: 10,
            page_max: 5,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::PageMinMaxInvalid {
                page_min: 10,
                page_max: 5
            })
        );
    }

    #[test]
    fn test_params_validate_page_target_out_of_range() {
        let params = Params {
            page_target: 15,
            page_min: 3,
            page_max: 10,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::PageTargetOutOfRange {
                page_target: 15,
                page_min: 3,
                page_max: 10
            })
        );
    }

    #[test]
    fn test_params_validate_photos_per_page_min_max_invalid() {
        let params = Params {
            photos_per_page_min: 20,
            photos_per_page_max: 10,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::PhotosPerPageMinMaxInvalid {
                photos_per_page_min: 20,
                photos_per_page_max: 10
            })
        );
    }

    #[test]
    fn test_params_validate_photos_per_page_min_too_small() {
        let params = Params {
            photos_per_page_min: 2,
            group_min_photos: 5,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::PhotosPerPageMinTooSmall {
                photos_per_page_min: 2,
                group_min_photos: 5
            })
        );
    }

    #[test]
    fn test_params_validate_group_max_per_page_zero() {
        let params = Params {
            group_max_per_page: 0,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::GroupMaxPerPageZero)
        );
    }

    #[test]
    fn test_params_validate_negative_weights() {
        let params = Params {
            weight_even: -1.0,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::NegativeWeights {
                weight_even: -1.0,
                weight_split: 2.0,
                weight_pages: 0.5
            })
        );
    }

    #[test]
    fn test_params_validate_max_coverage_cost_invalid() {
        let params = Params {
            max_coverage_cost: -0.1,
            ..default_params()
        };

        assert_eq!(
            params.validate(50),
            Err(ValidationError::MaxCoverageCostInvalid {
                max_coverage_cost: -0.1
            })
        );
    }

    #[test]
    fn test_params_validate_photo_count_too_small() {
        let params = default_params();
        // min capacity = 3 * 5 = 15, max capacity = 10 * 15 = 150
        // 10 photos is too few
        assert_eq!(
            params.validate(10),
            Err(ValidationError::PhotoCountInfeasible {
                total_photos: 10,
                min_capacity: 15,
                max_capacity: 150
            })
        );
    }

    #[test]
    fn test_params_validate_photo_count_too_large() {
        let params = default_params();
        // max capacity = 10 * 15 = 150
        // 200 photos is too many
        assert_eq!(
            params.validate(200),
            Err(ValidationError::PhotoCountInfeasible {
                total_photos: 200,
                min_capacity: 15,
                max_capacity: 150
            })
        );
    }

    fn default_params() -> Params {
        Params {
            page_target: 5,
            page_min: 3,
            page_max: 10,
            photos_per_page_min: 5,
            photos_per_page_max: 15,
            group_max_per_page: 3,
            group_min_photos: 3,
            weight_even: 1.0,
            weight_split: 2.0,
            weight_pages: 0.5,
            search_timeout: Duration::from_secs(10),
            max_coverage_cost: 0.1,
        }
    }

    #[test]
    fn test_group_info_basic() {
        let group_info = GroupInfo::new(&[3, 5, 2]);
        assert_eq!(group_info.num_groups(), 3);
        assert_eq!(group_info.total_photos(), 10);
        assert_eq!(group_info.group_size(0), 3);
        assert_eq!(group_info.group_size(1), 5);
        assert_eq!(group_info.group_size(2), 2);
    }

    #[test]
    fn test_group_info_group_of_photo() {
        let group_info = GroupInfo::new(&[3, 5, 2]);
        assert_eq!(group_info.group_of_photo(0), 0);
        assert_eq!(group_info.group_of_photo(2), 0);
        assert_eq!(group_info.group_of_photo(3), 1);
        assert_eq!(group_info.group_of_photo(7), 1);
        assert_eq!(group_info.group_of_photo(8), 2);
        assert_eq!(group_info.group_of_photo(9), 2);
    }

    #[test]
    fn test_group_info_group_range() {
        let group_info = GroupInfo::new(&[3, 5, 2]);
        assert_eq!(group_info.group_range(0), 0..3);
        assert_eq!(group_info.group_range(1), 3..8);
        assert_eq!(group_info.group_range(2), 8..10);
    }

    #[test]
    fn test_group_info_from_photos() {
        let photos = vec![
            Photo::new("photo_0".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("photo_1".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("photo_2".to_string(), 1.5, 1.0, "groupA".to_string()),
            Photo::new("photo_3".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("photo_4".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("photo_5".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("photo_6".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("photo_7".to_string(), 1.5, 1.0, "groupB".to_string()),
            Photo::new("photo_8".to_string(), 1.5, 1.0, "groupC".to_string()),
            Photo::new("photo_9".to_string(), 1.5, 1.0, "groupC".to_string()),
        ];

        let group_info = GroupInfo::from_photos(&photos);

        assert_eq!(group_info.num_groups(), 3);
        assert_eq!(group_info.group_size(0), 3);
        assert_eq!(group_info.group_size(1), 5);
        assert_eq!(group_info.group_size(2), 2);
        assert_eq!(group_info.total_photos(), 10);
    }

    #[test]
    fn test_group_info_from_photos_empty() {
        let photos: Vec<Photo> = vec![];
        let group_info = GroupInfo::from_photos(&photos);

        assert_eq!(group_info.num_groups(), 0);
        assert_eq!(group_info.total_photos(), 0);
    }

    #[test]
    fn test_page_assignment_basic() {
        let assignment = PageAssignment::new(vec![0, 5, 12, 20]);
        assert_eq!(assignment.num_pages(), 3);
        assert_eq!(assignment.total_photos(), 20);
        assert_eq!(assignment.page_size(0), 5);
        assert_eq!(assignment.page_size(1), 7);
        assert_eq!(assignment.page_size(2), 8);
        assert_eq!(assignment.page_range(0), 0..5);
        assert_eq!(assignment.page_range(1), 5..12);
        assert_eq!(assignment.page_range(2), 12..20);
    }

    #[test]
    fn test_page_assignment_affected_pages() {
        let assignment = PageAssignment::new(vec![0, 5, 12, 20]);
        assert_eq!(assignment.affected_pages(1), (0, 1));
        assert_eq!(assignment.affected_pages(2), (1, 2));
    }

    #[test]
    fn test_page_assignment_empty() {
        let assignment = PageAssignment::empty();
        assert_eq!(assignment.num_pages(), 0);
        assert_eq!(assignment.total_photos(), 0);
    }

    #[test]
    fn test_page_assignment_single_page() {
        let assignment = PageAssignment::single_page(15);
        assert_eq!(assignment.num_pages(), 1);
        assert_eq!(assignment.total_photos(), 15);
        assert_eq!(assignment.page_size(0), 15);
        assert_eq!(assignment.page_range(0), 0..15);
    }

    #[test]
    #[should_panic(expected = "cuts[0] must be 0")]
    fn test_page_assignment_new_invalid_first_cut() {
        PageAssignment::new(vec![1, 5, 10]);
    }

    #[test]
    #[should_panic(expected = "cuts must be strictly increasing")]
    fn test_page_assignment_new_non_increasing() {
        PageAssignment::new(vec![0, 5, 5, 10]);
    }
}
