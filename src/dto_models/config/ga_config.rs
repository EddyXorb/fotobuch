use serde::{Deserialize, Serialize};

use super::fitness_weights::FitnessWeights;

/// Genetic algorithm configuration (persisted in YAML, mirrors internal GaConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaConfig {
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_population_size")]
    pub population_size: usize,
    #[serde(default = "default_max_generations")]
    pub max_generations: usize,
    #[serde(default = "default_mutation_rate")]
    pub mutation_rate: f64,
    #[serde(default = "default_crossover_rate")]
    pub crossover_rate: f64,
    #[serde(default = "default_elite_count")]
    pub elite_count: usize,
    #[serde(default = "default_no_improvement_limit")]
    pub no_improvement_limit: Option<usize>,
    #[serde(default)]
    pub weights: FitnessWeights,

    /// Number of islands (independent populations).
    #[serde(default = "default_islands_nr")]
    pub islands_nr: usize,

    /// Generations between migrations.
    #[serde(default = "default_islands_migration_interval")]
    pub islands_migration_interval: usize,

    /// Number of individuals to migrate per island per migration event.
    #[serde(default = "default_islands_nr_migrants")]
    pub islands_nr_migrants: usize,

    /// Enable deterministic in-page photo ordering via DFS-preorder assignment.
    #[serde(default = "default_enforce_order")]
    pub enforce_order: bool,
}

impl Default for GaConfig {
    fn default() -> Self {
        Self {
            islands_nr: default_islands_nr(),
            islands_migration_interval: default_islands_migration_interval(),
            islands_nr_migrants: default_islands_nr_migrants(),
            seed: default_seed(),
            population_size: default_population_size(),
            max_generations: default_max_generations(),
            mutation_rate: default_mutation_rate(),
            crossover_rate: default_crossover_rate(),
            elite_count: default_elite_count(),
            no_improvement_limit: default_no_improvement_limit(),
            weights: FitnessWeights::default(),
            enforce_order: default_enforce_order(),
        }
    }
}

fn default_islands_migration_interval() -> usize {
    5
}

fn default_islands_nr_migrants() -> usize {
    2
}

fn default_islands_nr() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn default_seed() -> u64 {
    42
}

fn default_population_size() -> usize {
    200
}

fn default_max_generations() -> usize {
    1000
}

fn default_mutation_rate() -> f64 {
    0.3
}

fn default_crossover_rate() -> f64 {
    0.7
}

fn default_elite_count() -> usize {
    20
}

fn default_no_improvement_limit() -> Option<usize> {
    Some(15)
}

fn default_enforce_order() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_config_default() {
        let config = GaConfig::default();
        assert_eq!(config.population_size, 200);
        assert_eq!(config.max_generations, 1000);
        assert!(config.islands_nr >= 1);
        assert!(config.islands_nr >= 1);
        assert_eq!(config.islands_migration_interval, 5);
        assert_eq!(config.islands_nr_migrants, 2);
    }
}
