//! Book layout solver that distributes photos across multiple pages.
//!
//! This module will eventually implement logic to:
//! - Group photos by lexicographic ordering of groups
//! - Sort photos within groups by timestamp
//! - Distribute photos across pages optimally
//! - Apply page layout solver to each page

mod feasibility;
mod model;

use super::page_layout_solver::run_ga;
use crate::models::{BookLayout, Canvas, GaConfig, Photo};

/// Solves the book layout problem by distributing photos across pages.
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
