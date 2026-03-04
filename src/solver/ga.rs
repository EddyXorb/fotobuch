//! Genetic algorithm main loop for photo layout optimization.

use crate::model::{Canvas, FitnessWeights, LayoutResult, Photo};
use super::tree::{random_tree, SlicingTree};
use super::tree::operators::{mutate, crossover};
use super::layout_solver::solve_layout;
use super::fitness::total_cost;
use rand::Rng;

/// Configuration for the genetic algorithm.
#[derive(Debug, Clone)]
pub struct GaConfig {
    /// Population size.
    pub population: usize,
    /// Maximum number of generations.
    pub generations: usize,
    /// Mutation probability.
    pub mutation_rate: f64,
    /// Crossover probability.
    pub crossover_rate: f64,
    /// Tournament selection size.
    pub tournament_size: usize,
    /// Elitism ratio (top % to keep unchanged).
    pub elitism_ratio: f64,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            population: 300,
            generations: 100,
            mutation_rate: 0.2,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.05,
        }
    }
}

/// Individual in the population with its fitness.
#[derive(Clone)]
struct Individual {
    tree: SlicingTree,
    layout: LayoutResult,
    fitness: f64,
}

/// Runs the genetic algorithm to find an optimal layout.
///
/// Returns the best tree, its layout, and its fitness cost.
pub fn run_ga<R: Rng>(
    photos: &[Photo],
    canvas: &Canvas,
    weights: &FitnessWeights,
    config: &GaConfig,
    rng: &mut R,
) -> (SlicingTree, LayoutResult, f64) {
    let n = photos.len();
    
    // Initialize population with random trees
    let mut population: Vec<Individual> = (0..config.population)
        .map(|_| {
            let tree = random_tree(n, rng);
            let layout = solve_layout(&tree, photos, canvas);
            let fitness = total_cost(&layout, photos, canvas, weights);
            Individual { tree, layout, fitness }
        })
        .collect();

    // Evolution loop
    for _generation in 0..config.generations {
        // Sort by fitness (lower is better)
        population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());

        // Elitism: keep top individuals
        let elite_count = (config.population as f64 * config.elitism_ratio).ceil() as usize;
        let mut next_population = population[..elite_count].to_vec();

        // Generate offspring to fill the rest of the population
        while next_population.len() < config.population {
            // Tournament selection for parents
            let parent1 = tournament_select(&population, config.tournament_size, rng);
            let parent2 = tournament_select(&population, config.tournament_size, rng);

            // Apply crossover (if implemented and successful)
            let (mut child1_tree, mut child2_tree) = 
                if rng.gen_range(0.0..1.0) < config.crossover_rate {
                    if let Some((c1, c2)) = crossover(&parent1.tree, &parent2.tree, rng) {
                        (c1, c2)
                    } else {
                        // Crossover not available or failed, use parents
                        (parent1.tree.clone(), parent2.tree.clone())
                    }
                } else {
                    (parent1.tree.clone(), parent2.tree.clone())
                };

            // Apply mutation
            if rng.gen_range(0.0..1.0) < config.mutation_rate {
                mutate(&mut child1_tree, rng);
            }
            if rng.gen_range(0.0..1.0) < config.mutation_rate {
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

            if next_population.len() < config.population {
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
    population.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap());
    let best = &population[0];
    (best.tree.clone(), best.layout.clone(), best.fitness)
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use crate::model::Photo;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population, 300);
        assert_eq!(config.generations, 100);
    }

    #[test]
    fn test_run_ga_simple() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        
        let photos = vec![
            Photo::new(1.5, 1.0, "group1".to_string()),
            Photo::new(1.0, 1.0, "group1".to_string()),
            Photo::new(0.8, 1.0, "group2".to_string()),
        ];
        
        let canvas = Canvas::new(1000.0, 800.0, 5.0, 0.0);
        let weights = FitnessWeights::default();
        
        let config = GaConfig {
            population: 20,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 3,
            elitism_ratio: 0.1,
        };
        
        let (best_tree, best_layout, best_fitness) = run_ga(
            &photos,
            &canvas,
            &weights,
            &config,
            &mut rng,
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
        
        let _photos = vec![Photo::new(1.0, 1.0, "group".to_string())];
        let canvas = Canvas::new(100.0, 100.0, 0.0, 0.0);
        
        // Create a population with different fitness values
        let population = vec![
            Individual {
                tree: random_tree(1, &mut rng),
                layout: LayoutResult::new(vec![], canvas),
                fitness: 10.0,
            },
            Individual {
                tree: random_tree(1, &mut rng),
                layout: LayoutResult::new(vec![], canvas),
                fitness: 5.0,
            },
            Individual {
                tree: random_tree(1, &mut rng),
                layout: LayoutResult::new(vec![], canvas),
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
}
