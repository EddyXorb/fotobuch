//! Single-population evolution loop.

use super::types::{GenerationParams, LayoutIndividual};
use super::population::{initialize_population, sort_by_fitness, extract_elite};
use super::generation::generate_offspring;
use crate::models::{Canvas, GaConfig, Photo};
use rand::Rng;
use std::time::Instant;

/// Runs a single-population genetic algorithm.
///
/// Returns the best tree, layout, and fitness found.
pub fn run_single_population<R: Rng>(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    rng: &mut R,
    start_time: Instant,
) -> LayoutIndividual {
    let weights = &ga_config.weights;
    let n = photos.len();
    
    // Initialize population
    let mut population = initialize_population(
        ga_config.population,
        n,
        photos,
        canvas,
        weights,
        rng,
    );
    
    // Create generation parameters
    let gen_params = GenerationParams::new(
        photos,
        canvas,
        weights,
        ga_config.tournament_size,
        ga_config.crossover_rate,
        ga_config.mutation_rate,
    );
    
    // Evolution loop
    for _generation in 0..ga_config.generations {
        // Check timeout
        if should_stop(ga_config, start_time) {
            break;
        }
        
        // Sort population
        sort_by_fitness(&mut population);
        
        // Extract elite
        let elite = extract_elite(&population, ga_config.elitism_ratio);
        
        // Generate next generation
        population = generate_offspring(
            &population,
            elite,
            &gen_params,
            ga_config.population,
            rng,
        );
    }
    
    // Return best individual
    sort_by_fitness(&mut population);
    population.into_iter().next().expect("Population should not be empty")
}

/// Checks if evolution should stop due to timeout.
fn should_stop(ga_config: &GaConfig, start_time: Instant) -> bool {
    if let Some(timeout) = ga_config.timeout {
        start_time.elapsed() > timeout
    } else {
        false
    }
}
