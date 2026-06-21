//! Single-page layout optimization using slicing trees.
//!
//! This module contains the core components for single-page layout:
//! - `tree`: Slicing tree data structure
//! - `affine_solver`: Affine layout solver (O(N) with β support)
//! - `fitness`: Fitness function components
//! - `individual`: LayoutIndividual implementing Individual trait
//! - `evolution`: Evolution dynamics for photo layouts

mod evolution;
mod fitness;
mod individual;
mod placer;
mod tree;

use crate::solver::prelude::*;
pub use evolution::LayoutEvolution;
pub use fitness::CostBreakdown;
pub use individual::LayoutIndividual;
use tracing::debug;

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
pub fn solve_page_layout(
    photos: &[Photo],
    canvas: &Canvas,
    config: &PageLayoutSolverConfig,
) -> GaResult {
    use crate::solver::algorithms::genetic_algorithm::{Config, GeneticAlgorithm, Individual};

    let start_time = std::time::Instant::now();

    let context = evolution::EvaluationContext::new(
        photos,
        canvas,
        &config.weights,
        config.enforce_order,
        config.seed,
    );

    let initial_pop = create_initial_population(&context, config.population_size);

    let ga_config = Config {
        population: config.population_size,
        generations: config.max_generations,
        elitism_ratio: config.elite_count as f64 / config.population_size as f64,
        timeout: config.timeout(),
        no_improvement_limit: config.no_improvement_limit,
        islands: config.islands_nr,
        migration_interval: config.islands_migration_interval,
        migrants: config.islands_nr_migrants,
    };

    let evolution = LayoutEvolution::new(
        context,
        config.tournament_size,
        config.crossover_rate,
        config.mutation_rate,
    );

    // Run GA
    let mut ga = GeneticAlgorithm::new(ga_config, evolution);
    let best = ga.solve(initial_pop).expect("GA returned no solution");

    // Extract results
    let _tree = best.tree().clone();
    let layout = best.layout().clone();
    let fitness = best.fitness();

    // Log cost breakdown
    let cost_breakdown = fitness::cost_breakdown(&layout, photos, canvas, &config.weights);
    debug!(
        "Finished layout for one page after {}ms. Fitness: total={:.4}  size={:.4}  coverage={:.4}  bary={:.4}",
        start_time.elapsed().as_millis(),
        cost_breakdown.total,
        cost_breakdown.size,
        cost_breakdown.coverage,
        cost_breakdown.barycenter,
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
) -> Vec<LayoutIndividual> {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    let next_seed = context
        .seed
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut rng = StdRng::seed_from_u64(next_seed);

    (0..population_size)
        .map(|_| {
            let tree =
                tree::create::random_tree(context.photos.len(), &mut rng, context.enforce_order);
            LayoutIndividual::from_tree(tree, context)
        })
        .collect()
}
