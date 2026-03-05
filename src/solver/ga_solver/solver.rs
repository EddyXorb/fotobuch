//! Generic genetic algorithm solver implementation.
//!
//! Provides the main GeneticAlgorithm struct that orchestrates evolution
//! across multiple islands in parallel using the Rayon library.

use super::config::Config;
use super::evolution::{EvolutionDynamic, Island, World};
use super::individual::Individual;
use rayon::prelude::*;
use std::marker::PhantomData;
use tracing::info;

/// Generic genetic algorithm with parallel island model support.
///
/// Coordinates evolution across multiple independent populations (islands)
/// that periodically exchange individuals through migration.
pub struct GeneticAlgorithm<I, E> {
    config: Config,
    evolutor: E,
    _phantom: PhantomData<I>,
}

impl<I, E> GeneticAlgorithm<I, E>
where
    I: Individual + Send + Sync,
    E: EvolutionDynamic<I> + Send + Sync,
{
    /// Creates a new genetic algorithm with the given configuration and evolution strategy.
    pub fn new(config: Config, evolutor: E) -> Self {
        Self {
            config,
            evolutor,
            _phantom: PhantomData,
        }
    }

    /// Runs the genetic algorithm and returns the best individual found.
    pub fn solve(&mut self, initial_population: Vec<I>) -> Option<I> {
        let mut world = self.init_world(initial_population);
        let start_time = std::time::Instant::now();
        let mut last_improvement_gen = 0;
        let mut best_fitness = world
            .global_best()
            .map_or(f64::INFINITY, |ind| ind.fitness());

        for generation in 0..self.config.generations {
            self.evolve_generation(&mut world);
            self.migrate_if_needed(generation, &mut world);

            // Check for significant fitness improvement (> 1% improvement required)
            let current_best_fitness = world
                .global_best()
                .map_or(f64::INFINITY, |ind| ind.fitness());
            const IMPROVEMENT_THRESHOLD: f64 = 0.99;
            if current_best_fitness < best_fitness * IMPROVEMENT_THRESHOLD {
                best_fitness = current_best_fitness;
                last_improvement_gen = generation;
            }

            // Check stopping conditions
            if let Some(reason) = self.should_stop(start_time, generation, last_improvement_gen) {
                info!("{}", reason);
                break;
            }

            info!(
                "Generation {}: Global best fitness = {:.4}",
                generation, current_best_fitness
            );
        }

        world.global_best()
    }

    /// Initializes the world with islands.
    fn init_world(&mut self, initial_population: Vec<I>) -> World<I> {
        let mut islands: Vec<_> = (0..self.config.islands.saturating_sub(1))
            .map(|_| Island::new(initial_population.clone()))
            .collect();
        islands.push(Island::new(initial_population));
        World::new(islands)
    }

    /// Evolves all islands in parallel for one generation.
    fn evolve_generation(&self, world: &mut World<I>) {
        world.islands.par_iter_mut().for_each(|island| {
            island.evolve(&self.evolutor, &self.config);
        });
    }

    /// Migrates individuals between islands if appropriate.
    fn migrate_if_needed(&self, generation: usize, world: &mut World<I>) {
        if generation > 0 && generation.is_multiple_of(self.config.migration_interval) {
            world.migrate(self.config.migrants);
        }
    }

    /// Checks if the algorithm should stop, returning the reason if so.
    fn should_stop(
        &self,
        start_time: std::time::Instant,
        current_generation: usize,
        last_improvement_gen: usize,
    ) -> Option<String> {
        // Check timeout
        if let Some(timeout) = self.config.timeout
            && start_time.elapsed() >= timeout
        {
            return Some("Stopping due to timeout".to_string());
        }

        // Check no improvement limit
        if let Some(limit) = self.config.no_improvement_limit
            && current_generation > 0
            && (current_generation - last_improvement_gen) >= limit
        {
            return Some(format!(
                "Stopping early: no significant improvement for {} generations",
                current_generation - last_improvement_gen
            ));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[derive(Clone, Debug, PartialEq)]
    struct NumberIndividual {
        value: f64,
    }

    impl NumberIndividual {
        fn new(value: f64) -> Self {
            Self { value }
        }
    }

    impl Individual for NumberIndividual {
        type Genome = f64;
        type Phenotype = f64;

        fn genome(&self) -> &Self::Genome {
            &self.value
        }

        fn phenotype(&self) -> &Self::Phenotype {
            &self.value
        }

        fn fitness(&self) -> f64 {
            self.value.abs()
        }
    }

    struct SimpleEvolution {
        mutation_strength: f64,
    }

    impl SimpleEvolution {
        fn new(mutation_strength: f64) -> Self {
            Self { mutation_strength }
        }
    }

    impl EvolutionDynamic<NumberIndividual> for SimpleEvolution {
        fn select(&self, population: &[NumberIndividual]) -> Vec<NumberIndividual> {
            let count = population.len() / 2;
            population.iter().take(count.max(2)).cloned().collect()
        }

        fn crossover(&self, parents: &[NumberIndividual]) -> Vec<NumberIndividual> {
            let mut offspring = Vec::new();
            for i in (0..parents.len().saturating_sub(1)).step_by(2) {
                let child_value = (parents[i].value + parents[i + 1].value) / 2.0;
                offspring.push(NumberIndividual::new(child_value));
                offspring.push(NumberIndividual::new(child_value));
            }
            offspring
        }

        fn mutate(&self, individuals: &mut [NumberIndividual]) {
            for (i, ind) in individuals.iter_mut().enumerate() {
                let perturbation = (i as f64 * 0.1 - 0.5) * self.mutation_strength;
                ind.value += perturbation;
            }
        }
    }

    #[test]
    fn test_genetic_algorithm_solve() {
        let initial_population = vec![
            NumberIndividual::new(100.0),
            NumberIndividual::new(-50.0),
            NumberIndividual::new(75.0),
            NumberIndividual::new(-25.0),
        ];

        let config = Config {
            population: 4,
            generations: 10,
            elitism_ratio: 0.25,
            timeout: None,
            no_improvement_limit: None,
            islands: 2,
            migration_interval: 5,
            migrants: 1,
        };

        let evolutor = SimpleEvolution::new(1.0);
        let mut ga = GeneticAlgorithm::new(config, evolutor);

        let result = ga.solve(initial_population);

        assert!(result.is_some());
        let best = result.unwrap();
        assert!(best.fitness() < 50.0);
    }

    #[test]
    fn test_genetic_algorithm_timeout() {
        let initial_population = vec![NumberIndividual::new(100.0)];

        let config = Config {
            population: 1,
            generations: 1000000,
            elitism_ratio: 0.25,
            timeout: Some(Duration::from_millis(1)),
            no_improvement_limit: None,
            islands: 1,
            migration_interval: 10,
            migrants: 1,
        };

        let evolutor = SimpleEvolution::new(1.0);
        let mut ga = GeneticAlgorithm::new(config, evolutor);

        let start = std::time::Instant::now();
        let _result = ga.solve(initial_population);
        let elapsed = start.elapsed();

        assert!(elapsed < Duration::from_secs(1));
    }
}
