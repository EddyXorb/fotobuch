use rayon::prelude::*;
use std::marker::PhantomData;
use std::time::Duration;

pub struct Config {
    /// Population size per island.
    pub population: usize,

    /// Maximum number of generations.
    pub generations: usize,

    /// Mutation probability (0.0 to 1.0).
    pub mutation_rate: f64,

    /// Crossover probability (0.0 to 1.0).
    pub crossover_rate: f64,

    /// Tournament selection size.
    pub tournament_size: usize,

    /// Elitism ratio - proportion of best individuals to keep unchanged (0.0 to 1.0).
    pub elitism_ratio: f64,

    /// Optional timeout for the entire optimization run.
    pub timeout: Option<Duration>,

    /// Number of independent islands (populations).
    /// Defaults to number of available CPU cores.
    pub islands: usize,

    /// Generations between migrations.
    pub migration_interval: usize,

    /// Number of individuals to migrate per island per migration event.
    pub migrants: usize,
}

trait Individual: Clone {
    type Genome;
    type Phenotype;

    fn genome(&self) -> &Self::Genome;
    fn phenotype(&self) -> &Self::Phenotype;
    fn fitness(&self) -> f64;
}

/// Evolution dynamics bundling selection, crossover, and mutation strategies.
///
/// Implementations define how populations evolve through genetic operations.
trait EvolutionDynamic<I: Individual> {
    /// Selects individuals from the population for reproduction.
    fn select(&self, population: &[I]) -> Vec<I>;

    /// Performs crossover on selected parents to produce offspring.
    fn crossover(&self, parents: &[I]) -> Vec<I>;

    /// Mutates individuals in-place.
    fn mutate(&self, individuals: &mut [I]);
}

struct Island<I: Individual> {
    population: Vec<I>,
}

impl<I> Island<I>
where
    I: Individual,
{
    fn new(population: Vec<I>) -> Self {
        let mut population = population;
        population.sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
        Self { population }
    }

    /// Evolves the island's population for one generation using the provided operators and configuration.
    fn evolve<E>(&mut self, evolutor: &E, config: &Config)
    where
        E: EvolutionDynamic<I>,
    {
        // is alredy sorted from constructor
        // Keep elite individuals
        let elite_count = (self.population.len() as f64 * config.elitism_ratio) as usize;
        let elite = self.population[..elite_count].to_vec();

        // Select individuals for reproduction
        let selected = evolutor.select(&self.population);

        // Generate offspring through crossover
        let mut offspring = evolutor.crossover(&selected);

        // Apply mutation
        evolutor.mutate(&mut offspring);

        // Create new population: elite + offspring
        let mut next_population = elite;
        next_population.extend(offspring);

        // Truncate to population size if needed
        next_population.truncate(config.population);

        self.population = next_population;

        // Sort population by fitness (best first)
        self.population
            .sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
    }

    fn best(&self) -> Option<&I> {
        self.population.first()
    }
}

struct World<I: Individual> {
    islands: Vec<Island<I>>,
}

impl<I: Individual> World<I> {
    fn new(islands: Vec<Island<I>>) -> Self {
        Self { islands }
    }

    fn global_best(&self) -> Option<I> {
        self.islands
            .iter()
            .filter_map(|island| island.best())
            .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
            .cloned()
    }

    fn migrate(&mut self, migrants_per_island: usize) {
        if self.islands.len() < 2 {
            return;
        }

        // Collect best individuals from each island
        let migrants: Vec<_> = self
            .islands
            .iter()
            .filter_map(|island| island.best().cloned())
            .collect();

        // Distribute migrants to other islands
        for (i, island) in self.islands.iter_mut().enumerate() {
            for (j, migrant) in migrants.iter().enumerate() {
                if i != j && island.population.len() > migrants_per_island {
                    // Replace worst individuals with migrants
                    let replace_idx = island.population.len() - 1;
                    island.population[replace_idx] = migrant.clone();
                }
            }
        }
    }
}

struct GeneticAlgorithm<I, E> {
    config: Config,
    evolutor: E,
    _phantom: PhantomData<I>,
}

impl<I, E> GeneticAlgorithm<I, E>
where
    I: Individual + Send,
    E: EvolutionDynamic<I> + Send + Sync,
{
    pub fn new(config: Config, evolutor: E) -> Self {
        Self {
            config,
            evolutor,
            _phantom: PhantomData,
        }
    }

    pub fn solve(&mut self, initial_population: Vec<I>) -> Option<I> {
        let mut world = self.create_world(initial_population);
        let start_time = std::time::Instant::now();

        // Main evolution loop
        for generation in 0..self.config.generations {
            // Evolve each island in parallel
            world.islands.par_iter_mut().for_each(|island| {
                island.evolve(&self.evolutor, &self.config);
            });

            // Periodic migration between islands
            if generation > 0 && generation % self.config.migration_interval == 0 {
                world.migrate(self.config.migrants);
            }

            // Check for timeout
            if let Some(timeout) = self.config.timeout
                && start_time.elapsed() >= timeout {
                    break;
                }
        }

        // Return the globally best individual
        world.global_best()
    }

    fn create_world(&mut self, initial_population: Vec<I>) -> World<I> {
        //move the last one to the end to ensure it is not cloned unnecessarily
        let mut islands: Vec<_> = (0..self.config.islands.saturating_sub(1))
            .map(|_| Island::new(initial_population.clone()))
            .collect();
        islands.push(Island::new(initial_population));

        World::new(islands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple test individual: optimizes a single f64 value towards zero.
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
            // Fitness is absolute value (lower is better, optimum is 0.0)
            self.value.abs()
        }
    }

    /// Simple evolution strategy: selection, crossover, and mutation for NumberIndividual.
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
            // Select best half
            let count = population.len() / 2;
            population.iter().take(count.max(2)).cloned().collect()
        }

        fn crossover(&self, parents: &[NumberIndividual]) -> Vec<NumberIndividual> {
            // Average pairs of parents
            let mut offspring = Vec::new();
            for i in (0..parents.len().saturating_sub(1)).step_by(2) {
                let child_value = (parents[i].value + parents[i + 1].value) / 2.0;
                offspring.push(NumberIndividual::new(child_value));
                offspring.push(NumberIndividual::new(child_value));
            }
            offspring
        }

        fn mutate(&self, individuals: &mut [NumberIndividual]) {
            // Add small random perturbation (deterministic for testing)
            for (i, ind) in individuals.iter_mut().enumerate() {
                let perturbation = (i as f64 * 0.1 - 0.5) * self.mutation_strength;
                ind.value += perturbation;
            }
        }
    }

    #[test]
    fn test_individual_fitness() {
        let ind1 = NumberIndividual::new(5.0);
        let ind2 = NumberIndividual::new(-3.0);
        let ind3 = NumberIndividual::new(0.0);

        assert_eq!(ind1.fitness(), 5.0);
        assert_eq!(ind2.fitness(), 3.0);
        assert_eq!(ind3.fitness(), 0.0);
    }

    #[test]
    fn test_island_creation_sorts_population() {
        let population = vec![
            NumberIndividual::new(5.0),
            NumberIndividual::new(1.0),
            NumberIndividual::new(10.0),
            NumberIndividual::new(2.0),
        ];

        let island = Island::new(population);

        assert_eq!(island.population[0].fitness(), 1.0);
        assert_eq!(island.population[1].fitness(), 2.0);
        assert_eq!(island.population[2].fitness(), 5.0);
        assert_eq!(island.population[3].fitness(), 10.0);
    }

    #[test]
    fn test_island_best() {
        let population = vec![
            NumberIndividual::new(5.0),
            NumberIndividual::new(1.0),
            NumberIndividual::new(10.0),
        ];

        let island = Island::new(population);
        let best = island.best();

        assert!(best.is_some());
        assert_eq!(best.unwrap().fitness(), 1.0);
    }

    #[test]
    fn test_island_evolve() {
        let population = vec![
            NumberIndividual::new(10.0),
            NumberIndividual::new(20.0),
            NumberIndividual::new(30.0),
            NumberIndividual::new(40.0),
        ];

        let mut island = Island::new(population);
        let evolutor = SimpleEvolution::new(0.1);
        let config = Config {
            population: 4,
            generations: 1,
            mutation_rate: 0.0,
            crossover_rate: 0.0,
            tournament_size: 2,
            elitism_ratio: 0.25,
            timeout: None,
            islands: 1,
            migration_interval: 10,
            migrants: 1,
        };

        let initial_best = island.best().unwrap().fitness();
        island.evolve(&evolutor, &config);
        let evolved_best = island.best().unwrap().fitness();

        // After evolution, best fitness should improve or stay same
        assert!(evolved_best <= initial_best);
    }

    #[test]
    fn test_world_global_best() {
        let island1 = Island::new(vec![
            NumberIndividual::new(5.0),
            NumberIndividual::new(2.0),
        ]);
        let island2 = Island::new(vec![
            NumberIndividual::new(1.0),
            NumberIndividual::new(3.0),
        ]);
        let island3 = Island::new(vec![
            NumberIndividual::new(4.0),
            NumberIndividual::new(6.0),
        ]);

        let world = World::new(vec![island1, island2, island3]);
        let best = world.global_best();

        assert!(best.is_some());
        assert_eq!(best.unwrap().fitness(), 1.0);
    }

    #[test]
    fn test_world_migrate() {
        let island1 = Island::new(vec![
            NumberIndividual::new(5.0),
            NumberIndividual::new(10.0),
        ]);
        let island2 = Island::new(vec![
            NumberIndividual::new(1.0),
            NumberIndividual::new(20.0),
        ]);

        let mut world = World::new(vec![island1, island2]);
        world.migrate(1);

        // After migration, islands should have exchanged best individuals
        // Island 1 should now have individual from island 2 (value 1.0)
        assert!(world.islands[0]
            .population
            .iter()
            .any(|ind| (ind.value - 1.0).abs() < 0.001));

        // Island 2 should now have individual from island 1 (value 5.0)
        assert!(world.islands[1]
            .population
            .iter()
            .any(|ind| (ind.value - 5.0).abs() < 0.001));
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
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 2,
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

        // After 10 generations, fitness should improve significantly
        assert!(best.fitness() < 50.0);
    }

    #[test]
    fn test_genetic_algorithm_with_timeout() {
        let initial_population = vec![
            NumberIndividual::new(100.0),
            NumberIndividual::new(50.0),
        ];

        let config = Config {
            population: 2,
            generations: 1000000, // Very large
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 2,
            elitism_ratio: 0.25,
            timeout: Some(Duration::from_millis(1)), // Very short timeout
            islands: 1,
            migration_interval: 10,
            migrants: 1,
        };

        let evolutor = SimpleEvolution::new(1.0);
        let mut ga = GeneticAlgorithm::new(config, evolutor);

        let start = std::time::Instant::now();
        let result = ga.solve(initial_population);
        let elapsed = start.elapsed();

        // Should return early due to timeout
        assert!(elapsed < Duration::from_secs(1));
        assert!(result.is_some());
    }

    #[test]
    fn test_config_with_single_island() {
        let initial_population = vec![
            NumberIndividual::new(10.0),
            NumberIndividual::new(20.0),
        ];

        let config = Config {
            population: 2,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 2,
            elitism_ratio: 0.5,
            timeout: None,
            islands: 1,
            migration_interval: 10,
            migrants: 1,
        };

        let evolutor = SimpleEvolution::new(0.5);
        let mut ga = GeneticAlgorithm::new(config, evolutor);

        let result = ga.solve(initial_population);
        assert!(result.is_some());
    }

    #[test]
    fn test_empty_population() {
        let initial_population: Vec<NumberIndividual> = vec![];

        let config = Config {
            population: 10,
            generations: 5,
            mutation_rate: 0.3,
            crossover_rate: 0.7,
            tournament_size: 2,
            elitism_ratio: 0.1,
            timeout: None,
            islands: 2,
            migration_interval: 10,
            migrants: 1,
        };

        let evolutor = SimpleEvolution::new(0.5);
        let mut ga = GeneticAlgorithm::new(config, evolutor);

        let result = ga.solve(initial_population);
        assert!(result.is_none());
    }

    #[test]
    fn test_elitism_preserves_best() {
        let population = vec![
            NumberIndividual::new(1.0), // Best
            NumberIndividual::new(5.0),
            NumberIndividual::new(10.0),
            NumberIndividual::new(20.0),
        ];

        let mut island = Island::new(population);
        let evolutor = SimpleEvolution::new(100.0); // Large mutation
        let config = Config {
            population: 4,
            generations: 1,
            mutation_rate: 1.0,
            crossover_rate: 1.0,
            tournament_size: 2,
            elitism_ratio: 0.25, // Keep 1 elite
            timeout: None,
            islands: 1,
            migration_interval: 10,
            migrants: 1,
        };

        let best_before = island.best().unwrap().value;
        island.evolve(&evolutor, &config);
        let best_after = island.best().unwrap().value;

        // Best individual should be preserved due to elitism
        assert_eq!(best_before, best_after);
    }
}
