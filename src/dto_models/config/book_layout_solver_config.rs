use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Configuration parameters for the book layout solver.
///
/// Corresponds to the parameters in the MIP formulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLayoutSolverConfig {
    /// Target number of pages (s in MIP).
    #[serde(default = "default_page_target")]
    pub page_target: usize,
    /// Minimum number of pages (b_min).
    #[serde(default = "default_page_min")]
    pub page_min: usize,
    /// Maximum number of pages (b_max).
    #[serde(default = "default_page_max")]
    pub page_max: usize,
    /// Minimum photos per page (p_min).
    #[serde(default = "default_photos_per_page_min")]
    pub photos_per_page_min: usize,
    /// Maximum photos per page (p_max).
    #[serde(default = "default_photos_per_page_max")]
    pub photos_per_page_max: usize,
    /// Maximum number of groups per page (g_max).
    #[serde(default = "default_group_max_per_page")]
    pub group_max_per_page: usize,
    /// Minimum photos in a split group (g_min).
    #[serde(default = "default_group_min_photos")]
    pub group_min_photos: usize,
    /// Weight for evenness objective (w_1 in MIP).
    #[serde(default = "default_weight_even")]
    pub weight_even: f64,
    /// Weight for split penalty (w_2 in MIP).
    #[serde(default = "default_weight_split")]
    pub weight_split: f64,
    /// Weight for page count deviation (w_3 in MIP).
    #[serde(default = "default_weight_pages")]
    pub weight_pages: f64,
    /// Timeout for local search.
    #[serde(default = "default_search_timeout")]
    pub search_timeout: Duration,
    /// Maximum coverage cost threshold (pages above this are considered "bad").
    #[serde(default = "default_max_coverage_cost")]
    pub max_coverage_cost: f64,
    /// Whether to run local search after MIP to improve page assignments.
    #[serde(default = "default_enable_local_search")]
    pub enable_local_search: bool,
    /// Relative MIP optimality gap (0.0 = exact, 0.01 = 1% tolerance).
    #[serde(default = "default_mip_rel_gap")]
    pub mip_rel_gap: f64,
    /// Maximum number of photos before triggering a split for large instances.
    #[serde(default = "default_max_photos_for_split")]
    pub max_photos_for_split: usize,
    /// Allowed deviation from ideal split point to find group boundaries.
    #[serde(default = "default_split_group_boundary_slack")]
    pub split_group_boundary_slack: usize,
}

// Default functions for serde
fn default_page_target() -> usize {
    12
}

fn default_page_min() -> usize {
    1
}

fn default_page_max() -> usize {
    26
}

fn default_photos_per_page_min() -> usize {
    1
}

fn default_photos_per_page_max() -> usize {
    20
}

fn default_group_max_per_page() -> usize {
    5
}

fn default_group_min_photos() -> usize {
    1
}

fn default_weight_even() -> f64 {
    1.0
}

fn default_weight_split() -> f64 {
    10.0
}

fn default_weight_pages() -> f64 {
    5.0
}

fn default_search_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_coverage_cost() -> f64 {
    0.95
}

fn default_enable_local_search() -> bool {
    true
}

fn default_mip_rel_gap() -> f64 {
    0.01
}

fn default_max_photos_for_split() -> usize {
    100
}

fn default_split_group_boundary_slack() -> usize {
    5
}

/// Error type for parameter validation.
#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("page_min ({page_min}) must be <= page_max ({page_max})")]
    PageMinMaxInvalid { page_min: usize, page_max: usize },

    #[error("page_target ({page_target}) must be in [{page_min}, {page_max}]")]
    PageTargetOutOfRange {
        page_target: usize,
        page_min: usize,
        page_max: usize,
    },

    #[error(
        "photos_per_page_min ({photos_per_page_min}) must be <= photos_per_page_max ({photos_per_page_max})"
    )]
    PhotosPerPageMinMaxInvalid {
        photos_per_page_min: usize,
        photos_per_page_max: usize,
    },

    #[error(
        "photos_per_page_min ({photos_per_page_min}) must be >= group_min_photos ({group_min_photos})"
    )]
    PhotosPerPageMinTooSmall {
        photos_per_page_min: usize,
        group_min_photos: usize,
    },

    #[error("group_max_per_page must be at least 1")]
    GroupMaxPerPageZero,

    #[error(
        "negative weight: weight_even={weight_even}, weight_split={weight_split}, weight_pages={weight_pages}"
    )]
    NegativeWeights {
        weight_even: f64,
        weight_split: f64,
        weight_pages: f64,
    },

    #[error("max_coverage_cost ({max_coverage_cost}) must be positive")]
    MaxCoverageCostInvalid { max_coverage_cost: f64 },

    #[error(
        "total photos ({total_photos}) cannot fit in page constraints: min capacity = {min_capacity}, max capacity = {max_capacity}"
    )]
    PhotoCountInfeasible {
        total_photos: usize,
        min_capacity: usize,
        max_capacity: usize,
    },
}

impl BookLayoutSolverConfig {
    /// Validates the parameters.
    ///
    /// Returns `Ok(())` if all parameters are consistent and feasible,
    /// or an error describing the first validation failure.
    ///
    /// # Arguments
    ///
    /// * `total_photos` - Total number of photos to be laid out
    pub fn validate(&self, total_photos: usize) -> Result<(), ValidationError> {
        // Check page bounds
        if self.page_min > self.page_max {
            return Err(ValidationError::PageMinMaxInvalid {
                page_min: self.page_min,
                page_max: self.page_max,
            });
        }

        // Check page target is in range
        if self.page_target < self.page_min || self.page_target > self.page_max {
            return Err(ValidationError::PageTargetOutOfRange {
                page_target: self.page_target,
                page_min: self.page_min,
                page_max: self.page_max,
            });
        }

        // Check photos per page bounds
        if self.photos_per_page_min > self.photos_per_page_max {
            return Err(ValidationError::PhotosPerPageMinMaxInvalid {
                photos_per_page_min: self.photos_per_page_min,
                photos_per_page_max: self.photos_per_page_max,
            });
        }

        // Check photos per page minimum vs. group minimum
        if self.photos_per_page_min < self.group_min_photos {
            return Err(ValidationError::PhotosPerPageMinTooSmall {
                photos_per_page_min: self.photos_per_page_min,
                group_min_photos: self.group_min_photos,
            });
        }

        // Check group max per page
        if self.group_max_per_page == 0 {
            return Err(ValidationError::GroupMaxPerPageZero);
        }

        // Check weights are non-negative
        if self.weight_even < 0.0 || self.weight_split < 0.0 || self.weight_pages < 0.0 {
            return Err(ValidationError::NegativeWeights {
                weight_even: self.weight_even,
                weight_split: self.weight_split,
                weight_pages: self.weight_pages,
            });
        }

        // Check max coverage cost
        if self.max_coverage_cost <= 0.0 {
            return Err(ValidationError::MaxCoverageCostInvalid {
                max_coverage_cost: self.max_coverage_cost,
            });
        }

        // Check that total photos can fit in page constraints
        let min_capacity = self.page_min * self.photos_per_page_min;
        let max_capacity = self.page_max * self.photos_per_page_max;

        if total_photos < min_capacity || total_photos > max_capacity {
            return Err(ValidationError::PhotoCountInfeasible {
                total_photos,
                min_capacity,
                max_capacity,
            });
        }

        Ok(())
    }
}

impl Default for BookLayoutSolverConfig {
    fn default() -> Self {
        Self {
            page_target: default_page_target(),
            page_min: default_page_min(),
            page_max: default_page_max(),
            photos_per_page_min: default_photos_per_page_min(),
            photos_per_page_max: default_photos_per_page_max(),
            group_max_per_page: default_group_max_per_page(),
            group_min_photos: default_group_min_photos(),
            weight_even: default_weight_even(),
            weight_split: default_weight_split(),
            weight_pages: default_weight_pages(),
            search_timeout: default_search_timeout(),
            max_coverage_cost: default_max_coverage_cost(),
            enable_local_search: default_enable_local_search(),
            mip_rel_gap: default_mip_rel_gap(),
            max_photos_for_split: default_max_photos_for_split(),
            split_group_boundary_slack: default_split_group_boundary_slack(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let config = BookLayoutSolverConfig::default();

        assert_eq!(config.page_target, 32);
        assert_eq!(config.page_min, 1);
        assert_eq!(config.page_max, 48);
        assert_eq!(config.photos_per_page_min, 2);
        assert_eq!(config.photos_per_page_max, 20);
        assert_eq!(config.group_max_per_page, 3);
        assert_eq!(config.group_min_photos, 2);
        assert_eq!(config.weight_even, 1.0);
        assert_eq!(config.weight_split, 10.0);
        assert_eq!(config.weight_pages, 5.0);
        assert_eq!(config.search_timeout, Duration::from_secs(30));
        assert_eq!(config.max_coverage_cost, 0.95);
        assert!(config.enable_local_search);
        assert_eq!(config.mip_rel_gap, 0.01);
        assert_eq!(config.max_photos_for_split, 100);
        assert_eq!(config.split_group_boundary_slack, 5);
    }

    #[test]
    fn test_serde_defaults() {
        // Test that serde defaults work for missing fields
        let yaml = "{}";
        let config: BookLayoutSolverConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.page_target, 32);
        assert_eq!(config.page_min, 1);
        assert_eq!(config.page_max, 48);
    }

    #[test]
    fn test_partial_serde_defaults() {
        // Test that serde defaults work for partially specified config
        let yaml = r#"
page_target: 25
weight_even: 2.0
"#;
        let config: BookLayoutSolverConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.page_target, 25); // Specified
        assert_eq!(config.weight_even, 2.0); // Specified
        assert_eq!(config.page_min, 1); // Default
        assert_eq!(config.page_max, 48); // Default
        assert_eq!(config.weight_split, 10.0); // Default
    }
}
