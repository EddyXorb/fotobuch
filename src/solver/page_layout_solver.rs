//! Single-page layout optimization using slicing trees.
//!
//! This module contains the core components for single-page layout:
//! - `tree`: Slicing tree data structure
//! - `affine_solver`: Affine layout solver (O(N) with β support)
//! - `fitness`: Fitness function components
//! - `individual`: LayoutIndividual implementing Individual trait
//! - `evolution`: Evolution dynamics for photo layouts

mod affine_solver;
mod evolution;
mod fitness;
mod individual;
mod tree;

pub(super) use evolution::LayoutEvolution;
pub(super) use individual::LayoutIndividual;

/// Entry point for running GA on a single page layout.
pub(super) fn run_ga(
    photos: &[crate::models::Photo],
    canvas: &crate::models::Canvas,
    ga_config: &crate::models::GaConfig,
    seed: u64,
) -> (tree::SlicingTree, crate::models::PageLayout, f64) {
    use crate::solver::ga_solver::{Config, GeneticAlgorithm, Individual};

    // Create evaluation context
    let context = evolution::EvaluationContext::new(photos, canvas, &ga_config.weights);

    // Create initial population
    let initial_pop = create_initial_population(&context, ga_config.population, seed);

    // Create GA configuration
    let config = Config {
        population: ga_config.population,
        generations: ga_config.generations,
        elitism_ratio: ga_config.elitism_ratio,
        timeout: ga_config.timeout,
        islands: ga_config
            .island_config
            .as_ref()
            .map(|c| c.islands)
            .unwrap_or(1),
        migration_interval: ga_config
            .island_config
            .as_ref()
            .map(|c| c.migration_interval)
            .unwrap_or(10),
        migrants: ga_config
            .island_config
            .as_ref()
            .map(|c| c.migrants)
            .unwrap_or(1),
    };

    // Create evolution dynamics
    let evolution = LayoutEvolution::new(
        context,
        ga_config.tournament_size,
        ga_config.crossover_rate,
        ga_config.mutation_rate,
    );

    // Run GA
    let mut ga = GeneticAlgorithm::new(config, evolution);
    let best = ga.solve(initial_pop).expect("GA returned no solution");

    // Extract results
    let tree = best.tree().clone();
    let layout = best.layout().clone();
    let fitness = best.fitness();

    (tree, layout, fitness)
}

/// Creates initial population of random layouts.
fn create_initial_population(
    context: &evolution::EvaluationContext,
    population_size: usize,
    seed: u64,
) -> Vec<LayoutIndividual> {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(seed);

    (0..population_size)
        .map(|_| {
            let tree = tree::create::random_tree(context.photos.len(), &mut rng);
            LayoutIndividual::from_tree(tree, context)
        })
        .collect()
}
