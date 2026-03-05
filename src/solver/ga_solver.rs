//! Generic genetic algorithm solver for photobook layout optimization.
//!
//! This module provides a trait-based, modular genetic algorithm implementation
//! that can run in single-population mode or parallel island model mode.

mod traits;
mod types;
mod population;
mod selection;
mod operators;
mod evaluation;
mod generation;
mod evolution;
mod island;
mod solver;

// Public API
pub use types::LayoutIndividual;

use crate::models::{Canvas, GaConfig, Photo};
use super::page_layout_solver::tree::SlicingTree;
use crate::models::PageLayout;
use std::time::Instant;

/// Runs the genetic algorithm to find an optimal layout.
///
/// Automatically switches between single-population GA and island model
/// based on whether `ga_config.island_config` is Some or None.
///
/// Returns the best tree, layout, and fitness found.
pub fn run_ga(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    seed: u64,
) -> (SlicingTree, PageLayout, f64) {
    let start_time = Instant::now();
    
    let best = match &ga_config.island_config {
        None => run_single_population_mode(photos, canvas, ga_config, seed, start_time),
        Some(island_config) => run_island_mode(photos, canvas, ga_config, island_config, seed, start_time),
    };
    
    (best.tree, best.layout, best.fitness)
}

/// Runs single-population mode.
fn run_single_population_mode(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    seed: u64,
    start_time: Instant,
) -> LayoutIndividual {
    use rand::{rngs::StdRng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    evolution::run_single_population(photos, canvas, ga_config, &mut rng, start_time)
}

/// Runs island mode.
fn run_island_mode(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    island_config: &crate::models::IslandConfig,
    seed: u64,
    start_time: Instant,
) -> LayoutIndividual {
    island::run_island_model(photos, canvas, ga_config, island_config, seed, start_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FitnessWeights, IslandConfig, Photo};
    use std::time::Duration;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population, 300);
        assert_eq!(config.generations, 100);
        assert!(config.island_config.is_some());
    }

    #[test]
    fn test_run_ga_simple() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: None,
            island_config: None,
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &config,
            42,
        );
        
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_tournament_select() {
        use rand_chacha::ChaCha8Rng;
        use rand::SeedableRng;
        use super::selection::tournament_select;
        use super::population::initialize_population;
        
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let photos = vec![Photo::new(1.0, 1.0, "group".to_string())];
        let canvas = Canvas::new(100.0, 100.0, 0.0, 0.0);
        let weights = FitnessWeights::default();
        
        let mut population = initialize_population(3, 1, &photos, &canvas, &weights, &mut rng);
        
        // Manually set different fitness values
        population[0].fitness = 10.0;
        population[1].fitness = 5.0;
        population[2].fitness = 20.0;
        
        // Tournament selection should prefer lower fitness
        let mut best_fitness_count = 0;
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        for _ in 0..100 {
            let selected = tournament_select(&population, 3, &mut rng);
            if selected.fitness == 5.0 {
                best_fitness_count += 1;
            }
        }
        
        assert!(best_fitness_count > 50, "Tournament should prefer better fitness");
    }

    #[test]
    fn test_island_config_default() {
        let config = IslandConfig::default();
        assert!(config.islands > 0);
        assert_eq!(config.migration_interval, 5);
        assert_eq!(config.migrants, 2);
    }

    #[test]
    fn test_run_ga_single_island() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_secs(5)),
            island_config: Some(IslandConfig {
                islands: 1,
                migration_interval: 2,
                migrants: 1,
            }),
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            42,
        );
        
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_run_ga_multiple_islands() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
            Photo::new(1.2, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 30,
            generations: 10,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_secs(10)),
            island_config: Some(IslandConfig {
                islands: 4,
                migration_interval: 3,
                migrants: 2,
            }),
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            999,
        );
        
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_ga_timeout() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 20,
            generations: 1000,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_millis(100)),
            island_config: Some(IslandConfig {
                islands: 2,
                migration_interval: 2,
                migrants: 1,
            }),
        };
        
        let start = Instant::now();
        let (_tree, _layout, _fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            42,
        );
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(500), "Timeout not respected");
    }
}
