//! Book layout solver that distributes photos across multiple pages.
//!
//! This module implements a two-phase approach for book layout optimization:
//! 1. **MIP Phase**: Use Mixed Integer Programming to find a feasible initial
//!    assignment of photos to pages, respecting group constraints.
//! 2. **Local Search Phase**: Refine the assignment using Variable Neighborhood
//!    Search (VNS) to improve coverage and balance.
//!
//! The module provides:
//! - High-level `solve()` API for complete book layout optimization
//! - `GaPageLayoutEvaluator` to connect single-page GA solver to book solver
//! - Internal modules for MIP, local search, feasibility checking, and caching

mod cache;
mod cost;
mod feasibility;
mod local_search;
mod mip;
mod model;

// Re-export public types
pub use model::{GroupInfo, PageAssignment, Params, ValidationError};
pub use local_search::PageLayoutEvaluator;

use super::page_layout_solver::run_ga;
use crate::models::{BookLayout, Canvas, GaConfig, Photo};
use cost::{AssignmentCost, PageCost};
use thiserror::Error;

/// Error type for book layout solver.
#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Parameter validation failed: {0}")]
    InvalidParams(#[from] ValidationError),

    #[error("MIP solver failed: {0}")]
    MipFailed(#[from] mip::MipError),

    #[error("No photos provided")]
    EmptyInput,
}

/// Result of the book layout solver.
#[derive(Debug, Clone)]
pub struct SolverResult {
    /// The final page assignment (cut points).
    pub assignment: PageAssignment,
    /// Cost breakdown for the assignment.
    pub cost: AssignmentCost,
    /// Number of local search iterations performed.
    pub iterations: usize,
    /// Number of cache hits during evaluation.
    pub cache_hits: usize,
}

/// Evaluator that uses the GA-based page layout solver.
///
/// This adapter connects the single-page GA solver (`page_layout_solver::run_ga`)
/// to the book layout solver's `PageLayoutEvaluator` trait.
struct GaPageLayoutEvaluator<'a> {
    canvas: &'a Canvas,
    ga_config: &'a GaConfig,
}

impl PageLayoutEvaluator for GaPageLayoutEvaluator<'_> {
    fn evaluate(&mut self, photos: &[Photo]) -> PageCost {
        let result = run_ga(photos, self.canvas, self.ga_config);
        
        // Convert CostBreakdown to PageCost
        PageCost::from(&result.cost_breakdown)
    }
}

/// Solves the book layout problem using MIP + local search.
///
/// # Algorithm
/// 1. Validate parameters
/// 2. Build GroupInfo from photos
/// 3. Run MIP solver to get initial feasible assignment
/// 4. Run local search to refine the assignment
/// 5. Return optimized assignment with cost
///
/// # Arguments
/// * `photos` - All photos to layout (must be sorted by group then timestamp)
/// * `params` - Solver parameters
/// * `canvas` - Canvas configuration for page layouts
/// * `ga_config` - GA configuration for single-page solver
///
/// # Returns
/// `SolverResult` with optimized assignment and cost, or `SolverError`.
pub fn solve(
    photos: &[Photo],
    params: &Params,
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> Result<SolverResult, SolverError> {
    // Handle empty input
    if photos.is_empty() {
        return Err(SolverError::EmptyInput);
    }

    // Validate parameters
    params.validate(photos.len())?;

    // Build group information from photos
    let groups = GroupInfo::from_photos(photos);

    // Phase 1: MIP solver for initial assignment
    let initial_assignment = mip::solve_mip(&groups, params)?;

    // Phase 2: Local search refinement
    let mut evaluator = GaPageLayoutEvaluator { canvas, ga_config };
    
    let (final_assignment, cost, iterations) = local_search::improve(
        initial_assignment,
        photos,
        &groups,
        params,
        &mut evaluator,
    );

    Ok(SolverResult {
        assignment: final_assignment,
        cost,
        iterations,
        cache_hits: 0, // TODO: Track cache hits if needed
    })
}

/// Legacy entry point for backward compatibility.
///
/// This function maintains the old single-page stub behavior for now.
/// Future versions will use the new `solve()` function above.
///
/// Currently, this is a stub implementation that places all photos on a single page.
/// Future versions will implement intelligent photo distribution based on:
/// - Lexicographic ordering of photo groups
/// - Temporal ordering within groups (timestamps)
/// - Optimal page filling strategies
///
/// # Arguments
///
/// * `photos` - All photos to layout in the book
/// * `canvas` - Canvas configuration for each page
/// * `ga_config` - Genetic algorithm configuration (includes seed)
///
/// # Returns
///
/// A `BookLayout` containing one or more pages with optimized photo placements.
pub(crate) fn solve_book_layout(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> BookLayout {
    // Handle empty photo case
    if photos.is_empty() {
        return BookLayout::new(vec![]);
    }

    // Current stub implementation: place all photos on a single page
    let ga_result = run_ga(photos, canvas, ga_config);

    let centered_page = ga_result.layout.centered();

    BookLayout::single_page(centered_page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FitnessWeights, IslandConfig};

    #[test]
    fn test_solve_book_layout_single_page() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.5, "group1".to_string()),
        ];

        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
        let ga_config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.05,
            weights: FitnessWeights::default(),
            timeout: None,
            no_improvement_limit: None,
            island_config: IslandConfig::default(),
            seed: 42,
        };

        let book = solve_book_layout(&photos, &canvas, &ga_config);

        assert_eq!(book.page_count(), 1);
        assert_eq!(book.total_photo_count(), 2);
        assert!(!book.is_empty());
    }

    #[test]
    fn test_solve_book_layout_empty() {
        let photos = vec![];

        let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
        let ga_config = GaConfig::default();

        let book = solve_book_layout(&photos, &canvas, &ga_config);

        assert_eq!(book.page_count(), 0);
        assert_eq!(book.total_photo_count(), 0);
        assert!(book.is_empty());
    }

    // Integration tests for the new solve() API
    mod integration {
        use super::*;
        use std::time::Duration;

        fn create_test_params() -> Params {
            Params {
                photos_per_page_min: 4,
                photos_per_page_max: 10,
                page_min: 1,
                page_max: 5,
                page_target: 3,
                group_min_photos: 2,
                group_max_per_page: 3,
                weight_even: 1.0,
                weight_split: 5.0,  // Penalize splits heavily
                weight_pages: 1.0,
                search_timeout: Duration::from_millis(100),
                max_coverage_cost: 0.5,
            }
        }

        #[test]
        fn test_solve_single_group() {
            // 10 photos in one group
            let photos: Vec<Photo> = (0..10)
                .map(|_| Photo::new(1.5, 1.0, "groupA".to_string()))
                .collect();

            let params = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
            let ga_config = GaConfig {
                population: 10,
                generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let result = solve(&photos, &params, &canvas, &ga_config);
            
            assert!(result.is_ok());
            let result = result.unwrap();
            
            // Should fit in one or two pages (depending on MIP/local search)
            assert!(result.assignment.num_pages() >= 1);
            assert!(result.assignment.num_pages() <= 3);
            assert_eq!(result.assignment.total_photos(), 10);
        }

        #[test]
        fn test_solve_multiple_groups() {
            // 3 groups with 5 photos each (15 total)
            let mut photos = Vec::new();
            for group in &["groupA", "groupB", "groupC"] {
                for _ in 0..5 {
                    photos.push(Photo::new(1.5, 1.0, group.to_string()));
                }
            }

            let params = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
            let ga_config = GaConfig {
                population: 10,
                generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let result = solve(&photos, &params, &canvas, &ga_config);
            
            assert!(result.is_ok());
            let result = result.unwrap();
            
            // Should fit reasonably given constraints
            assert!(result.assignment.num_pages() >= 2);
            assert!(result.assignment.num_pages() <= 4);
            assert_eq!(result.assignment.total_photos(), 15);
            
            // Check that each page respects size constraints
            for page_idx in 0..result.assignment.num_pages() {
                let page_size = result.assignment.page_size(page_idx);
                assert!(
                    page_size >= params.photos_per_page_min,
                    "Page {} has {} photos, min is {}",
                    page_idx,
                    page_size,
                    params.photos_per_page_min
                );
                assert!(
                    page_size <= params.photos_per_page_max,
                    "Page {} has {} photos, max is {}",
                    page_idx,
                    page_size,
                    params.photos_per_page_max
                );
            }
        }

        #[test]
        fn test_solve_empty_photos() {
            let photos: Vec<Photo> = vec![];
            let params = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
            let ga_config = GaConfig::default();

            let result = solve(&photos, &params, &canvas, &ga_config);
            
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), SolverError::EmptyInput));
        }

        #[test]
        fn test_solve_infeasible_params() {
            // 20 photos, but params require at least 50 capacity
            let photos: Vec<Photo> = (0..20)
                .map(|_| Photo::new(1.5, 1.0, "groupA".to_string()))
                .collect();

            let mut params = create_test_params();
            params.page_min = 5;
            params.page_max = 10;
            params.photos_per_page_min = 10;
            params.photos_per_page_max = 20;
            // min capacity = 5 * 10 = 50, but we only have 20 photos

            let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
            let ga_config = GaConfig::default();

            let result = solve(&photos, &params, &canvas, &ga_config);
            
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                SolverError::InvalidParams(_)
            ));
        }

        #[test]
        fn test_solve_iterations_tracked() {
            let photos: Vec<Photo> = (0..12)
                .map(|_| Photo::new(1.5, 1.0, "groupA".to_string()))
                .collect();

            let params = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0, 0.0);
            let ga_config = GaConfig {
                population: 10,
                generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let result = solve(&photos, &params, &canvas, &ga_config).unwrap();
            
            // Local search should have run at least one iteration
            assert!(result.iterations > 0);
        }
    }
}
