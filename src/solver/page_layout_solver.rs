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

use crate::solver::prelude::*;
pub use evolution::LayoutEvolution;
pub use fitness::CostBreakdown;
pub use individual::LayoutIndividual;
use tracing::info;

/// Result of a genetic algorithm run for a single page layout.
#[derive(Debug, Clone)]
pub struct GaResult {
    /// The corresponding page layout with photo placements.
    pub layout: SolverPageLayout,
    /// The raw fitness value (lower is better).
    pub fitness: f64,
    /// Detailed breakdown of fitness cost components.
    pub cost_breakdown: CostBreakdown,
}

/// Entry point for running GA on a single page layout.
pub fn run_ga(photos: &[Photo], canvas: &Canvas, ga_config: &GaConfig) -> GaResult {
    use crate::solver::ga_solver::{Config, GeneticAlgorithm, Individual};

    // Create evaluation context
    let context = evolution::EvaluationContext::new(photos, canvas, &ga_config.weights);

    // Create initial population
    let initial_pop =
        create_initial_population(&context, ga_config.population_size, ga_config.seed);

    // Create GA configuration
    let config = Config {
        population: ga_config.population_size,
        generations: ga_config.max_generations,
        elitism_ratio: ga_config.elite_count as f64 / ga_config.population_size as f64,
        timeout: None, // TODO: Add timeout to dto_models::GaConfig
        no_improvement_limit: ga_config.no_improvement_limit,
        islands: ga_config.islands_nr,
        migration_interval: ga_config.islands_migration_interval,
        migrants: ga_config.islands_nr_migrants,
    };

    // Create evolution dynamics
    let tournament_size = 3; // TODO: Add tournament_size to dto_models::GaConfig
    let evolution = LayoutEvolution::new(
        context,
        tournament_size,
        ga_config.crossover_rate,
        ga_config.mutation_rate,
    );

    // Run GA
    let mut ga = GeneticAlgorithm::new(config, evolution);
    let best = ga.solve(initial_pop).expect("GA returned no solution");

    // Extract results
    let _tree = best.tree().clone();
    let layout = best.layout().clone();
    let fitness = best.fitness();

    // Log cost breakdown
    let cost_breakdown = fitness::cost_breakdown(&layout, photos, canvas, &ga_config.weights);
    info!(
        "Finished layout for one page. Fitness: total={:.4}  size={:.4}  coverage={:.4}  bary={:.4}  order={:.4}",
        cost_breakdown.total,
        cost_breakdown.size,
        cost_breakdown.coverage,
        cost_breakdown.barycenter,
        cost_breakdown.order
    );

    GaResult {
        layout,
        fitness,
        cost_breakdown,
    }
}

/// Creates initial population of random layouts.
fn create_initial_population(
    context: &evolution::EvaluationContext,
    population_size: usize,
    seed: u64,
) -> Vec<LayoutIndividual> {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(seed);

    (0..population_size)
        .map(|_| {
            let tree = tree::create::random_tree(context.photos.len(), &mut rng);
            LayoutIndividual::from_tree(tree, context)
        })
        .collect()
}
