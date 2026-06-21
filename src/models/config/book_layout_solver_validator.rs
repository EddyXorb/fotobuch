use super::{BookLayoutSolverConfig, ValidationError};

/// Validates a [`BookLayoutSolverConfig`] against application state.
///
/// Separated from the DTO so that `BookLayoutSolverConfig` stays pure data
/// and the validation logic (which requires `total_photos`) lives here.
pub fn validate_book_layout_solver_config(
    config: &BookLayoutSolverConfig,
    total_photos: usize,
) -> Result<(), ValidationError> {
    if config.page_min > config.page_max {
        return Err(ValidationError::PageMinMaxInvalid {
            page_min: config.page_min,
            page_max: config.page_max,
        });
    }

    if config.page_target < config.page_min || config.page_target > config.page_max {
        return Err(ValidationError::PageTargetOutOfRange {
            page_target: config.page_target,
            page_min: config.page_min,
            page_max: config.page_max,
        });
    }

    if config.photos_per_page_min > config.photos_per_page_max {
        return Err(ValidationError::PhotosPerPageMinMaxInvalid {
            photos_per_page_min: config.photos_per_page_min,
            photos_per_page_max: config.photos_per_page_max,
        });
    }

    if config.photos_per_page_min < config.group_min_photos {
        return Err(ValidationError::PhotosPerPageMinTooSmall {
            photos_per_page_min: config.photos_per_page_min,
            group_min_photos: config.group_min_photos,
        });
    }

    if config.group_max_per_page == 0 {
        return Err(ValidationError::GroupMaxPerPageZero);
    }

    if config.weight_even < 0.0 || config.weight_split < 0.0 || config.weight_pages < 0.0 {
        return Err(ValidationError::NegativeWeights {
            weight_even: config.weight_even,
            weight_split: config.weight_split,
            weight_pages: config.weight_pages,
        });
    }

    if config.max_coverage_cost <= 0.0 {
        return Err(ValidationError::MaxCoverageCostInvalid {
            max_coverage_cost: config.max_coverage_cost,
        });
    }

    let min_capacity = config.page_min * config.photos_per_page_min;
    let max_capacity = config.page_max * config.photos_per_page_max;

    if total_photos < min_capacity || total_photos > max_capacity {
        return Err(ValidationError::PhotoCountInfeasible {
            total_photos,
            min_capacity,
            max_capacity,
        });
    }

    Ok(())
}
