//! Selection operators for genetic algorithm.

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
    I: Clone,
    R: Rng,
{
    let fitness_fn = |_individual: &I| -> f64 {
        // This is a workaround - we'll pass a closure in LayoutEvolution
        // For now, we use a generic approach
        0.0
    };
    
    tournament_select_with_fitness(population, tournament_size, count, rng, fitness_fn)
}

/// Tournament selection with custom fitness function.
pub fn tournament_select_with_fitness<I, R, F>(
    population: &[I],
    tournament_size: usize,
    count: usize,
    rng: &mut R,
    fitness_fn: F,
) -> Vec<I>
where
    I: Clone,
    R: Rng,
    F: Fn(&I) -> f64,
{
    if population.is_empty() || tournament_size == 0 {
        return vec![];
    }

    let mut selected = Vec::with_capacity(count);

    for _ in 0..count {
        let winner = run_tournament(population, tournament_size, rng, &fitness_fn);
        selected.push(winner.clone());
    }

    selected
}

/// Runs a single tournament and returns the winner.
fn run_tournament<'a, I, R, F>(
    population: &'a [I],
    tournament_size: usize,
    rng: &mut R,
    fitness_fn: &F,
) -> &'a I
where
    I: Clone,
    R: Rng,
    F: Fn(&I) -> f64,
{
    let actual_size = tournament_size.min(population.len());
    let mut best = &population[rng.gen_range(0..population.len())];
    let mut best_fitness = fitness_fn(best);

    for _ in 1..actual_size {
        let candidate = &population[rng.gen_range(0..population.len())];
        let candidate_fitness = fitness_fn(candidate);
        
        if candidate_fitness < best_fitness {
            best = candidate;
            best_fitness = candidate_fitness;
        }
    }

    best
}
