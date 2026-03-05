//! Core generation step for genetic algorithm.

use super::types::{GenerationParams, LayoutIndividual};
use super::selection::select_parents;
use super::operators::{apply_crossover, apply_mutation};
use super::evaluation::evaluate_offspring;
use rand::Rng;

/// Executes one generation: selection, crossover, mutation, evaluation.
///
/// Generates offspring from the current population using elitism,
/// tournament selection, crossover, and mutation.
pub fn generate_offspring<R: Rng>(
    population: &[LayoutIndividual],
    elite: Vec<LayoutIndividual>,
    params: &GenerationParams,
    target_size: usize,
    rng: &mut R,
) -> Vec<LayoutIndividual> {
    let mut next_population = elite;
    
    while next_population.len() < target_size {
        // Select parents
        let (parent1, parent2) = select_parents(population, params.tournament_size, rng);
        
        // Apply crossover
        let (mut child1, mut child2) = apply_crossover(
            &parent1.tree,
            &parent2.tree,
            params.crossover_rate,
            rng,
        );
        
        // Apply mutation
        apply_mutation(&mut child1, params.mutation_rate, rng);
        apply_mutation(&mut child2, params.mutation_rate, rng);
        
        // Evaluate offspring
        evaluate_offspring(
            child1,
            child2,
            params.photos,
            params.canvas,
            params.weights,
            &mut next_population,
            target_size,
        );
    }
    
    next_population
}
