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
pub(super) use fitness::CostBreakdown;
pub(super) use individual::LayoutIndividual;
use tracing::info;

/// Result of a genetic algorithm run for a single page layout.
#[derive(Debug, Clone)]
pub(super) struct GaResult {
    /// The best slicing tree found.
    pub tree: tree::SlicingTree,
    /// The corresponding page layout.
    pub layout: crate::models::PageLayout,
    /// The raw fitness value (lower is better).
    pub fitness: f64,
    /// Detailed breakdown of fitness cost components.
    pub cost_breakdown: CostBreakdown,
}

/// Entry point for running GA on a single page layout.
pub(super) fn run_ga(
    photos: &[crate::models::Photo],
    canvas: &crate::models::Canvas,
    ga_config: &crate::models::GaConfig,
) -> GaResult {
    use crate::solver::ga_solver::{Config, GeneticAlgorithm, Individual};

    // Create evaluation context
    let context = evolution::EvaluationContext::new(photos, canvas, &ga_config.weights);

    // Create initial population
    let initial_pop = create_initial_population(&context, ga_config.population, ga_config.seed);

    // Create GA configuration
    let config = Config {
        population: ga_config.population,
        generations: ga_config.generations,
        elitism_ratio: ga_config.elitism_ratio,
        timeout: ga_config.timeout,
        no_improvement_limit: ga_config.no_improvement_limit,
        islands: ga_config.island_config.islands,
        migration_interval: ga_config.island_config.migration_interval,
        migrants: ga_config.island_config.migrants,
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

    // Log cost breakdown
    let cost_breakdown = fitness::cost_breakdown(&layout, photos, canvas, &ga_config.weights);
    info!(
        "Finished layout for one page. Fitness: total={:.4}  size={:.4}  coverage={:.4}  bary={:.4}  order={:.4}",
        cost_breakdown.total, cost_breakdown.size, cost_breakdown.coverage, cost_breakdown.barycenter, cost_breakdown.order
    );

    GaResult {
        tree,
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
