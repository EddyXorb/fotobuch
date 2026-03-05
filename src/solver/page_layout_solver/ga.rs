//! Genetic algorithm main loop for photo layout optimization.

use crate::models::{Canvas, FitnessWeights, IslandConfig, PageLayout, Photo, GaConfig};
use super::tree::SlicingTree;
use super::tree::build::random_tree;
use super::tree::mutate::mutate;
use super::tree::crossover::crossover;
use super::layout_solver::solve_layout;
use super::fitness::total_cost;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Individual in the population with its fitness.
#[derive(Clone)]
struct Individual {
    tree: SlicingTree,
    layout: PageLayout,
    fitness: f64,
}

/// Runs the genetic algorithm to find an optimal layout.
///
/// Automatically switches between single-population GA and island model
/// based on whether `ga_config.island_config` is Some or None.
///
/// Returns the best tree, its layout, and its fitness cost.
pub fn run_ga(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    seed: u64,
) -> (SlicingTree, PageLayout, f64) {
    let start_time = Instant::now();
    
    // Decide between single-population and island model
    match &ga_config.island_config {
        None => {
            // Single-population GA
            use rand::{rngs::StdRng, SeedableRng};
            let mut rng = StdRng::seed_from_u64(seed);
            run_single_population_ga(
                photos,
                canvas,
                ga_config,
                &mut rng,
                start_time,
            )
        }
        Some(island_config) => {
            // Island model GA with multiple parallel populations
            run_island_model_ga(
                photos,
                canvas,
                ga_config,
                island_config,
                seed,
                start_time,
            )
        }
    }
}

/// Tournament selection: pick N random individuals and return the best.
fn tournament_select<'a, R: Rng>(
    population: &'a [Individual],
    tournament_size: usize,
    rng: &mut R,
) -> &'a Individual {
    let mut best = &population[rng.gen_range(0..population.len())];
    
    for _ in 1..tournament_size {
        let candidate = &population[rng.gen_range(0..population.len())];
        if candidate.fitness < best.fitness {
            best = candidate;
        }
    }
    
    best
}

/// Runs a single-population GA (no islands).
fn run_single_population_ga<R: Rng>(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    rng: &mut R,
    start_time: Instant,
) -> (SlicingTree, PageLayout, f64) {
    let weights = &ga_config.weights;
    let n = photos.len();
    
    // Initialize population with random trees
    let mut population: Vec<Individual> = (0..ga_config.population)
        .map(|_| {
            let tree = random_tree(n, rng);
            let layout = solve_layout(&tree, photos, canvas);
            let fitness = total_cost(&layout, photos, canvas, weights);
            Individual { tree, layout, fitness }
        })
        .collect();

    // Evolution loop
    for _generation in 0..ga_config.generations {
        // Check timeout
        if let Some(timeout) = ga_config.timeout
            && start_time.elapsed() > timeout {
                break;
            }

        // Sort by fitness (lower is better)
        population.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));

        // Elitism: keep top individuals
        let elite_count = (ga_config.population as f64 * ga_config.elitism_ratio).ceil() as usize;
        let mut next_population = population[..elite_count].to_vec();

        // Generate offspring to fill the rest of the population
        while next_population.len() < ga_config.population {
            // Tournament selection for parents
            let parent1 = tournament_select(&population, ga_config.tournament_size, rng);
            let parent2 = tournament_select(&population, ga_config.tournament_size, rng);

            // Apply crossover
            let (mut child1_tree, mut child2_tree) = 
                if rng.gen_range(0.0..1.0) < ga_config.crossover_rate {
                    if let Some((c1, c2)) = crossover(&parent1.tree, &parent2.tree, rng) {
                        (c1, c2)
                    } else {
                        (parent1.tree.clone(), parent2.tree.clone())
                    }
                } else {
                    (parent1.tree.clone(), parent2.tree.clone())
                };

            // Apply mutation
            if rng.gen_range(0.0..1.0) < ga_config.mutation_rate {
                mutate(&mut child1_tree, rng);
            }
            if rng.gen_range(0.0..1.0) < ga_config.mutation_rate {
                mutate(&mut child2_tree, rng);
            }

            // Evaluate children
            let layout1 = solve_layout(&child1_tree, photos, canvas);
            let fitness1 = total_cost(&layout1, photos, canvas, weights);
            next_population.push(Individual {
                tree: child1_tree,
                layout: layout1,
                fitness: fitness1,
            });

            if next_population.len() < ga_config.population {
                let layout2 = solve_layout(&child2_tree, photos, canvas);
                let fitness2 = total_cost(&layout2, photos, canvas, weights);
                next_population.push(Individual {
                    tree: child2_tree,
                    layout: layout2,
                    fitness: fitness2,
                });
            }
        }

        population = next_population;
    }

    // Return best individual
    population.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));
    let best = &population[0];
    (best.tree.clone(), best.layout.clone(), best.fitness)
}

/// Runs the island model GA: multiple independent populations with periodic migration.
///
/// Each island runs on its own thread. The best individuals migrate between islands
/// every `migration_interval` generations. Returns the globally best solution found.
fn run_island_model_ga(
    photos: &[Photo],
    canvas: &Canvas,
    ga_config: &GaConfig,
    island_config: &IslandConfig,
    seed: u64,
    start_time: Instant,
) -> (SlicingTree, PageLayout, f64) {
    let weights = &ga_config.weights;
    let num_islands = island_config.islands;
    
    // Shared best solution across all islands
    let global_best = Arc::new(Mutex::new(None::<(SlicingTree, PageLayout, f64)>));
    
    // Run each island in a scoped thread
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..num_islands)
            .map(|island_id| {
                let global_best = Arc::clone(&global_best);
                let photos = photos.to_vec();
                let canvas = *canvas;
                let weights = *weights;
                let ga_config = ga_config.clone();
                
                scope.spawn(move || {
                    // Each island gets its own RNG seeded differently
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
            })
            .collect();
        
        // Collect results from all islands
        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Island thread panicked"))
            .collect();
        
        // Return the best result across all islands
        results
            .into_iter()
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .expect("Should have at least one island result")
    })
}

/// Runs a single island GA with periodic global best updates.
#[allow(clippy::too_many_arguments)]
fn run_single_island<R: Rng>(
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    ga_config: &GaConfig,
    island_config: &IslandConfig,
    rng: &mut R,
    start_time: Instant,
    global_best: Arc<Mutex<Option<(SlicingTree, PageLayout, f64)>>>,
) -> (SlicingTree, PageLayout, f64) {
    let n = photos.len();
    
    // Initialize population with random trees
    let mut population: Vec<Individual> = (0..ga_config.population)
        .map(|_| {
            let tree = random_tree(n, rng);
            let layout = solve_layout(&tree, photos, canvas);
            let fitness = total_cost(&layout, photos, canvas, weights);
            Individual { tree, layout, fitness }
        })
        .collect();

    let mut local_best = None::<(SlicingTree, PageLayout, f64)>;

    // Evolution loop
    for generation in 0..ga_config.generations {
        // Check timeout
        if let Some(timeout) = ga_config.timeout
            && start_time.elapsed() > timeout {
                break;
            }

        // Sort by fitness (lower is better)
        population.sort_by(|a, b| a.fitness.total_cmp(&b.fitness));

        // Track local best
        let current_best = &population[0];
        if local_best.is_none() || current_best.fitness < local_best.as_ref().unwrap().2 {
            local_best = Some((
                current_best.tree.clone(),
                current_best.layout.clone(),
                current_best.fitness,
            ));
        }

        // Periodic migration: check global best and potentially import better solution
        if generation % island_config.migration_interval == 0 {
            // Update global best if we have a better solution
            {
                let mut global = global_best.lock().unwrap();
                if global.is_none() || current_best.fitness < global.as_ref().unwrap().2 {
                    *global = Some((
                        current_best.tree.clone(),
                        current_best.layout.clone(),
                        current_best.fitness,
                    ));
                } else if let Some(ref global_solution) = *global {
                    // Import global best if it's better than our worst elite
                    let elite_count = (ga_config.population as f64 * ga_config.elitism_ratio).ceil() as usize;
                    if elite_count > 0 && global_solution.2 < population[elite_count - 1].fitness {
                        // Replace worst elite with global best
                        population[elite_count - 1] = Individual {
                            tree: global_solution.0.clone(),
                            layout: global_solution.1.clone(),
                            fitness: global_solution.2,
                        };
                    }
                }
            }
        }

        // Elitism: keep top individuals
        let elite_count = (ga_config.population as f64 * ga_config.elitism_ratio).ceil() as usize;
        let mut next_population = population[..elite_count].to_vec();

        // Generate offspring to fill the rest of the population
        while next_population.len() < ga_config.population {
            // Tournament selection for parents
            let parent1 = tournament_select(&population, ga_config.tournament_size, rng);
            let parent2 = tournament_select(&population, ga_config.tournament_size, rng);

            // Apply crossover
            let (mut child1_tree, mut child2_tree) = 
                if rng.gen_range(0.0..1.0) < ga_config.crossover_rate {
                    if let Some((c1, c2)) = crossover(&parent1.tree, &parent2.tree, rng) {
                        (c1, c2)
                    } else {
                        (parent1.tree.clone(), parent2.tree.clone())
                    }
                } else {
                    (parent1.tree.clone(), parent2.tree.clone())
                };

            // Apply mutation
            if rng.gen_range(0.0..1.0) < ga_config.mutation_rate {
                mutate(&mut child1_tree, rng);
            }
            if rng.gen_range(0.0..1.0) < ga_config.mutation_rate {
                mutate(&mut child2_tree, rng);
            }

            // Evaluate children
            let layout1 = solve_layout(&child1_tree, photos, canvas);
            let fitness1 = total_cost(&layout1, photos, canvas, weights);
            next_population.push(Individual {
                tree: child1_tree,
                layout: layout1,
                fitness: fitness1,
            });

            if next_population.len() < ga_config.population {
                let layout2 = solve_layout(&child2_tree, photos, canvas);
                let fitness2 = total_cost(&layout2, photos, canvas, weights);
                next_population.push(Individual {
                    tree: child2_tree,
                    layout: layout2,
                    fitness: fitness2,
                });
            }
        }

        population = next_population;
    }

    // Return local best
    local_best.unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{IslandConfig, Photo};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::time::Duration;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population, 300);
        assert_eq!(config.generations, 100);
        assert!(config.island_config.is_some());
    }

    #[test]
    fn test_run_ga_simple() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: None,
            island_config: None,
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &config,
            42,
        );
        
        // Check that we got a valid result
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_tournament_select() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        
        let _photos = [Photo::new(1.0, 1.0, "group".to_string())];
        let canvas = Canvas::new(100.0, 100.0, 0.0, 0.0);
        
        // Create a population with different fitness values
        let population = vec![
            Individual {
                tree: random_tree(1, &mut rng),
                layout: PageLayout::new(vec![], canvas),
                fitness: 10.0,
            },
            Individual {
                tree: random_tree(1, &mut rng),
                layout: PageLayout::new(vec![], canvas),
                fitness: 5.0,
            },
            Individual {
                tree: random_tree(1, &mut rng),
                layout: PageLayout::new(vec![], canvas),
                fitness: 20.0,
            },
        ];
        
        // Tournament selection should prefer lower fitness
        let mut best_fitness_count = 0;
        for _ in 0..100 {
            let selected = tournament_select(&population, 3, &mut rng);
            if selected.fitness == 5.0 {
                best_fitness_count += 1;
            }
        }
        
        // Best individual should be selected most of the time
        assert!(best_fitness_count > 50, "Tournament should prefer better fitness");
    }

    #[test]
    fn test_island_config_default() {
        let config = IslandConfig::default();
        assert!(config.islands > 0);
        assert_eq!(config.migration_interval, 5);
        assert_eq!(config.migrants, 2);
    }

    #[test]
    fn test_run_ga_single_island() {
        // With 1 island, should behave similar to regular GA
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_secs(5)),
            island_config: Some(IslandConfig {
                islands: 1,
                migration_interval: 2,
                migrants: 1,
            }),
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            42,
        );
        
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_run_ga_multiple_islands() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
            Photo::new(1.2, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 30,
            generations: 10,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_secs(10)),
            island_config: Some(IslandConfig {
                islands: 4,
                migration_interval: 3,
                migrants: 2,
            }),
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            999,
        );
        
        assert_eq!(best_tree.len(), 2 * photos.len() - 1);
        assert_eq!(best_layout.placements.len(), photos.len());
        assert!(best_fitness.is_finite());
        assert!(best_fitness >= 0.0);
    }

    #[test]
    fn test_ga_timeout() {
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        
        let ga_config = GaConfig {
            population: 20,
            generations: 1000, // Many generations
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_millis(100)), // Short timeout
            island_config: Some(IslandConfig {
                islands: 2,
                migration_interval: 2,
                migrants: 1,
            }),
        };
        
        let start = Instant::now();
        let (_tree, _layout, _fitness) = run_ga(
            &photos,
            &canvas,
            &ga_config,
            42,
        );
        let elapsed = start.elapsed();
        
        // Should respect timeout (with some margin for thread overhead)
        assert!(elapsed < Duration::from_millis(500), "Timeout not respected");
    }
}
