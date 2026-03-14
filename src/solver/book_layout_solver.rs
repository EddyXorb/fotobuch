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
//! - Internal modules for MIP, local search, feasibility checking, and caching

mod cache;
mod create_start_solution;
mod feasibility;
mod local_search;
mod mip;
mod model;

// Re-export public types
pub use local_search::PageLayoutEvaluator;
pub use model::GroupInfo;
use tracing::{debug, info};

use super::data_models::book_layout::BookLayout;
use crate::dto_models::BookLayoutSolverConfig as Params;
use crate::solver::page_layout_solver::{self, GaResult};
use crate::solver::prelude::*;
use thiserror::Error;

/// Error type for book layout solver.
#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Parameter validation failed: {0}")]
    InvalidParams(#[from] crate::dto_models::ValidationError),

    #[error("MIP solver failed: {0}")]
    MipFailed(#[from] mip::MipError),
}

/// Solves the book layout problem using MIP + local search.
///
/// # Algorithm
/// 1. Validate parameters
/// 2. Build GroupInfo from photos
/// 3. Run MIP solver to get initial feasible assignment
/// 4. Run local search to refine the assignment
/// 5. Collect layouts from cache and build BookLayout
///
pub fn solve_book_layout(
    photos: &[Photo],
    params: &Params,
    canvas: &Canvas,
    ga_config: &GaConfig,
) -> Result<BookLayout, SolverError> {
    // Handle empty input
    if photos.is_empty() {
        return Ok(BookLayout::new(vec![]));
    }

    // Validate parameters
    params.validate(photos.len())?;

    // Build group information from photos
    let groups = GroupInfo::from_photos(photos);

    // Phase 0: Construction Heuristic for initial solution as Hint to Mip
    let hint = create_start_solution::create_start_solution(params, photos);
    debug!("Hint cuts: {:?}", hint.cuts());

    // Phase 1: MIP solver for initial assignment
    let initial_assignment = mip::solve_mip(&groups, params, Some(&hint))?;
    debug!("Cuts: {:?}", initial_assignment.cuts());

    // Phase 2: Evaluate pages (with optional local search refinement)
    let mut evaluator = GAPageEvaluator::new(canvas, ga_config);

    let (final_assignment, layout_cache) = if params.enable_local_search {
        info!("Start local search refinement..");

        let result =
            local_search::improve(initial_assignment, photos, &groups, params, &mut evaluator);

        info!(
            "Finished local search after {} iterations, start fitness: {:.3}, end fitness: {:.3}",
            result.iterations, result.start_fitness, result.end_fitness
        );
        (result.assignment, result.cache)
    } else {
        let mut cache = cache::PhotoCombinationCache::new();
        for page_idx in 0..initial_assignment.num_pages() {
            let range = initial_assignment.page_range(page_idx);
            let result = evaluator.evaluate(&photos[range.clone()]);
            cache.insert_if_better(&photos[range], result);
        }
        (initial_assignment, cache)
    };

    debug!("Cuts after local search: {:?}", final_assignment.cuts());

    // Phase 3: Build BookLayout from the layout cache
    let page_layouts: Vec<SolverPageLayout> = (0..final_assignment.num_pages())
        .filter_map(|page_idx| {
            let range = final_assignment.page_range(page_idx);
            layout_cache.get(&photos[range]).map(|r| r.layout.clone())
        })
        .collect();

    Ok(BookLayout::new(page_layouts))
}

/// Evaluator that runs the GA-based page layout solver.
struct GAPageEvaluator<'a> {
    canvas: &'a Canvas,
    ga_config: &'a GaConfig,
}

impl<'a> GAPageEvaluator<'a> {
    fn new(canvas: &'a Canvas, ga_config: &'a GaConfig) -> Self {
        Self { canvas, ga_config }
    }
}

impl PageLayoutEvaluator for GAPageEvaluator<'_> {
    fn evaluate(&mut self, photos: &[Photo]) -> GaResult {
        page_layout_solver::run_ga(photos, self.canvas, self.ga_config)
    }
}

#[cfg(test)]
mod tests {
    use crate::dto_models::BookLayoutSolverConfig;

    use super::*;

    #[test]
    fn test_solve_book_layout_single_page() {
        let photos = vec![
            Photo::new("photo_0".to_string(), 1.5, 1.0, "group1".to_string()),
            Photo::new("photo_1".to_string(), 1.0, 1.5, "group1".to_string()),
        ];

        let canvas = Canvas::new(297.0, 210.0, 5.0);
        let ga_config = GaConfig {
            seed: 42,
            ..GaConfig::default()
        };

        let solver_config = BookLayoutSolverConfig::default();

        let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

        assert_eq!(book.page_count(), 1);
        assert_eq!(book.total_photo_count(), 2);
        assert!(!book.is_empty());
    }

    #[test]
    fn test_solve_book_layout_empty() {
        let photos = vec![];

        let canvas = Canvas::new(297.0, 210.0, 5.0);
        let ga_config = GaConfig::default();
        let solver_config = BookLayoutSolverConfig::default();

        let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

        assert_eq!(book.page_count(), 0);
        assert_eq!(book.total_photo_count(), 0);
        assert!(book.is_empty());
    }

    // Integration tests for the new solve() API
    mod integration {
        use super::*;
        use std::time::Duration;

        fn create_test_params() -> BookLayoutSolverConfig {
            BookLayoutSolverConfig {
                photos_per_page_min: 4,
                photos_per_page_max: 10,
                page_min: 1,
                page_max: 5,
                page_target: 3,
                group_min_photos: 2,
                group_max_per_page: 3,
                weight_even: 1.0,
                weight_split: 5.0, // Penalize splits heavily
                weight_pages: 1.0,
                search_timeout: Duration::from_millis(100),
                max_coverage_cost: 0.5,
                enable_local_search: true,
                mip_rel_gap: 0.01,
            }
        }

        #[test]
        fn test_solve_single_group() {
            // 10 photos in one group
            let photos: Vec<Photo> = (0..10)
                .map(|i| Photo::new(format!("photo_{}", i), 1.5, 1.0, "groupA".to_string()))
                .collect();

            let solver_config = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0);
            let ga_config = GaConfig {
                population_size: 10,
                max_generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

            // Should fit in one or two pages (depending on MIP/local search)
            assert!(book.page_count() >= 1);
            assert!(book.page_count() <= 3);
            assert_eq!(book.total_photo_count(), 10);
        }

        #[test]
        fn test_solve_multiple_groups() {
            // 3 groups with 5 photos each (15 total)
            let mut photos = Vec::new();
            let mut id_counter = 0;
            for group in &["groupA", "groupB", "groupC"] {
                for _ in 0..5 {
                    photos.push(Photo::new(
                        format!("photo_{}", id_counter),
                        1.5,
                        1.0,
                        group.to_string(),
                    ));
                    id_counter += 1;
                }
            }

            let solver_config = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0);
            let ga_config = GaConfig {
                population_size: 10,
                max_generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

            // Should fit reasonably given constraints
            assert!(book.page_count() >= 2);
            assert!(book.page_count() <= 4);
            assert_eq!(book.total_photo_count(), 15);

            // Check that each page respects size constraints
            for (page_idx, page) in book.pages.iter().enumerate() {
                let page_size = page.placements.len();
                assert!(
                    page_size >= solver_config.photos_per_page_min,
                    "Page {} has {} photos, min is {}",
                    page_idx,
                    page_size,
                    solver_config.photos_per_page_min
                );
                assert!(
                    page_size <= solver_config.photos_per_page_max,
                    "Page {} has {} photos, max is {}",
                    page_idx,
                    page_size,
                    solver_config.photos_per_page_max
                );
            }
        }

        #[test]
        fn test_solve_empty_photos() {
            let photos: Vec<Photo> = vec![];
            let solver_config = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0);
            let ga_config = GaConfig::default();

            let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

            assert_eq!(book.page_count(), 0);
            assert!(book.is_empty());
        }

        #[test]
        fn test_solve_infeasible_params() {
            // 20 photos, but params require at least 50 capacity
            let photos: Vec<Photo> = (0..20)
                .map(|i| Photo::new(format!("photo_{}", i), 1.5, 1.0, "groupA".to_string()))
                .collect();

            let mut solver_config = create_test_params();
            solver_config.page_min = 5;
            solver_config.page_max = 10;
            solver_config.photos_per_page_min = 10;
            solver_config.photos_per_page_max = 20;
            // min capacity = 5 * 10 = 50, but we only have 20 photos

            let canvas = Canvas::new(297.0, 210.0, 5.0);
            let ga_config = GaConfig::default();

            let result = solve_book_layout(&photos, &solver_config, &canvas, &ga_config);

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), SolverError::InvalidParams(_)));
        }

        #[test]
        fn test_solve_success_with_valid_params() {
            let photos: Vec<Photo> = (0..12)
                .map(|i| Photo::new(format!("photo_{}", i), 1.5, 1.0, "groupA".to_string()))
                .collect();

            let solver_config = create_test_params();
            let canvas = Canvas::new(297.0, 210.0, 5.0);
            let ga_config = GaConfig {
                population_size: 10,
                max_generations: 3,
                seed: 42,
                ..GaConfig::default()
            };

            let book = solve_book_layout(&photos, &solver_config, &canvas, &ga_config).unwrap();

            // Should have created a valid book layout
            assert!(book.page_count() > 0);
            assert_eq!(book.total_photo_count(), 12);
            assert!(!book.is_empty());
        }
    }
}
