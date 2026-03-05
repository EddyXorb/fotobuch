//! Configuration for the genetic algorithm.

use std::time::Duration;

/// Configuration parameters for the genetic algorithm.
#[derive(Clone)]
pub struct Config {
    /// Population size per island.
    pub population: usize,

    /// Maximum number of generations.
    pub generations: usize,

    /// Elitism ratio - proportion of best individuals to keep unchanged (0.0 to 1.0).
    pub elitism_ratio: f64,

    /// Optional timeout for the entire optimization run.
    pub timeout: Option<Duration>,

    /// Stop early if fitness hasn't improved for this many generations.
    /// None means no early stopping.
    pub no_improvement_limit: Option<usize>,

    /// Number of independent islands (populations).
    /// For parallel evolution across multiple populations.
    pub islands: usize,

    /// Generations between migrations.
    /// Controls how often individuals migrate between islands.
    pub migration_interval: usize,

    /// Number of individuals to migrate per island per migration event.
    pub migrants: usize,
}
