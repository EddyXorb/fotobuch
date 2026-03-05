//! Core generation step for genetic algorithm.

use super::types::LayoutIndividual;
use super::selection::select_parents;
use super::operators::{apply_crossover, apply_mutation};
use super::evaluation::evaluate_offspring;
use crate::models::{Canvas, FitnessWeights, Photo};
use rand::Rng;

/// Executes one generation: selection, crossover, mutation, evaluation.
///
/// Generates offspring from the current population using elitism,
/// tournament selection, crossover, and mutation.
#[allow(clippy::too_many_arguments)]
pub fn generate_offspring<R: Rng>(
    population: &[LayoutIndividual],
    elite: Vec<LayoutIndividual>,
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    tournament_size: usize,
    crossover_rate: f64,
    mutation_rate: f64,
    target_size: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    let mut next_population = elite;
    
    while next_population.len() < target_size {
        // Select parents
        let (parent1, parent2) = select_parents(population, tournament_size, rng);
        
        // Apply crossover
        let (mut child1, mut child2) = apply_crossover(
            &parent1.tree,
            &parent2.tree,
            crossover_rate,
            rng,
        );
        
        // Apply mutation
        apply_mutation(&mut child1, mutation_rate, rng);
        apply_mutation(&mut child2, mutation_rate, rng);
        
        // Evaluate offspring
        evaluate_offspring(
            child1,
            child2,
            photos,
            canvas,
            weights,
            &mut next_population,
            target_size,
        );
    }
    
    next_population
}
