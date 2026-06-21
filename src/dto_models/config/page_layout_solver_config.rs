use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::fitness_weights::FitnessWeights;

/// Genetic algorithm configuration (persisted in YAML, mirrors internal PageLayoutSolverConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayoutSolverConfig {
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

    /// Tournament size for selection (number of candidates per tournament).
    #[serde(default = "default_tournament_size")]
    pub tournament_size: usize,

    /// Per-page layout timeout in milliseconds; absent means unlimited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

impl Default for PageLayoutSolverConfig {
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
            tournament_size: default_tournament_size(),
            timeout_ms: None,
        }
    }
}

impl PageLayoutSolverConfig {
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_ms.map(Duration::from_millis)
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
    750
}

fn default_max_generations() -> usize {
    100
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
    Some(5)
}

fn default_enforce_order() -> bool {
    true
}

fn default_tournament_size() -> usize {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ga_config_default() {
        let config = PageLayoutSolverConfig::default();
        assert_eq!(config.population_size, 750);
        assert_eq!(config.max_generations, 100);
        assert!(config.islands_nr >= 1);
        assert!(config.islands_nr >= 1);
        assert_eq!(config.islands_migration_interval, 5);
        assert_eq!(config.islands_nr_migrants, 2);
    }
}
