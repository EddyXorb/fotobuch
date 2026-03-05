//! Population initialization and management.

use super::types::LayoutIndividual;
use crate::models::{Canvas, FitnessWeights, Photo};
use super::super::page_layout_solver::tree::build::random_tree;
use super::super::page_layout_solver::solver::solve_layout;
use super::super::page_layout_solver::fitness::total_cost;
use rand::Rng;

/// Initializes a random population.
pub fn initialize_population<R: Rng>(
    size: usize,
    num_photos: usize,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    (0..size)
        .map(|_| create_individual(num_photos, photos, canvas, weights, rng))
        .collect()
}

/// Creates a single individual.
fn create_individual<R: Rng>(
    num_photos: usize,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    rng: &mut R,
) -> LayoutIndividual {
    let tree = random_tree(num_photos, rng);
    let layout = solve_layout(&tree, photos, canvas);
    let fitness = total_cost(&layout, photos, canvas, weights);
    LayoutIndividual { tree, layout, fitness }
}

/// Sorts population by fitness (ascending - lower is better).
pub fn sort_by_fitness(population: &mut [LayoutIndividual]) {
    population.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));
}

/// Extracts elite individuals from population.
pub fn extract_elite(
    population: &[LayoutIndividual],
    elitism_ratio: f64,
) -> Vec<LayoutIndividual> {
    let elite_count = (population.len() as f64 * elitism_ratio).ceil() as usize;
    population[..elite_count].to_vec()
}
