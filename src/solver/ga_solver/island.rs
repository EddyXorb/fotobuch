//! Island model with migration for parallel evolution.

use super::types::LayoutIndividual;
use super::population::{initialize_population, sort_by_fitness, extract_elite};
use super::generation::generate_offspring;
use crate::models::{Canvas, FitnessWeights, GaConfig, IslandConfig, Photo};
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Global best solution shared across islands.
pub type GlobalBest = Arc<Mutex<Option<(LayoutIndividual, f64)>>>;

/// Runs the island model GA with parallel populations.
pub fn run_island_model(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    island_config: &IslandConfig,
    seed: u64,
    start_time: Instant,
) -> LayoutIndividual {
    let num_islands = island_config.islands;
    let global_best = Arc::new(Mutex::new(None));
    
    // Run islands in parallel
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..num_islands)
            .map(|island_id| {
                spawn_island(
                    scope,
                    island_id,
                    photos,
                    canvas,
                    ga_config,
                    island_config,
                    seed,
                    start_time,
                    Arc::clone(&global_best),
                )
            })
            .collect();
        
        // Collect and return best result
        collect_best_result(handles)
    })
}

/// Spawns a single island thread.
#[allow(clippy::too_many_arguments)]
fn spawn_island<'scope>(
    scope: &'scope std::thread::Scope<'scope, '_>,
    island_id: usize,
    photos: &'scope [Photo],
    canvas: &'scope Canvas,
    ga_config: &'scope GaConfig,
    island_config: &'scope IslandConfig,
    seed: u64,
    start_time: Instant,
    global_best: GlobalBest,
) -> std::thread::ScopedJoinHandle<'scope, LayoutIndividual> {
    let photos = photos.to_vec();
    let canvas = *canvas;
    let weights = ga_config.weights;
    let ga_config = ga_config.clone();
    
    scope.spawn(move || {
        use rand::{rngs::StdRng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(island_id as u64));
        
        run_single_island(
            &photos,
            &canvas,
            &weights,
            &ga_config,
            island_config,
            &mut rng,
            start_time,
            global_best,
        )
    })
}

/// Runs evolution on a single island with migration.
#[allow(clippy::too_many_arguments)]
fn run_single_island<R: Rng>(
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    ga_config: &GaConfig,
    island_config: &IslandConfig,
    rng: &mut R,
    start_time: Instant,
    global_best: GlobalBest,
) -> LayoutIndividual {
    let n = photos.len();
    
    // Initialize population
    let mut population = initialize_population(
        ga_config.population,
        n,
        photos,
        canvas,
        weights,
        rng,
    );
    
    let mut local_best: Option<LayoutIndividual> = None;
    
    // Evolution loop
    for generation in 0..ga_config.generations {
        // Check timeout
        if should_stop(ga_config, start_time) {
            break;
        }
        
        // Sort and track best
        sort_by_fitness(&mut population);
        update_local_best(&mut local_best, &population[0]);
        
        // Handle migration
        if generation % island_config.migration_interval == 0 {
            let current_best = population[0].clone();
            handle_migration(
                &current_best,
                &mut population,
                ga_config.elitism_ratio,
                &global_best,
            );
        }
        
        // Extract elite
        let elite = extract_elite(&population, ga_config.elitism_ratio);
        
        // Generate next generation
        population = generate_offspring(
            &population,
            elite,
            photos,
            canvas,
            weights,
            ga_config.tournament_size,
            ga_config.crossover_rate,
            ga_config.mutation_rate,
            ga_config.population,
            rng,
        );
    }
    
    local_best.expect("Should have found at least one solution")
}

/// Updates the local best if the current individual is better.
fn update_local_best(local_best: &mut Option<LayoutIndividual>, current: &LayoutIndividual) {
    let should_update = match local_best {
        None => true,
        Some(best) => current.fitness < best.fitness,
    };
    
    if should_update {
        *local_best = Some(current.clone());
    }
}

/// Handles migration between islands.
fn handle_migration(
    current_best: &LayoutIndividual,
    population: &mut [LayoutIndividual],
    elitism_ratio: f64,
    global_best: &GlobalBest,
) {
    let mut global = global_best.lock().unwrap_or_else(|e| e.into_inner());
    
    // Update global best if we have a better solution
    let should_update = match *global {
        None => true,
        Some((_, global_fitness)) => current_best.fitness < global_fitness,
    };
    
    if should_update {
        *global = Some((current_best.clone(), current_best.fitness));
    } else if let Some((ref global_individual, global_fitness)) = *global {
        // Import global best if it's better than our worst elite
        import_migrant(population, global_individual, global_fitness, elitism_ratio);
    }
}

/// Imports a migrant into the population if it's better than worst elite.
fn import_migrant(
    population: &mut [LayoutIndividual],
    migrant: &LayoutIndividual,
    migrant_fitness: f64,
    elitism_ratio: f64,
) {
    let elite_count = (population.len() as f64 * elitism_ratio).ceil() as usize;
    if elite_count > 0 && migrant_fitness < population[elite_count - 1].fitness {
        population[elite_count - 1] = migrant.clone();
    }
}

/// Collects results from all islands and returns the best.
fn collect_best_result(
    handles: Vec<std::thread::ScopedJoinHandle<'_, LayoutIndividual>>,
) -> LayoutIndividual {
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Island thread panicked"))
        .collect();
    
    results
        .into_iter()
        .min_by(|a, b| a.fitness.total_cmp(&b.fitness))
        .expect("Should have at least one result")
}

/// Checks if evolution should stop due to timeout.
fn should_stop(ga_config: &GaConfig, start_time: Instant) -> bool {
    if let Some(timeout) = ga_config.timeout {
        start_time.elapsed() > timeout
    } else {
        false
    }
}
