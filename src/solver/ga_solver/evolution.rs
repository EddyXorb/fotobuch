//! Evolution dynamics for genetic algorithms.
//!
//! Provides the core genetic operators and population structures for parallel island-based GA.

use super::individual::Individual;
use super::config::Config;

/// Evolution dynamics bundling selection, crossover, and mutation strategies.
///
/// Implementations define how populations evolve through genetic operations.
/// The trait takes immutable references to enable parallel execution.
pub trait EvolutionDynamic<I: Individual> {
    /// Selects individuals from the population for reproduction.
    ///
    /// Typically implements tournament selection, roulette wheel, or rank-based selection.
    fn select(&self, population: &[I]) -> Vec<I>;

    /// Performs crossover on selected parents to produce offspring.
    ///
    /// Takes parent individuals and generates child individuals through recombination.
    fn crossover(&self, parents: &[I]) -> Vec<I>;

    /// Mutates individuals in-place.
    ///
    /// Applies random modifications to introduce genetic diversity.
    fn mutate(&self, individuals: &mut [I]);
}

/// A single island in the island model GA.
///
/// Each island maintains its own population and evolves independently,
/// with occasional migration between islands.
pub struct Island<I: Individual> {
    population: Vec<I>,
}

impl<I> Island<I>
where
    I: Individual,
{
    /// Creates a new island with the given population.
    ///
    /// The population is sorted by fitness (best first).
    pub fn new(population: Vec<I>) -> Self {
        let mut population = population;
        population.sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
        Self { population }
    }

    /// Evolves the island's population for one generation.
    pub fn evolve<E>(&mut self, evolutor: &E, config: &Config)
    where
        E: EvolutionDynamic<I>,
    {
        let elite_count = calc_elite_count(self.population.len(), config.elitism_ratio);
        let elite = self.population[..elite_count].to_vec();

        let selected = evolutor.select(&self.population);
        let mut offspring = evolutor.crossover(&selected);
        evolutor.mutate(&mut offspring);

        self.population = build_next_population(elite, offspring, config.population);
        self.population
            .sort_by(|a, b| a.fitness().total_cmp(&b.fitness()));
    }

    /// Returns the best individual in the island.
    pub fn best(&self) -> Option<&I> {
        self.population.first()
    }
}

/// Calculates the number of elite individuals to keep.
fn calc_elite_count(population_size: usize, elitism_ratio: f64) -> usize {
    (population_size as f64 * elitism_ratio) as usize
}

/// Builds the next generation population from elite and offspring.
fn build_next_population<I: Individual>(
    elite: Vec<I>,
    offspring: Vec<I>,
    target_size: usize,
) -> Vec<I> {
    let mut next_population = elite;
    next_population.extend(offspring);
    next_population.truncate(target_size);
    next_population
}

/// A world of islands for parallel evolution.
///
/// Manages multiple independent populations (islands) that evolve in parallel
/// and exchange individuals periodically through migration.
pub struct World<I: Individual> {
    pub islands: Vec<Island<I>>,
}

impl<I: Individual> World<I> {
    /// Creates a new world with the given islands.
    pub fn new(islands: Vec<Island<I>>) -> Self {
        Self { islands }
    }

    /// Returns the globally best individual across all islands.
    pub fn global_best(&self) -> Option<I> {
        self.islands
            .iter()
            .filter_map(|island| island.best())
            .min_by(|a, b| a.fitness().total_cmp(&b.fitness()))
            .cloned()
    }

    /// Migrates best individuals between islands.
    pub fn migrate(&mut self, migrants_per_island: usize) {
        if self.islands.len() < 2 {
            return;
        }

        let migrants = collect_migrants(&self.islands);
        distribute_migrants(&mut self.islands, &migrants, migrants_per_island);
    }
}

/// Collects the best individuals from each island.
fn collect_migrants<I: Individual>(islands: &[Island<I>]) -> Vec<I> {
    islands
        .iter()
        .filter_map(|island| island.best().cloned())
        .collect()
}

/// Distributes migrants to other islands, replacing worst individuals.
fn distribute_migrants<I: Individual>(
    islands: &mut [Island<I>],
    migrants: &[I],
    migrants_per_island: usize,
) {
    for (i, island) in islands.iter_mut().enumerate() {
        for (j, migrant) in migrants.iter().enumerate() {
            if i != j && island.population.len() > migrants_per_island {
                let replace_idx = island.population.len() - 1;
                island.population[replace_idx] = migrant.clone();
            }
        }
    }
}
