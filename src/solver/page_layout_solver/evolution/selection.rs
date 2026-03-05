//! Selection operators for genetic algorithm.

use crate::solver::ga_solver::Individual;
use rand::Rng;

/// Performs tournament selection on a population.
///
/// Randomly selects `tournament_size` individuals, picks the best (lowest fitness),
/// and repeats `count` times to build the selected population.
pub fn tournament_select<I, R>(
    population: &[I],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
) -> Vec<I>
where
    I: Individual,
    R: Rng,
{
    if population.is_empty() || tournament_size == 0 {
        return vec![];
    }

    let mut selected = Vec::with_capacity(count);

    for _ in 0..count {
        let winner = run_tournament(population, tournament_size, rng);
        selected.push(winner.clone());
    }

    selected
}

/// Runs a single tournament and returns the winner.
fn run_tournament<'a, I, R>(
    population: &'a [I],
    tournament_size: usize,
    rng: &mut R,
) -> &'a I
where
    I: Individual,
    R: Rng,
{
    let actual_size = tournament_size.min(population.len());
    let mut best = &population[rng.gen_range(0..population.len())];
    let mut best_fitness = best.fitness();

    for _ in 1..actual_size {
        let candidate = &population[rng.gen_range(0..population.len())];
        let candidate_fitness = candidate.fitness();
        
        if candidate_fitness < best_fitness {
            best = candidate;
            best_fitness = candidate_fitness;
        }
    }

    best
}
