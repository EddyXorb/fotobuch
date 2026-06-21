use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Configuration parameters for the book layout solver.
///
/// Corresponds to the parameters in the DP formulation
/// (`docs/design/book_layout_solver_dp/dp.typ`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLayoutSolverConfig {
    /// Target number of pages (s in the DP formulation).
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
    /// Weight for evenness objective (w_1 in the DP formulation).
    #[serde(default = "default_weight_even")]
    pub weight_even: f64,
    /// Weight for split penalty (w_2 in the DP formulation).
    #[serde(default = "default_weight_split")]
    pub weight_split: f64,
    /// Weight for page count deviation (w_3 in the DP formulation).
    #[serde(default = "default_weight_pages")]
    pub weight_pages: f64,
    /// Timeout for book layout solver.
    #[serde(default = "default_search_timeout")]
    pub search_timeout: Duration,
    /// Maximum coverage cost threshold (pages above this are considered "bad").
    #[serde(default = "default_max_coverage_cost")]
    pub max_coverage_cost: f64,
    /// Whether to run local search after the DP to improve page assignments.
    #[serde(default = "default_enable_local_search")]
    pub enable_local_search: bool,
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
    false
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let config = BookLayoutSolverConfig::default();

        assert_eq!(config.page_target, 12);
        assert_eq!(config.page_min, 1);
        assert_eq!(config.page_max, 26);
        assert_eq!(config.photos_per_page_min, 1);
        assert_eq!(config.photos_per_page_max, 20);
        assert_eq!(config.group_max_per_page, 5);
        assert_eq!(config.group_min_photos, 1);
        assert_eq!(config.weight_even, 1.0);
        assert_eq!(config.weight_split, 10.0);
        assert_eq!(config.weight_pages, 5.0);
        assert_eq!(config.search_timeout, Duration::from_secs(30));
        assert_eq!(config.max_coverage_cost, 0.95);
        assert!(!config.enable_local_search);
    }

    #[test]
    fn test_serde_defaults() {
        // Test that serde defaults work for missing fields
        let yaml = "{}";
        let config: BookLayoutSolverConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.page_target, 12);
        assert_eq!(config.page_min, 1);
        assert_eq!(config.page_max, 26);
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
        assert_eq!(config.page_max, 26); // Default
        assert_eq!(config.weight_split, 10.0); // Default
    }

    #[test]
    fn test_legacy_config_with_mip_fields_still_loads() {
        // Old YAML files with removed MIP fields must still parse (serde ignores unknown fields).
        let yaml = r#"
page_target: 20
mip_rel_gap: 0.0001
max_photos_for_split: 300
split_group_boundary_slack: 5
"#;
        let config: BookLayoutSolverConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.page_target, 20);
    }
}
