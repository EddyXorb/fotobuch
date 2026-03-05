//! Selection operators for genetic algorithms.

use super::types::LayoutIndividual;
use rand::Rng;

/// Performs tournament selection on a population.
///
/// Randomly selects `tournament_size` individuals and returns the best one.
pub fn tournament_select<'a, R: Rng>(
    population: &'a [LayoutIndividual],
    tournament_size: usize,
    rng: &mut R,
) -> &'a LayoutIndividual {
    let mut best = &population[rng.gen_range(0..population.len())];
    
    for _ in 1..tournament_size {
        let candidate = &population[rng.gen_range(0..population.len())];
        if candidate.fitness < best.fitness {
            best = candidate;
        }
    }
    
    best
}

/// Selects two parents using tournament selection.
pub fn select_parents<'a, R: Rng>(
    population: &'a [LayoutIndividual],
    tournament_size: usize,
    rng: &mut R,
) -> (&'a LayoutIndividual, &'a LayoutIndividual) {
    let parent1 = tournament_select(population, tournament_size, rng);
    let parent2 = tournament_select(population, tournament_size, rng);
    (parent1, parent2)
}
