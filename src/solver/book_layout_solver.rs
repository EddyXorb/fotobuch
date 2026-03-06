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
}
