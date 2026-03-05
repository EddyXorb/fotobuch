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

        for generation in 0..self.config.generations {
            self.evolve_generation(&mut world);
            self.migrate_if_needed(generation, &mut world);

            if self.should_stop(start_time) {
                break;
            }
            info!(
                "Generation {}: Global best fitness = {:.4}",
                generation,
                world.global_best().map_or(0.0, |ind| ind.fitness())
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

    /// Checks if the algorithm should stop due to timeout.
    fn should_stop(&self, start_time: std::time::Instant) -> bool {
        if let Some(timeout) = self.config.timeout {
            start_time.elapsed() >= timeout
        } else {
            false
        }
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
