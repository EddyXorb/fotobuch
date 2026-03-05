//! Genetic algorithm configuration.

use super::FitnessWeights;
use std::time::Duration;

/// Configuration for the genetic algorithm.
#[derive(Debug, Clone)]
pub struct GaConfig {
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

    /// Fitness function weights.
    pub weights: FitnessWeights,

    /// Optional timeout for the entire optimization run.
    pub timeout: Option<Duration>,

    /// Island model configuration for parallel evolution.
    pub island_config: IslandConfig,

    /// Random seed for reproducibility.
    pub seed: u64,
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
            weights: FitnessWeights::default(),
            timeout: Some(Duration::from_secs(30)),
            island_config: IslandConfig::default(),
            seed: 0,
        }
    }
}

/// Configuration for the island model (parallel GA with migration).
///
/// Multiple populations evolve independently with periodic migration of best individuals.
/// This helps maintain diversity and can escape local optima.
#[derive(Debug, Clone)]
pub struct IslandConfig {
    /// Number of independent islands (populations).
    /// Defaults to number of available CPU cores.
    pub islands: usize,

    /// Generations between migrations.
    pub migration_interval: usize,

    /// Number of individuals to migrate per island per migration event.
    pub migrants: usize,
}

impl Default for IslandConfig {
    fn default() -> Self {
        let islands = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            islands,
            migration_interval: 5,
            migrants: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population, 300);
        assert_eq!(config.generations, 100);
        assert!(config.island_config.islands >= 1);
    }

    #[test]
    fn test_island_config_default() {
        let config = IslandConfig::default();
        assert!(config.islands >= 1);
        assert_eq!(config.migration_interval, 5);
        assert_eq!(config.migrants, 2);
    }
}
